use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fmt::Write as _,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;
use zip::{write::SimpleFileOptions, ZipWriter};

pub const PROJECT_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_MAX_ASSET_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("invalid project data: {0}")]
    Invalid(String),
    #[error("project item was not found")]
    NotFound,
    #[error("project state conflict: {0}")]
    Conflict(String),
    #[error("asset exceeds the configured byte limit")]
    AssetTooLarge,
    #[error("project store lock is unavailable")]
    Lock,
    #[error("filesystem operation failed")]
    Io(#[from] io::Error),
    #[error("project metadata is invalid")]
    Json(#[from] serde_json::Error),
    #[error("timestamp generation failed")]
    Time(#[from] time::error::Format),
    #[error("package export failed")]
    Zip(#[from] zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, ProjectError>;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthUnit {
    #[default]
    Metres,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetRole {
    SourcePackage,
    SourceModel,
    Texture,
    Photo,
    Calibration,
    Preview,
    Overlay,
    Report,
    Export,
    DerivedScene,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneKind {
    Mesh,
    PointCloud,
    Hybrid,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyRelation {
    SameTopology,
    Subselection,
    Resampled,
    Reconstructed,
    RigidTransform,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionStatus {
    Candidate,
    Accepted,
    AcceptedWithWarnings,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Import,
    CleanupComponents,
    LevelToPlane,
    SmoothTaubin,
    PreparePointCloud,
    ReconstructScreenedPoisson,
    SampleMeshToPoints,
    ProjectImagesToVertices,
    BakeTexture,
    ApplyScaleCorrection,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Started,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisKind {
    Structure,
    Recognition,
    ReconstructionQuality,
    RoomPlan,
    ImageQuality,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Import,
    Prepare,
    Reconstruct,
    Recognize,
    ProjectImages,
    Export,
    RoomSolve,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    CancellationRequested,
    Cancelled,
    Failed,
    ReviewRequired,
    Complete,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ProjectWarning {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct AttributeContract {
    pub normals: bool,
    pub colors: bool,
    pub uvs: bool,
    pub material_ids: bool,
    pub source_ids: bool,
    pub confidence: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DatumMetadata {
    pub kind: String,
    pub origin: [f64; 3],
    pub normal: [f64; 3],
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SpatialMetadata {
    pub transform: [[f64; 4]; 4],
    pub datum: Option<DatumMetadata>,
    pub origin: [f64; 3],
}

impl Default for SpatialMetadata {
    fn default() -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            datum: None,
            origin: [0.0; 3],
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Project {
    pub schema_version: u32,
    pub generation: u64,
    pub project_id: Uuid,
    pub name: String,
    pub canonical_units: LengthUnit,
    pub original_units: Option<String>,
    pub source_asset_ids: Vec<Uuid>,
    pub root_revision_id: Option<Uuid>,
    pub active_revision_id: Option<Uuid>,
    pub revision_ids: Vec<Uuid>,
    pub analysis_ids: Vec<Uuid>,
    pub measurement_set_ids: Vec<Uuid>,
    pub created_at: String,
    pub updated_at: String,
    pub warnings: Vec<ProjectWarning>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Asset {
    pub asset_id: Uuid,
    pub sha256: String,
    pub original_name: String,
    pub media_type: String,
    pub byte_length: u64,
    pub role: AssetRole,
    pub created_by: Option<Uuid>,
    pub created_at: String,
    pub metadata: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Revision {
    pub revision_id: Uuid,
    pub parent_revision_ids: Vec<Uuid>,
    pub operation_id: Option<Uuid>,
    pub scene_kind: SceneKind,
    pub scene_asset_id: Uuid,
    pub render_asset_id: Option<Uuid>,
    pub attribute_contract: AttributeContract,
    pub topology_relation: TopologyRelation,
    pub source_mapping_asset_id: Option<Uuid>,
    pub spatial: SpatialMetadata,
    pub camera_set_id: Option<Uuid>,
    pub quality_report_id: Option<Uuid>,
    pub status: RevisionStatus,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperationRecord {
    pub operation_id: Uuid,
    pub kind: OperationKind,
    pub state: OperationState,
    pub project_id: Uuid,
    pub input_revision_ids: Vec<Uuid>,
    pub parameters: Value,
    pub engine_versions: Value,
    pub deterministic_seed: Option<u64>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub output_revision_ids: Vec<Uuid>,
    pub output_analysis_ids: Vec<Uuid>,
    pub warnings: Vec<ProjectWarning>,
    pub error_code: Option<String>,
    pub metrics: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnalysisRecord {
    pub analysis_id: Uuid,
    pub kind: AnalysisKind,
    pub input_revision_id: Uuid,
    pub algorithm: String,
    pub algorithm_version: String,
    pub parameters: Value,
    pub finding_ids: Vec<Uuid>,
    pub overlay_asset_ids: Vec<Uuid>,
    pub source_mapping_asset_id: Option<Uuid>,
    pub warnings: Vec<ProjectWarning>,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JobRecord {
    pub job_id: Uuid,
    pub project_id: Uuid,
    pub kind: JobKind,
    pub state: JobState,
    pub stage: String,
    pub progress: Option<f64>,
    pub elapsed_ms: u64,
    pub cancel_mode: String,
    pub diagnostics: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct RootSceneImport {
    pub project_name: String,
    pub original_name: String,
    pub media_type: String,
    pub scene_kind: SceneKind,
    pub attributes: AttributeContract,
    pub original_units: Option<String>,
    pub warnings: Vec<ProjectWarning>,
}

#[derive(Clone, Debug)]
pub struct PackageAssetImport {
    pub source_path: PathBuf,
    pub package_paths: Vec<String>,
    pub original_name: String,
    pub media_type: String,
    pub role: AssetRole,
    pub source: bool,
    pub expected_sha256: Option<String>,
    pub expected_bytes: Option<u64>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct PackageSceneImport {
    pub project_name: String,
    pub original_units: Option<String>,
    pub warnings: Vec<ProjectWarning>,
    pub primary_package_path: String,
    pub scene_kind: SceneKind,
    pub attributes: AttributeContract,
    pub assets: Vec<PackageAssetImport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PackageSceneImportResult {
    pub project: Project,
    pub assets: Vec<Asset>,
    pub root_revision: Revision,
}

#[derive(Clone)]
pub struct ProjectStore {
    root: PathBuf,
    max_asset_bytes: u64,
    lock: Arc<Mutex<()>>,
}

impl ProjectStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        Self::open_with_asset_limit(root, DEFAULT_MAX_ASSET_BYTES)
    }

    pub fn open_with_asset_limit(root: impl Into<PathBuf>, max_asset_bytes: u64) -> Result<Self> {
        if max_asset_bytes == 0 {
            return Err(ProjectError::Invalid(
                "asset byte limit must be greater than zero".to_owned(),
            ));
        }
        let store = Self {
            root: root.into(),
            max_asset_bytes,
            lock: Arc::new(Mutex::new(())),
        };
        fs::create_dir_all(store.projects_dir())?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create_project(&self, name: &str) -> Result<Project> {
        let _guard = self.guard()?;
        let name = validated_project_name(name)?;
        let project_id = Uuid::new_v4();
        let timestamp = now()?;
        let project = Project {
            schema_version: PROJECT_SCHEMA_VERSION,
            generation: 0,
            project_id,
            name,
            canonical_units: LengthUnit::Metres,
            original_units: None,
            source_asset_ids: vec![],
            root_revision_id: None,
            active_revision_id: None,
            revision_ids: vec![],
            analysis_ids: vec![],
            measurement_set_ids: vec![],
            created_at: timestamp.clone(),
            updated_at: timestamp,
            warnings: vec![],
        };

        let final_dir = self.project_dir(project_id);
        let temp_dir = self
            .projects_dir()
            .join(format!(".{}.{}.tmp", project_id, Uuid::new_v4()));
        let result = (|| {
            create_project_layout(&temp_dir)?;
            write_new_json(&temp_dir.join("project.json"), &project)?;
            fs::rename(&temp_dir, &final_dir)?;
            Ok(project)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&temp_dir);
        }
        result
    }

    pub fn import_root_scene(
        &self,
        request: RootSceneImport,
        mut reader: impl Read,
    ) -> Result<(Project, Asset, Revision)> {
        let _guard = self.guard()?;
        let project_name = validated_project_name(&request.project_name)?;
        let media_type = request.media_type.trim();
        if media_type.is_empty() || media_type.len() > 200 {
            return Err(ProjectError::Invalid(
                "asset media type is missing or too long".to_owned(),
            ));
        }

        let project_id = Uuid::new_v4();
        let timestamp = now()?;
        let final_dir = self.project_dir(project_id);
        let temp_dir = self
            .projects_dir()
            .join(format!(".{}.{}.tmp", project_id, Uuid::new_v4()));
        let result = (|| {
            create_project_layout(&temp_dir)?;

            let upload_path = temp_dir.join("temp").join("source.asset.tmp");
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&upload_path)?;
            let mut hash = Sha256::new();
            let mut byte_length = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = reader.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                byte_length = byte_length
                    .checked_add(read as u64)
                    .ok_or(ProjectError::AssetTooLarge)?;
                if byte_length > self.max_asset_bytes {
                    return Err(ProjectError::AssetTooLarge);
                }
                hash.update(&buffer[..read]);
                output.write_all(&buffer[..read])?;
            }
            output.sync_all()?;
            drop(output);

            let digest = hex_digest(hash.finalize().as_slice());
            let asset = Asset {
                asset_id: Uuid::new_v4(),
                sha256: digest.clone(),
                original_name: safe_display_name(&request.original_name),
                media_type: media_type.to_owned(),
                byte_length,
                role: AssetRole::SourceModel,
                created_by: None,
                created_at: timestamp.clone(),
                metadata: Value::Object(Default::default()),
            };
            let asset_dir = temp_dir.join("assets").join(&digest);
            fs::create_dir(&asset_dir)?;
            fs::rename(upload_path, asset_dir.join("payload"))?;
            write_new_json(&asset_dir.join("asset.json"), &asset)?;

            let revision = Revision {
                revision_id: Uuid::new_v4(),
                parent_revision_ids: vec![],
                operation_id: None,
                scene_kind: request.scene_kind,
                scene_asset_id: asset.asset_id,
                render_asset_id: Some(asset.asset_id),
                attribute_contract: request.attributes,
                topology_relation: TopologyRelation::SameTopology,
                source_mapping_asset_id: None,
                spatial: SpatialMetadata::default(),
                camera_set_id: None,
                quality_report_id: None,
                status: RevisionStatus::Accepted,
                created_at: timestamp.clone(),
            };
            let revision_dir = temp_dir
                .join("revisions")
                .join(revision.revision_id.to_string());
            fs::create_dir(&revision_dir)?;
            write_new_json(&revision_dir.join("revision.json"), &revision)?;

            let project = Project {
                schema_version: PROJECT_SCHEMA_VERSION,
                generation: 0,
                project_id,
                name: project_name,
                canonical_units: LengthUnit::Metres,
                original_units: request.original_units,
                source_asset_ids: vec![asset.asset_id],
                root_revision_id: Some(revision.revision_id),
                active_revision_id: Some(revision.revision_id),
                revision_ids: vec![revision.revision_id],
                analysis_ids: vec![],
                measurement_set_ids: vec![],
                created_at: timestamp.clone(),
                updated_at: timestamp,
                warnings: request.warnings,
            };
            write_new_json(&temp_dir.join("project.json"), &project)?;
            fs::rename(&temp_dir, &final_dir)?;
            Ok((project, asset, revision))
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&temp_dir);
        }
        result
    }

    pub fn import_package_scene(
        &self,
        request: PackageSceneImport,
    ) -> Result<PackageSceneImportResult> {
        let _guard = self.guard()?;
        let project_name = validated_project_name(&request.project_name)?;
        if request.assets.is_empty() || !valid_package_path(&request.primary_package_path) {
            return Err(ProjectError::Invalid(
                "package assets or primary model are missing".to_owned(),
            ));
        }
        let mut seen_paths = HashMap::new();
        for (asset_index, input) in request.assets.iter().enumerate() {
            if input.package_paths.is_empty()
                || input
                    .package_paths
                    .iter()
                    .any(|path| !valid_package_path(path))
            {
                return Err(ProjectError::Invalid(
                    "package asset path is invalid".to_owned(),
                ));
            }
            for path in &input.package_paths {
                if seen_paths
                    .insert(path.to_ascii_lowercase(), asset_index)
                    .is_some()
                {
                    return Err(ProjectError::Conflict(
                        "package contains duplicate asset paths".to_owned(),
                    ));
                }
            }
        }

        let project_id = Uuid::new_v4();
        let timestamp = now()?;
        let final_dir = self.project_dir(project_id);
        let temp_dir = self
            .projects_dir()
            .join(format!(".{}.{}.tmp", project_id, Uuid::new_v4()));
        let result = (|| {
            create_project_layout(&temp_dir)?;
            let mut assets = vec![];
            let mut source_asset_ids = vec![];
            let mut asset_by_package_path = HashMap::new();

            for input in request.assets {
                let media_type = input.media_type.trim();
                if media_type.is_empty() || media_type.len() > 200 {
                    return Err(ProjectError::Invalid(
                        "asset media type is missing or too long".to_owned(),
                    ));
                }
                let mut source = File::open(&input.source_path)?;
                let upload_path = temp_dir
                    .join("temp")
                    .join(format!("{}.asset.tmp", Uuid::new_v4()));
                let mut output = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&upload_path)?;
                let mut hash = Sha256::new();
                let mut byte_length = 0_u64;
                let mut buffer = [0_u8; 64 * 1024];
                loop {
                    let read = source.read(&mut buffer)?;
                    if read == 0 {
                        break;
                    }
                    byte_length = byte_length
                        .checked_add(read as u64)
                        .ok_or(ProjectError::AssetTooLarge)?;
                    if byte_length > self.max_asset_bytes {
                        return Err(ProjectError::AssetTooLarge);
                    }
                    hash.update(&buffer[..read]);
                    output.write_all(&buffer[..read])?;
                }
                output.sync_all()?;
                drop(output);
                if input
                    .expected_bytes
                    .is_some_and(|expected| expected != byte_length)
                {
                    return Err(ProjectError::Invalid(
                        "package asset byte length changed after validation".to_owned(),
                    ));
                }
                let digest_bytes = hash.finalize();
                let digest = hex_digest(&digest_bytes);
                if input
                    .expected_sha256
                    .as_deref()
                    .is_some_and(|expected| !expected.eq_ignore_ascii_case(&digest))
                {
                    return Err(ProjectError::Invalid(
                        "package asset digest changed after validation".to_owned(),
                    ));
                }

                let asset_dir = temp_dir.join("assets").join(&digest);
                let asset = if asset_dir.exists() {
                    fs::remove_file(&upload_path)?;
                    let mut existing = read_json::<Asset>(&asset_dir.join("asset.json"))?;
                    merge_package_paths(&mut existing.metadata, &input.package_paths)?;
                    write_json_recoverable(&asset_dir.join("asset.json"), &existing)?;
                    existing
                } else {
                    let mut metadata = input.metadata;
                    merge_package_paths(&mut metadata, &input.package_paths)?;
                    let asset = Asset {
                        asset_id: Uuid::new_v4(),
                        sha256: digest.clone(),
                        original_name: safe_display_name(&input.original_name),
                        media_type: media_type.to_owned(),
                        byte_length,
                        role: input.role,
                        created_by: None,
                        created_at: timestamp.clone(),
                        metadata,
                    };
                    fs::create_dir(&asset_dir)?;
                    fs::rename(upload_path, asset_dir.join("payload"))?;
                    write_new_json(&asset_dir.join("asset.json"), &asset)?;
                    assets.push(asset.clone());
                    asset
                };
                for package_path in input.package_paths {
                    asset_by_package_path.insert(package_path.to_ascii_lowercase(), asset.asset_id);
                }
                if input.source && !source_asset_ids.contains(&asset.asset_id) {
                    source_asset_ids.push(asset.asset_id);
                }
            }

            let scene_asset_id = asset_by_package_path
                .get(&request.primary_package_path.to_ascii_lowercase())
                .copied()
                .ok_or_else(|| {
                    ProjectError::Invalid(
                        "primary model is not present in package assets".to_owned(),
                    )
                })?;
            let revision = Revision {
                revision_id: Uuid::new_v4(),
                parent_revision_ids: vec![],
                operation_id: None,
                scene_kind: request.scene_kind,
                scene_asset_id,
                render_asset_id: Some(scene_asset_id),
                attribute_contract: request.attributes,
                topology_relation: TopologyRelation::SameTopology,
                source_mapping_asset_id: None,
                spatial: SpatialMetadata::default(),
                camera_set_id: None,
                quality_report_id: None,
                status: RevisionStatus::Accepted,
                created_at: timestamp.clone(),
            };
            let revision_dir = temp_dir
                .join("revisions")
                .join(revision.revision_id.to_string());
            fs::create_dir(&revision_dir)?;
            write_new_json(&revision_dir.join("revision.json"), &revision)?;
            let project = Project {
                schema_version: PROJECT_SCHEMA_VERSION,
                generation: 0,
                project_id,
                name: project_name,
                canonical_units: LengthUnit::Metres,
                original_units: request.original_units,
                source_asset_ids,
                root_revision_id: Some(revision.revision_id),
                active_revision_id: Some(revision.revision_id),
                revision_ids: vec![revision.revision_id],
                analysis_ids: vec![],
                measurement_set_ids: vec![],
                created_at: timestamp.clone(),
                updated_at: timestamp,
                warnings: request.warnings,
            };
            write_new_json(&temp_dir.join("project.json"), &project)?;
            fs::rename(&temp_dir, &final_dir)?;
            Ok(PackageSceneImportResult {
                project,
                assets,
                root_revision: revision,
            })
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&temp_dir);
        }
        result
    }

    pub fn load_project(&self, project_id: Uuid) -> Result<Project> {
        let _guard = self.guard()?;
        self.load_project_locked(project_id)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let _guard = self.guard()?;
        let mut projects = vec![];
        for entry in fs::read_dir(self.projects_dir())? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Ok(project_id) = Uuid::parse_str(&name) else {
                continue;
            };
            projects.push(self.load_project_locked(project_id)?);
        }
        projects.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(projects)
    }

    pub fn list_assets(&self, project_id: Uuid) -> Result<Vec<Asset>> {
        let _guard = self.guard()?;
        self.load_project_locked(project_id)?;
        self.list_assets_locked(project_id)
    }

    fn list_assets_locked(&self, project_id: Uuid) -> Result<Vec<Asset>> {
        let mut assets = vec![];
        for entry in fs::read_dir(self.project_dir(project_id).join("assets"))? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let metadata = entry.path().join("asset.json");
            if metadata.is_file() {
                assets.push(read_json::<Asset>(&metadata)?);
            }
        }
        assets.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(assets)
    }

    pub fn export_package(&self, project_id: Uuid, output_path: &Path) -> Result<()> {
        let _guard = self.guard()?;
        let project = self.load_project_locked(project_id)?;
        let revisions = project
            .revision_ids
            .iter()
            .map(|revision_id| self.load_revision_locked(project_id, *revision_id))
            .collect::<Result<Vec<_>>>()?;
        let assets = self.list_assets_locked(project_id)?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output_path)?;
        let mut archive = ZipWriter::new(file);
        archive.start_file(
            "3dmk-package.json",
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )?;
        let manifest = serde_json::json!({
            "package_format_version": "3dmk-package-v1",
            "project": project.clone(),
            "revisions": revisions.clone(),
            "assets": assets.clone(),
        });
        let mut manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
        manifest_bytes.push(b'\n');
        archive.write_all(&manifest_bytes)?;

        let mut written_paths = HashMap::<String, ()>::new();
        for asset in &assets {
            let payload_path = self
                .project_dir(project_id)
                .join("assets")
                .join(&asset.sha256)
                .join("payload");
            let package_paths = asset
                .metadata
                .get("package_paths")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|path| valid_package_path(path))
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let package_paths = if package_paths.is_empty() {
                vec![format!("__3dmk/assets/{}.bin", asset.sha256)]
            } else {
                package_paths
            };
            for package_path in package_paths {
                if written_paths
                    .insert(package_path.to_ascii_lowercase(), ())
                    .is_some()
                {
                    continue;
                }
                archive.start_file(
                    package_path,
                    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
                )?;
                let mut payload = File::open(&payload_path)?;
                io::copy(&mut payload, &mut archive)?;
            }
        }
        let file = archive.finish()?;
        file.sync_all()?;
        Ok(())
    }

    pub fn store_source_asset(
        &self,
        project_id: Uuid,
        original_name: &str,
        media_type: &str,
        role: AssetRole,
        reader: impl Read,
    ) -> Result<Asset> {
        self.store_asset(project_id, original_name, media_type, role, true, reader)
    }

    pub fn store_derived_asset(
        &self,
        project_id: Uuid,
        original_name: &str,
        media_type: &str,
        role: AssetRole,
        reader: impl Read,
    ) -> Result<Asset> {
        self.store_asset(project_id, original_name, media_type, role, false, reader)
    }

    pub fn create_root_revision(
        &self,
        project_id: Uuid,
        scene_asset_id: Uuid,
        scene_kind: SceneKind,
        attributes: AttributeContract,
    ) -> Result<Revision> {
        let _guard = self.guard()?;
        let project = self.load_project_locked(project_id)?;
        if project.root_revision_id.is_some() {
            return Err(ProjectError::Conflict(
                "root revision already exists".to_owned(),
            ));
        }
        self.find_asset_locked(project_id, scene_asset_id)?;
        let revision = Revision {
            revision_id: Uuid::new_v4(),
            parent_revision_ids: vec![],
            operation_id: None,
            scene_kind,
            scene_asset_id,
            render_asset_id: Some(scene_asset_id),
            attribute_contract: attributes,
            topology_relation: TopologyRelation::SameTopology,
            source_mapping_asset_id: None,
            spatial: SpatialMetadata::default(),
            camera_set_id: None,
            quality_report_id: None,
            status: RevisionStatus::Accepted,
            created_at: now()?,
        };
        self.publish_revision_locked(project, revision, true)
    }

    // ponytail: this is one atomic domain boundary; keep its fields explicit until more call sites justify a request type.
    #[allow(clippy::too_many_arguments)]
    pub fn create_child_revision(
        &self,
        project_id: Uuid,
        parent_revision_id: Uuid,
        scene_asset_id: Uuid,
        scene_kind: SceneKind,
        topology_relation: TopologyRelation,
        attributes: AttributeContract,
        operation_id: Option<Uuid>,
    ) -> Result<Revision> {
        let _guard = self.guard()?;
        let project = self.load_project_locked(project_id)?;
        if !project.revision_ids.contains(&parent_revision_id) {
            return Err(ProjectError::NotFound);
        }
        self.load_revision_locked(project_id, parent_revision_id)?;
        self.find_asset_locked(project_id, scene_asset_id)?;
        let revision = Revision {
            revision_id: Uuid::new_v4(),
            parent_revision_ids: vec![parent_revision_id],
            operation_id,
            scene_kind,
            scene_asset_id,
            render_asset_id: Some(scene_asset_id),
            attribute_contract: attributes,
            topology_relation,
            source_mapping_asset_id: None,
            spatial: SpatialMetadata::default(),
            camera_set_id: None,
            quality_report_id: None,
            status: RevisionStatus::Candidate,
            created_at: now()?,
        };
        self.publish_revision_locked(project, revision, false)
    }

    pub fn load_revision(&self, project_id: Uuid, revision_id: Uuid) -> Result<Revision> {
        let _guard = self.guard()?;
        self.load_revision_locked(project_id, revision_id)
    }

    pub fn list_revisions(&self, project_id: Uuid) -> Result<Vec<Revision>> {
        let _guard = self.guard()?;
        let project = self.load_project_locked(project_id)?;
        project
            .revision_ids
            .iter()
            .map(|revision_id| self.load_revision_locked(project_id, *revision_id))
            .collect()
    }

    pub fn asset_payload(&self, project_id: Uuid, asset_id: Uuid) -> Result<(Asset, PathBuf)> {
        let _guard = self.guard()?;
        self.load_project_locked(project_id)?;
        let asset = self.find_asset_locked(project_id, asset_id)?;
        let path = self
            .project_dir(project_id)
            .join("assets")
            .join(&asset.sha256)
            .join("payload");
        if fs::metadata(&path)?.len() != asset.byte_length {
            return Err(ProjectError::Invalid(
                "asset payload length does not match metadata".to_owned(),
            ));
        }
        Ok((asset, path))
    }

    pub fn set_active_revision(&self, project_id: Uuid, revision_id: Uuid) -> Result<Project> {
        let _guard = self.guard()?;
        let mut project = self.load_project_locked(project_id)?;
        if !project.revision_ids.contains(&revision_id) {
            return Err(ProjectError::NotFound);
        }
        self.load_revision_locked(project_id, revision_id)?;
        project.active_revision_id = Some(revision_id);
        self.save_project_locked(&mut project)?;
        Ok(project)
    }

    pub fn save_job(&self, job: &JobRecord) -> Result<()> {
        let _guard = self.guard()?;
        self.load_project_locked(job.project_id)?;
        write_json_recoverable(
            &self
                .project_dir(job.project_id)
                .join("jobs")
                .join(format!("{}.json", job.job_id)),
            job,
        )
    }

    pub fn load_job(&self, project_id: Uuid, job_id: Uuid) -> Result<JobRecord> {
        let _guard = self.guard()?;
        self.load_job_locked(project_id, job_id)
    }

    pub fn find_job(&self, job_id: Uuid) -> Result<JobRecord> {
        let _guard = self.guard()?;
        // ponytail: O(projects) lookup; add an index only if job lookup becomes measurable.
        for entry in fs::read_dir(self.projects_dir())? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Ok(project_id) = Uuid::parse_str(&name) else {
                continue;
            };
            match self.load_job_locked(project_id, job_id) {
                Ok(job) => return Ok(job),
                Err(ProjectError::NotFound) => {}
                Err(error) => return Err(error),
            }
        }
        Err(ProjectError::NotFound)
    }

    pub fn list_jobs(&self, project_id: Uuid) -> Result<Vec<JobRecord>> {
        let _guard = self.guard()?;
        self.load_project_locked(project_id)?;
        self.list_jobs_locked(project_id)
    }

    pub fn recover_abandoned_jobs(&self) -> Result<Vec<JobRecord>> {
        let _guard = self.guard()?;
        let mut recovered = vec![];
        for entry in fs::read_dir(self.projects_dir())? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Ok(project_id) = Uuid::parse_str(&name) else {
                continue;
            };
            for mut job in self.list_jobs_locked(project_id)? {
                if matches!(
                    job.state,
                    JobState::Queued | JobState::Running | JobState::CancellationRequested
                ) {
                    job.state = JobState::Failed;
                    job.stage = "interrupted".to_owned();
                    job.updated_at = now()?;
                    job.diagnostics
                        .push("Job was interrupted by application restart.".to_owned());
                    write_json_recoverable(
                        &self
                            .project_dir(project_id)
                            .join("jobs")
                            .join(format!("{}.json", job.job_id)),
                        &job,
                    )?;
                    let temp_dir = self
                        .project_dir(project_id)
                        .join("temp")
                        .join(job.job_id.to_string());
                    if temp_dir.exists() {
                        fs::remove_dir_all(temp_dir)?;
                    }
                    recovered.push(job);
                }
            }
        }
        Ok(recovered)
    }

    fn store_asset(
        &self,
        project_id: Uuid,
        original_name: &str,
        media_type: &str,
        role: AssetRole,
        source: bool,
        mut reader: impl Read,
    ) -> Result<Asset> {
        let _guard = self.guard()?;
        let mut project = self.load_project_locked(project_id)?;
        let media_type = media_type.trim();
        if media_type.is_empty() || media_type.len() > 200 {
            return Err(ProjectError::Invalid(
                "asset media type is missing or too long".to_owned(),
            ));
        }

        let temp_path = self
            .project_dir(project_id)
            .join("temp")
            .join(format!("{}.asset.tmp", Uuid::new_v4()));
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;
        let mut hash = Sha256::new();
        let mut byte_length = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        let stream_result = (|| {
            loop {
                let read = reader.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                byte_length = byte_length
                    .checked_add(read as u64)
                    .ok_or(ProjectError::AssetTooLarge)?;
                if byte_length > self.max_asset_bytes {
                    return Err(ProjectError::AssetTooLarge);
                }
                hash.update(&buffer[..read]);
                output.write_all(&buffer[..read])?;
            }
            output.sync_all()?;
            Ok(())
        })();
        drop(output);
        if let Err(error) = stream_result {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }

        let digest = hex_digest(hash.finalize().as_slice());
        let final_dir = self.project_dir(project_id).join("assets").join(&digest);
        if final_dir.exists() {
            let _ = fs::remove_file(&temp_path);
            let asset = read_json::<Asset>(&final_dir.join("asset.json"))?;
            if source && !project.source_asset_ids.contains(&asset.asset_id) {
                project.source_asset_ids.push(asset.asset_id);
                self.save_project_locked(&mut project)?;
            }
            return Ok(asset);
        }

        let asset = Asset {
            asset_id: Uuid::new_v4(),
            sha256: digest.clone(),
            original_name: safe_display_name(original_name),
            media_type: media_type.to_owned(),
            byte_length,
            role,
            created_by: None,
            created_at: now()?,
            metadata: Value::Object(Default::default()),
        };
        let temp_dir = self
            .project_dir(project_id)
            .join("assets")
            .join(format!(".{}.tmp", Uuid::new_v4()));
        let publish_result = (|| {
            fs::create_dir(&temp_dir)?;
            fs::rename(&temp_path, temp_dir.join("payload"))?;
            write_new_json(&temp_dir.join("asset.json"), &asset)?;
            fs::rename(&temp_dir, &final_dir)?;
            Ok(())
        })();
        if let Err(error) = publish_result {
            let _ = fs::remove_file(&temp_path);
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(error);
        }

        if source {
            project.source_asset_ids.push(asset.asset_id);
            self.save_project_locked(&mut project)?;
        }
        Ok(asset)
    }

    fn publish_revision_locked(
        &self,
        mut project: Project,
        revision: Revision,
        root: bool,
    ) -> Result<Revision> {
        let revisions_dir = self.project_dir(project.project_id).join("revisions");
        let final_dir = revisions_dir.join(revision.revision_id.to_string());
        let temp_dir = revisions_dir.join(format!(".{}.tmp", Uuid::new_v4()));
        let publish_result = (|| {
            fs::create_dir(&temp_dir)?;
            write_new_json(&temp_dir.join("revision.json"), &revision)?;
            fs::rename(&temp_dir, &final_dir)?;
            Ok(())
        })();
        if let Err(error) = publish_result {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(error);
        }

        project.revision_ids.push(revision.revision_id);
        project.active_revision_id = Some(revision.revision_id);
        if root {
            project.root_revision_id = Some(revision.revision_id);
        }
        self.save_project_locked(&mut project)?;
        Ok(revision)
    }

    fn load_project_locked(&self, project_id: Uuid) -> Result<Project> {
        let path = self.project_dir(project_id).join("project.json");
        recover_json_file(&path)?;
        let project = read_json::<Project>(&path).map_err(|error| match error {
            ProjectError::Io(source) if source.kind() == io::ErrorKind::NotFound => {
                ProjectError::NotFound
            }
            other => other,
        })?;
        validate_project(project_id, &project)?;
        Ok(project)
    }

    fn save_project_locked(&self, project: &mut Project) -> Result<()> {
        project.generation = project.generation.saturating_add(1);
        project.updated_at = now()?;
        write_json_recoverable(
            &self.project_dir(project.project_id).join("project.json"),
            project,
        )
    }

    fn load_revision_locked(&self, project_id: Uuid, revision_id: Uuid) -> Result<Revision> {
        let path = self
            .project_dir(project_id)
            .join("revisions")
            .join(revision_id.to_string())
            .join("revision.json");
        let revision = read_json::<Revision>(&path).map_err(|error| match error {
            ProjectError::Io(source) if source.kind() == io::ErrorKind::NotFound => {
                ProjectError::NotFound
            }
            other => other,
        })?;
        if revision.revision_id != revision_id {
            return Err(ProjectError::Invalid(
                "revision identifier does not match its directory".to_owned(),
            ));
        }
        Ok(revision)
    }

    fn load_job_locked(&self, project_id: Uuid, job_id: Uuid) -> Result<JobRecord> {
        let path = self
            .project_dir(project_id)
            .join("jobs")
            .join(format!("{}.json", job_id));
        recover_json_file(&path)?;
        let job = read_json::<JobRecord>(&path).map_err(|error| match error {
            ProjectError::Io(source) if source.kind() == io::ErrorKind::NotFound => {
                ProjectError::NotFound
            }
            other => other,
        })?;
        if job.job_id != job_id || job.project_id != project_id {
            return Err(ProjectError::Invalid(
                "job identifier does not match its storage location".to_owned(),
            ));
        }
        Ok(job)
    }

    fn list_jobs_locked(&self, project_id: Uuid) -> Result<Vec<JobRecord>> {
        let jobs_dir = self.project_dir(project_id).join("jobs");
        for entry in fs::read_dir(&jobs_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(target) = name.strip_suffix(".backup") {
                recover_json_file(&jobs_dir.join(target))?;
            }
        }
        let mut jobs = vec![];
        for entry in fs::read_dir(jobs_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file()
                || entry.path().extension().and_then(|value| value.to_str()) != Some("json")
            {
                continue;
            }
            jobs.push(read_json::<JobRecord>(&entry.path())?);
        }
        jobs.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(jobs)
    }

    fn find_asset_locked(&self, project_id: Uuid, asset_id: Uuid) -> Result<Asset> {
        // ponytail: O(n) metadata scan; add an index only if projects reach thousands of assets.
        let assets_dir = self.project_dir(project_id).join("assets");
        for entry in fs::read_dir(assets_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let metadata = entry.path().join("asset.json");
            if !metadata.exists() {
                continue;
            }
            let asset = read_json::<Asset>(&metadata)?;
            if asset.asset_id == asset_id {
                let payload = entry.path().join("payload");
                if fs::metadata(payload)?.len() != asset.byte_length {
                    return Err(ProjectError::Invalid(
                        "asset payload length does not match metadata".to_owned(),
                    ));
                }
                return Ok(asset);
            }
        }
        Err(ProjectError::NotFound)
    }

    fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    fn project_dir(&self, project_id: Uuid) -> PathBuf {
        self.projects_dir().join(project_id.to_string())
    }

    fn guard(&self) -> Result<MutexGuard<'_, ()>> {
        self.lock.lock().map_err(|_| ProjectError::Lock)
    }
}

fn create_project_layout(path: &Path) -> Result<()> {
    fs::create_dir(path)?;
    for child in [
        "assets",
        "revisions",
        "analyses",
        "measurements",
        "room-plans",
        "operations",
        "jobs",
        "temp",
    ] {
        fs::create_dir(path.join(child))?;
    }
    Ok(())
}

fn valid_package_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 512
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn merge_package_paths(metadata: &mut Value, paths: &[String]) -> Result<()> {
    let object = metadata
        .as_object_mut()
        .ok_or_else(|| ProjectError::Invalid("asset metadata must be a JSON object".to_owned()))?;
    let values = object
        .entry("package_paths")
        .or_insert_with(|| Value::Array(vec![]))
        .as_array_mut()
        .ok_or_else(|| ProjectError::Invalid("asset package paths are invalid".to_owned()))?;
    for path in paths {
        if !values.iter().any(|value| {
            value
                .as_str()
                .is_some_and(|existing| existing.eq_ignore_ascii_case(path))
        }) {
            values.push(Value::String(path.clone()));
        }
    }
    Ok(())
}

fn validated_project_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 200 || name.chars().any(char::is_control) {
        return Err(ProjectError::Invalid(
            "project name is missing, too long, or contains control characters".to_owned(),
        ));
    }
    Ok(name.to_owned())
}

fn safe_display_name(name: &str) -> String {
    let basename = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|character| !character.is_control())
        .take(255)
        .collect::<String>();
    let basename = basename.trim();
    if basename.is_empty() {
        "asset".to_owned()
    } else {
        basename.to_owned()
    }
}

fn validate_project(expected_id: Uuid, project: &Project) -> Result<()> {
    if project.schema_version != PROJECT_SCHEMA_VERSION {
        return Err(ProjectError::Invalid(format!(
            "unsupported project schema version {}",
            project.schema_version
        )));
    }
    if project.project_id != expected_id {
        return Err(ProjectError::Invalid(
            "project identifier does not match its directory".to_owned(),
        ));
    }
    if project
        .active_revision_id
        .is_some_and(|id| !project.revision_ids.contains(&id))
        || project
            .root_revision_id
            .is_some_and(|id| !project.revision_ids.contains(&id))
    {
        return Err(ProjectError::Invalid(
            "project references a revision outside its revision list".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn current_timestamp() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn now() -> Result<String> {
    current_timestamp()
}

pub(crate) fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_new_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

fn write_json_recoverable(path: &Path, value: &impl Serialize) -> Result<()> {
    let temp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let backup = path.with_extension("json.backup");
    write_new_json(&temp, value)?;

    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    if path.exists() {
        fs::rename(path, &backup)?;
    }
    if let Err(error) = fs::rename(&temp, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    if backup.exists() {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

fn recover_json_file(path: &Path) -> Result<()> {
    let backup = path.with_extension("json.backup");
    match (path.exists(), backup.exists()) {
        (false, true) => fs::rename(backup, path)?,
        (true, true) => {
            let _ = fs::remove_file(backup);
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("3dmk-project-test-{}", Uuid::new_v4()));
            fs::create_dir(&path).expect("test directory should be created");
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn project_assets_and_revisions_survive_reopen() {
        let directory = TestDir::new();
        let store = ProjectStore::open(&directory.0).unwrap();
        let project = store.create_project("Room scan").unwrap();
        let source = store
            .store_source_asset(
                project.project_id,
                r"..\capture\room.ply",
                "application/ply",
                AssetRole::SourceModel,
                Cursor::new(b"ply\nsource".to_vec()),
            )
            .unwrap();
        assert_eq!(source.original_name, "room.ply");

        let root = store
            .create_root_revision(
                project.project_id,
                source.asset_id,
                SceneKind::PointCloud,
                AttributeContract {
                    colors: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let derived = store
            .store_derived_asset(
                project.project_id,
                "cleaned.ply",
                "application/ply",
                AssetRole::DerivedScene,
                Cursor::new(b"ply\ncleaned".to_vec()),
            )
            .unwrap();
        let child = store
            .create_child_revision(
                project.project_id,
                root.revision_id,
                derived.asset_id,
                SceneKind::PointCloud,
                TopologyRelation::Subselection,
                AttributeContract {
                    colors: true,
                    source_ids: true,
                    ..Default::default()
                },
                Some(Uuid::new_v4()),
            )
            .unwrap();
        store
            .set_active_revision(project.project_id, root.revision_id)
            .unwrap();
        drop(store);

        let reopened = ProjectStore::open(&directory.0).unwrap();
        let loaded = reopened.load_project(project.project_id).unwrap();
        assert_eq!(loaded.root_revision_id, Some(root.revision_id));
        assert_eq!(loaded.active_revision_id, Some(root.revision_id));
        assert_eq!(
            loaded.revision_ids,
            vec![root.revision_id, child.revision_id]
        );
        assert_eq!(
            reopened.list_revisions(project.project_id).unwrap().len(),
            2
        );
        assert_eq!(reopened.list_projects().unwrap().len(), 1);
    }

    #[test]
    fn identical_asset_content_is_deduplicated_by_digest() {
        let directory = TestDir::new();
        let store = ProjectStore::open(&directory.0).unwrap();
        let project = store.create_project("Dedup").unwrap();
        let first = store
            .store_source_asset(
                project.project_id,
                "first.bin",
                "application/octet-stream",
                AssetRole::Other,
                Cursor::new(b"same bytes".to_vec()),
            )
            .unwrap();
        let second = store
            .store_source_asset(
                project.project_id,
                "second.bin",
                "application/octet-stream",
                AssetRole::Other,
                Cursor::new(b"same bytes".to_vec()),
            )
            .unwrap();
        assert_eq!(first.asset_id, second.asset_id);
        assert_eq!(first.sha256, second.sha256);
        assert_eq!(
            store
                .load_project(project.project_id)
                .unwrap()
                .source_asset_ids,
            vec![first.asset_id]
        );
    }

    #[test]
    fn root_scene_import_publishes_complete_project_once() {
        let directory = TestDir::new();
        let store = ProjectStore::open(&directory.0).unwrap();
        let (project, asset, revision) = store
            .import_root_scene(
                RootSceneImport {
                    project_name: "Imported room".to_owned(),
                    original_name: r"..\capture\room.glb".to_owned(),
                    media_type: "model/gltf-binary".to_owned(),
                    scene_kind: SceneKind::Mesh,
                    attributes: AttributeContract {
                        normals: true,
                        uvs: true,
                        ..Default::default()
                    },
                    original_units: Some("metres".to_owned()),
                    warnings: vec![],
                },
                Cursor::new(b"glb bytes".to_vec()),
            )
            .unwrap();

        assert_eq!(project.source_asset_ids, vec![asset.asset_id]);
        assert_eq!(project.root_revision_id, Some(revision.revision_id));
        assert_eq!(project.active_revision_id, Some(revision.revision_id));
        assert_eq!(asset.original_name, "room.glb");
        assert_eq!(
            store.load_project(project.project_id).unwrap().revision_ids,
            vec![revision.revision_id]
        );
    }

    #[test]
    fn failed_root_scene_import_leaves_no_published_project() {
        let directory = TestDir::new();
        let store = ProjectStore::open_with_asset_limit(&directory.0, 3).unwrap();
        let result = store.import_root_scene(
            RootSceneImport {
                project_name: "Too large".to_owned(),
                original_name: "room.glb".to_owned(),
                media_type: "model/gltf-binary".to_owned(),
                scene_kind: SceneKind::Mesh,
                attributes: AttributeContract::default(),
                original_units: Some("metres".to_owned()),
                warnings: vec![],
            },
            Cursor::new(vec![1, 2, 3, 4]),
        );

        assert!(matches!(result, Err(ProjectError::AssetTooLarge)));
        assert!(store.list_projects().unwrap().is_empty());
    }

    #[test]
    fn package_scene_import_preserves_model_texture_and_paths_after_reopen() {
        let directory = TestDir::new();
        let model_path = directory.0.join("model.glb");
        let texture_path = directory.0.join("albedo.png");
        fs::write(&model_path, b"glb package payload").unwrap();
        fs::write(&texture_path, b"png texture payload").unwrap();
        let store = ProjectStore::open(&directory.0).unwrap();

        let imported = store
            .import_package_scene(PackageSceneImport {
                project_name: "Textured package".to_owned(),
                original_units: Some("metres".to_owned()),
                warnings: vec![],
                primary_package_path: "models/room.glb".to_owned(),
                scene_kind: SceneKind::Mesh,
                attributes: AttributeContract {
                    normals: true,
                    uvs: true,
                    material_ids: true,
                    ..Default::default()
                },
                assets: vec![
                    PackageAssetImport {
                        source_path: model_path,
                        package_paths: vec!["models/room.glb".to_owned()],
                        original_name: "room.glb".to_owned(),
                        media_type: "model/gltf-binary".to_owned(),
                        role: AssetRole::SourceModel,
                        source: true,
                        expected_sha256: None,
                        expected_bytes: Some(19),
                        metadata: serde_json::json!({}),
                    },
                    PackageAssetImport {
                        source_path: texture_path,
                        package_paths: vec!["textures/albedo.png".to_owned()],
                        original_name: "albedo.png".to_owned(),
                        media_type: "image/png".to_owned(),
                        role: AssetRole::Texture,
                        source: true,
                        expected_sha256: None,
                        expected_bytes: Some(19),
                        metadata: serde_json::json!({ "color_space": "srgb" }),
                    },
                ],
            })
            .unwrap();

        assert_eq!(imported.assets.len(), 2);
        assert_eq!(imported.project.source_asset_ids.len(), 2);
        assert_eq!(
            imported.root_revision.attribute_contract,
            AttributeContract {
                normals: true,
                uvs: true,
                material_ids: true,
                ..Default::default()
            }
        );
        drop(store);

        let reopened = ProjectStore::open(&directory.0).unwrap();
        let assets = reopened.list_assets(imported.project.project_id).unwrap();
        assert_eq!(assets.len(), 2);
        let texture = assets
            .iter()
            .find(|asset| asset.role == AssetRole::Texture)
            .unwrap();
        assert_eq!(
            texture.metadata["package_paths"],
            serde_json::json!(["textures/albedo.png"])
        );
        let revision = reopened
            .load_revision(
                imported.project.project_id,
                imported.root_revision.revision_id,
            )
            .unwrap();
        assert_eq!(
            revision.scene_asset_id,
            imported.root_revision.scene_asset_id
        );
    }

    #[test]
    fn package_scene_hash_mismatch_leaves_no_published_project() {
        let directory = TestDir::new();
        let model_path = directory.0.join("model.glb");
        fs::write(&model_path, b"glb package payload").unwrap();
        let store = ProjectStore::open(&directory.0).unwrap();

        let result = store.import_package_scene(PackageSceneImport {
            project_name: "Changed package".to_owned(),
            original_units: Some("metres".to_owned()),
            warnings: vec![],
            primary_package_path: "model.glb".to_owned(),
            scene_kind: SceneKind::Mesh,
            attributes: AttributeContract::default(),
            assets: vec![PackageAssetImport {
                source_path: model_path,
                package_paths: vec!["model.glb".to_owned()],
                original_name: "model.glb".to_owned(),
                media_type: "model/gltf-binary".to_owned(),
                role: AssetRole::SourceModel,
                source: true,
                expected_sha256: Some("00".repeat(32)),
                expected_bytes: Some(19),
                metadata: serde_json::json!({}),
            }],
        });

        assert!(matches!(result, Err(ProjectError::Invalid(_))));
        assert!(store.list_projects().unwrap().is_empty());
    }

    #[test]
    fn exported_project_is_a_reopenable_package_with_manifest_and_assets() {
        let directory = TestDir::new();
        let model_path = directory.0.join("model.glb");
        fs::write(&model_path, b"glb package payload").unwrap();
        let store = ProjectStore::open(&directory.0).unwrap();
        let imported = store
            .import_package_scene(PackageSceneImport {
                project_name: "Export package".to_owned(),
                original_units: Some("metres".to_owned()),
                warnings: vec![],
                primary_package_path: "models/model.glb".to_owned(),
                scene_kind: SceneKind::Mesh,
                attributes: AttributeContract::default(),
                assets: vec![PackageAssetImport {
                    source_path: model_path,
                    package_paths: vec!["models/model.glb".to_owned()],
                    original_name: "model.glb".to_owned(),
                    media_type: "model/gltf-binary".to_owned(),
                    role: AssetRole::SourceModel,
                    source: true,
                    expected_sha256: None,
                    expected_bytes: Some(19),
                    metadata: serde_json::json!({}),
                }],
            })
            .unwrap();
        let output = directory.0.join("export.zip");
        store
            .export_package(imported.project.project_id, &output)
            .unwrap();
        let file = File::open(output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("3dmk-package.json").is_ok());
        assert!(archive.by_name("models/model.glb").is_ok());
    }

    #[test]
    fn asset_limit_fails_without_publishing_partial_data() {
        let directory = TestDir::new();
        let store = ProjectStore::open_with_asset_limit(&directory.0, 3).unwrap();
        let project = store.create_project("Limits").unwrap();
        let result = store.store_source_asset(
            project.project_id,
            "large.bin",
            "application/octet-stream",
            AssetRole::Other,
            Cursor::new(vec![1, 2, 3, 4]),
        );
        assert!(matches!(result, Err(ProjectError::AssetTooLarge)));
        assert!(store
            .load_project(project.project_id)
            .unwrap()
            .source_asset_ids
            .is_empty());
        assert_eq!(
            fs::read_dir(store.project_dir(project.project_id).join("assets"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn missing_project_metadata_recovers_from_backup() {
        let directory = TestDir::new();
        let store = ProjectStore::open(&directory.0).unwrap();
        let project = store.create_project("Recovery").unwrap();
        let metadata = store.project_dir(project.project_id).join("project.json");
        let backup = metadata.with_extension("json.backup");
        fs::rename(&metadata, &backup).unwrap();

        let loaded = store.load_project(project.project_id).unwrap();
        assert_eq!(loaded.project_id, project.project_id);
        assert!(metadata.exists());
        assert!(!backup.exists());
    }

    #[test]
    fn invalid_names_and_unknown_revisions_are_rejected() {
        let directory = TestDir::new();
        let store = ProjectStore::open(&directory.0).unwrap();
        assert!(matches!(
            store.create_project(" \n "),
            Err(ProjectError::Invalid(_))
        ));
        let project = store.create_project("Valid").unwrap();
        assert!(matches!(
            store.set_active_revision(project.project_id, Uuid::new_v4()),
            Err(ProjectError::NotFound)
        ));
    }
}
