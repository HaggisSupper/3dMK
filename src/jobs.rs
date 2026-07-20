use crate::projects::{
    current_timestamp, JobKind, JobRecord, JobState, ProjectError, ProjectStore,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("job was not found")]
    NotFound,
    #[error("job queue is full")]
    QueueFull,
    #[error("invalid job transition")]
    InvalidTransition,
    #[error("invalid job update: {0}")]
    Invalid(String),
    #[error("job registry lock is unavailable")]
    Lock,
    #[error(transparent)]
    Project(#[from] ProjectError),
}

pub type Result<T> = std::result::Result<T, JobError>;

struct RuntimeJob {
    record: JobRecord,
    started: Instant,
}

#[derive(Clone)]
pub struct JobRegistry {
    store: ProjectStore,
    max_active: usize,
    active: Arc<Mutex<HashMap<Uuid, RuntimeJob>>>,
}

impl JobRegistry {
    pub fn open(store: ProjectStore, max_active: usize) -> std::result::Result<Self, ProjectError> {
        if max_active == 0 {
            return Err(ProjectError::Invalid(
                "maximum active jobs must be greater than zero".to_owned(),
            ));
        }
        store.recover_abandoned_jobs()?;
        Ok(Self {
            store,
            max_active,
            active: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn create(&self, project_id: Uuid, kind: JobKind, cancel_mode: &str) -> Result<JobRecord> {
        self.store.load_project(project_id)?;
        let cancel_mode = validated_text(cancel_mode, "cancel mode", 100)?;
        let mut active = self.active.lock().map_err(|_| JobError::Lock)?;
        if active.len() >= self.max_active {
            return Err(JobError::QueueFull);
        }
        let timestamp = current_timestamp()?;
        let record = JobRecord {
            job_id: Uuid::new_v4(),
            project_id,
            kind,
            state: JobState::Queued,
            stage: "queued".to_owned(),
            progress: Some(0.0),
            elapsed_ms: 0,
            cancel_mode,
            diagnostics: vec![],
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        self.store.save_job(&record)?;
        active.insert(
            record.job_id,
            RuntimeJob {
                record: record.clone(),
                started: Instant::now(),
            },
        );
        Ok(record)
    }

    pub fn get(&self, job_id: Uuid) -> Result<JobRecord> {
        if let Some(record) = self
            .active
            .lock()
            .map_err(|_| JobError::Lock)?
            .get(&job_id)
            .map(|job| job.record.clone())
        {
            return Ok(record);
        }
        self.store.find_job(job_id).map_err(|error| match error {
            ProjectError::NotFound => JobError::NotFound,
            other => other.into(),
        })
    }

    pub fn transition(
        &self,
        job_id: Uuid,
        next: JobState,
        stage: &str,
        progress: Option<f64>,
        diagnostic: Option<&str>,
    ) -> Result<JobRecord> {
        validate_progress(progress)?;
        let stage = validated_text(stage, "job stage", 100)?;
        let diagnostic = diagnostic
            .map(|value| validated_text(value, "job diagnostic", 1000))
            .transpose()?;
        let mut active = self.active.lock().map_err(|_| JobError::Lock)?;
        let runtime = active.get(&job_id).ok_or(JobError::NotFound)?;
        if !valid_transition(runtime.record.state, next) {
            return Err(JobError::InvalidTransition);
        }

        let mut updated = runtime.record.clone();
        updated.state = next;
        updated.stage = stage;
        updated.progress = progress;
        updated.elapsed_ms = runtime.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        updated.updated_at = current_timestamp()?;
        if let Some(diagnostic) = diagnostic {
            updated.diagnostics.push(diagnostic);
        }
        self.store.save_job(&updated)?;
        if is_terminal(next) {
            active.remove(&job_id);
        } else if let Some(runtime) = active.get_mut(&job_id) {
            runtime.record = updated.clone();
        }
        Ok(updated)
    }

    pub fn request_cancel(&self, job_id: Uuid) -> Result<JobRecord> {
        let state = self.get(job_id)?.state;
        match state {
            JobState::Queued => self.transition(
                job_id,
                JobState::Cancelled,
                "cancelled",
                None,
                Some("Job was cancelled before it started."),
            ),
            JobState::Running => self.transition(
                job_id,
                JobState::CancellationRequested,
                "cancellation_requested",
                None,
                Some("Cancellation was requested."),
            ),
            JobState::CancellationRequested => self.get(job_id),
            _ => Err(JobError::InvalidTransition),
        }
    }
}

fn valid_transition(current: JobState, next: JobState) -> bool {
    matches!(
        (current, next),
        (
            JobState::Queued,
            JobState::Running | JobState::Cancelled | JobState::Failed
        ) | (
            JobState::Running,
            JobState::CancellationRequested
                | JobState::Cancelled
                | JobState::Failed
                | JobState::ReviewRequired
                | JobState::Complete
        ) | (
            JobState::CancellationRequested,
            JobState::Cancelled | JobState::Failed | JobState::Complete
        ) | (
            JobState::ReviewRequired,
            JobState::Cancelled | JobState::Failed | JobState::Complete
        )
    )
}

fn is_terminal(state: JobState) -> bool {
    matches!(
        state,
        JobState::Cancelled | JobState::Failed | JobState::Complete
    )
}

fn validate_progress(progress: Option<f64>) -> Result<()> {
    if progress.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
        return Err(JobError::Invalid(
            "progress must be between zero and one".to_owned(),
        ));
    }
    Ok(())
}

fn validated_text(value: &str, field: &str, max_chars: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars || value.chars().any(char::is_control)
    {
        return Err(JobError::Invalid(format!(
            "{field} is missing, too long, or contains control characters"
        )));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("3dmk-job-test-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn queue_limit_releases_after_terminal_state() {
        let directory = TestDir::new();
        let store = ProjectStore::open(&directory.0).unwrap();
        let project = store.create_project("Jobs").unwrap();
        let jobs = JobRegistry::open(store, 1).unwrap();
        let first = jobs
            .create(project.project_id, JobKind::Prepare, "cooperative")
            .unwrap();
        assert!(matches!(
            jobs.create(project.project_id, JobKind::Export, "cooperative"),
            Err(JobError::QueueFull)
        ));
        jobs.transition(
            first.job_id,
            JobState::Running,
            "preparing",
            Some(0.5),
            None,
        )
        .unwrap();
        jobs.transition(
            first.job_id,
            JobState::Complete,
            "complete",
            Some(1.0),
            None,
        )
        .unwrap();
        jobs.create(project.project_id, JobKind::Export, "cooperative")
            .unwrap();
    }

    #[test]
    fn job_state_is_persisted_and_recovered_after_restart() {
        let directory = TestDir::new();
        let store = ProjectStore::open(&directory.0).unwrap();
        let project = store.create_project("Recovery").unwrap();
        let jobs = JobRegistry::open(store.clone(), 2).unwrap();
        let job = jobs
            .create(project.project_id, JobKind::Reconstruct, "process_kill")
            .unwrap();
        jobs.transition(job.job_id, JobState::Running, "solving_field", None, None)
            .unwrap();
        let temp_dir = directory
            .0
            .join("projects")
            .join(project.project_id.to_string())
            .join("temp")
            .join(job.job_id.to_string());
        fs::create_dir(&temp_dir).unwrap();
        fs::write(temp_dir.join("partial-output"), b"unfinished").unwrap();
        drop(jobs);

        let reopened = JobRegistry::open(store, 2).unwrap();
        let recovered = reopened.get(job.job_id).unwrap();
        assert_eq!(recovered.state, JobState::Failed);
        assert_eq!(recovered.stage, "interrupted");
        assert!(recovered
            .diagnostics
            .iter()
            .any(|message| message.contains("application restart")));
        assert!(!temp_dir.exists());
    }

    #[test]
    fn cancellation_and_invalid_transitions_are_explicit() {
        let directory = TestDir::new();
        let store = ProjectStore::open(&directory.0).unwrap();
        let project = store.create_project("Cancel").unwrap();
        let jobs = JobRegistry::open(store, 2).unwrap();
        let queued = jobs
            .create(project.project_id, JobKind::Import, "cooperative")
            .unwrap();
        assert_eq!(
            jobs.request_cancel(queued.job_id).unwrap().state,
            JobState::Cancelled
        );

        let running = jobs
            .create(project.project_id, JobKind::Prepare, "cooperative")
            .unwrap();
        jobs.transition(running.job_id, JobState::Running, "preparing", None, None)
            .unwrap();
        assert_eq!(
            jobs.request_cancel(running.job_id).unwrap().state,
            JobState::CancellationRequested
        );
        assert!(matches!(
            jobs.transition(
                running.job_id,
                JobState::Running,
                "running_again",
                Some(f64::NAN),
                None
            ),
            Err(JobError::Invalid(_))
        ));
    }
}
