use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use federated_rag_contract_governance::DomainProfile;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::projects::{recover_json_file, write_json_recoverable};

pub const DEFAULT_PROFILE: &str = include_str!("../config/default-domain.json");
const PANEL_IDS: &[&str] = &["import", "scene_tree", "analysis", "measure", "export"];
const MAX_INSTALLED_PROFILES: usize = 64;
const MAX_CONFIG_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum DomainPreferenceError {
    #[error("domain profile is invalid: {0}")]
    Contract(#[from] federated_rag_contract_governance::DomainProfileError),
    #[error("domain preference storage failed: {0}")]
    Storage(#[from] crate::projects::ProjectError),
    #[error("domain profile storage failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("domain profile serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("domain configuration is unsupported: {0}")]
    Unsupported(String),
    #[error("domain profile was not found")]
    NotFound,
    #[error("domain preference lock is unavailable")]
    Lock,
}

pub type Result<T> = std::result::Result<T, DomainPreferenceError>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DomainSelection {
    pub domain_id: String,
    pub domain_version: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainSummary {
    pub id: String,
    pub version: String,
    pub label: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainState {
    pub active: Option<DomainSelection>,
    pub effective: DomainSelection,
    pub profiles: Vec<DomainSummary>,
    pub diagnostic: Option<String>,
    pub recovery_required: bool,
    pub locked: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredProfile {
    profile: DomainProfile,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SavedConfiguration {
    schema_version: u32,
    active: Option<DomainSelection>,
    active_sha256: Option<String>,
    installed: Vec<StoredProfile>,
}

impl Default for SavedConfiguration {
    fn default() -> Self {
        Self {
            schema_version: 1,
            active: None,
            active_sha256: None,
            installed: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct DomainPreferenceStore {
    root: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl DomainPreferenceStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        validate_for_3dmk(&DomainProfile::from_json_str(DEFAULT_PROFILE)?)?;
        Ok(Self {
            root,
            lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn state(&self) -> Result<DomainState> {
        let _guard = self.lock.lock().map_err(|_| DomainPreferenceError::Lock)?;
        self.state_unlocked()
    }

    pub fn preview_profile(&self, source: &str) -> Result<DomainProfile> {
        let profile = DomainProfile::from_json_str(source)?;
        validate_for_3dmk(&profile)?;
        if profile.domain.id == default_profile()?.domain.id {
            return Err(DomainPreferenceError::Unsupported(
                "the bundled default domain cannot be replaced".into(),
            ));
        }
        Ok(profile)
    }

    pub fn apply_profile(
        &self,
        domain_id: &str,
        domain_version: &str,
        source: Option<&str>,
    ) -> Result<DomainState> {
        let _guard = self.lock.lock().map_err(|_| DomainPreferenceError::Lock)?;
        let mut saved = match self.load_unlocked() {
            Ok(saved) => saved,
            Err(_) if source.is_none() && domain_id == default_profile()?.domain.id => {
                SavedConfiguration::default()
            }
            Err(error) => return Err(error),
        };
        if let Some(source) = source {
            let profile = self.preview_profile(source)?;
            if profile.domain.id != domain_id || profile.domain.version != domain_version {
                return Err(DomainPreferenceError::Unsupported(
                    "the profile content does not match the selected identity".into(),
                ));
            }
            let sha256 = profile_digest(&profile)?;
            if let Some(existing) = saved.installed.iter().find(|item| {
                item.profile.domain.id == domain_id && item.profile.domain.version == domain_version
            }) {
                if existing.sha256 != sha256 {
                    return Err(DomainPreferenceError::Unsupported(
                        "this domain version already has different content".into(),
                    ));
                }
            } else {
                if saved.installed.len() >= MAX_INSTALLED_PROFILES {
                    return Err(DomainPreferenceError::Unsupported(
                        "installed domain profile limit reached".into(),
                    ));
                }
                saved.installed.push(StoredProfile { profile, sha256 });
            }
        }
        let profile = find_profile(&saved, domain_id, domain_version)?
            .ok_or(DomainPreferenceError::NotFound)?;
        let digest = profile_digest(&profile)?;
        saved.active = Some(DomainSelection {
            domain_id: domain_id.into(),
            domain_version: domain_version.into(),
        });
        saved.active_sha256 = Some(digest);
        write_json_recoverable(&self.preference_path(), &saved)?;
        self.state_unlocked()
    }

    pub fn select(&self, domain_id: &str, domain_version: &str) -> Result<DomainState> {
        self.apply_profile(domain_id, domain_version, None)
    }

    pub fn profile(&self, domain_id: &str, domain_version: &str) -> Result<DomainProfile> {
        let _guard = self.lock.lock().map_err(|_| DomainPreferenceError::Lock)?;
        let default = default_profile()?;
        if default.domain.id == domain_id && default.domain.version == domain_version {
            return Ok(default);
        }
        let saved = self.load_unlocked()?;
        find_profile(&saved, domain_id, domain_version)?.ok_or(DomainPreferenceError::NotFound)
    }

    fn state_unlocked(&self) -> Result<DomainState> {
        let default = default_profile()?;
        let fallback = DomainSelection {
            domain_id: default.domain.id.clone(),
            domain_version: default.domain.version.clone(),
        };
        let mut profiles = vec![summary(&default)?];
        let (saved, diagnostic) = match self.load_unlocked() {
            Ok(saved) => (saved, None),
            Err(error) => (SavedConfiguration::default(), Some(format!("Saved domain configuration is invalid: {error}. Confirm a replacement domain to recover."))),
        };
        for item in &saved.installed {
            profiles.push(summary(&item.profile)?);
        }
        profiles.sort_by(|left, right| (&left.id, &left.version).cmp(&(&right.id, &right.version)));
        let active = saved.active;
        let effective = active.clone().unwrap_or(fallback);
        Ok(DomainState {
            active,
            effective,
            profiles,
            recovery_required: diagnostic.is_some(),
            diagnostic,
            locked: false,
        })
    }

    fn load_unlocked(&self) -> Result<SavedConfiguration> {
        let path = self.preference_path();
        recover_json_file(&path)?;
        if !path.exists() {
            return Ok(SavedConfiguration::default());
        }
        if fs::metadata(&path)?.len() > MAX_CONFIG_BYTES {
            return Err(DomainPreferenceError::Unsupported(
                "saved configuration exceeds the size limit".into(),
            ));
        }
        let saved: SavedConfiguration = serde_json::from_slice(&fs::read(path)?)?;
        if saved.schema_version != 1 || saved.installed.len() > MAX_INSTALLED_PROFILES {
            return Err(DomainPreferenceError::Unsupported(
                "saved configuration version or profile count is unsupported".into(),
            ));
        }
        let mut identities = std::collections::HashSet::new();
        for item in &saved.installed {
            validate_for_3dmk(&item.profile)?;
            if item.profile.domain.id == default_profile()?.domain.id
                || !identities.insert((
                    item.profile.domain.id.as_str(),
                    item.profile.domain.version.as_str(),
                ))
                || profile_digest(&item.profile)? != item.sha256
            {
                return Err(DomainPreferenceError::Unsupported(
                    "saved profile identity or fingerprint is invalid".into(),
                ));
            }
        }
        match (&saved.active, &saved.active_sha256) {
            (None, None) => {}
            (Some(selection), Some(digest)) => {
                let profile =
                    find_profile(&saved, &selection.domain_id, &selection.domain_version)?
                        .ok_or(DomainPreferenceError::NotFound)?;
                if profile_digest(&profile)? != *digest {
                    return Err(DomainPreferenceError::Unsupported(
                        "active profile fingerprint changed".into(),
                    ));
                }
            }
            _ => {
                return Err(DomainPreferenceError::Unsupported(
                    "saved active profile identity is incomplete".into(),
                ))
            }
        }
        Ok(saved)
    }

    fn preference_path(&self) -> PathBuf {
        self.root.join("domain-preference.json")
    }
}

fn default_profile() -> Result<DomainProfile> {
    Ok(DomainProfile::from_json_str(DEFAULT_PROFILE)?)
}

fn find_profile(
    saved: &SavedConfiguration,
    domain_id: &str,
    domain_version: &str,
) -> Result<Option<DomainProfile>> {
    let default = default_profile()?;
    if default.domain.id == domain_id && default.domain.version == domain_version {
        return Ok(Some(default));
    }
    Ok(saved
        .installed
        .iter()
        .find(|item| {
            item.profile.domain.id == domain_id && item.profile.domain.version == domain_version
        })
        .map(|item| item.profile.clone()))
}

fn profile_digest(profile: &DomainProfile) -> Result<String> {
    let bytes = serde_json::to_vec(profile)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn summary(profile: &DomainProfile) -> Result<DomainSummary> {
    Ok(DomainSummary {
        id: profile.domain.id.clone(),
        version: profile.domain.version.clone(),
        label: profile.domain.label.clone(),
        sha256: profile_digest(profile)?,
    })
}

fn validate_for_3dmk(profile: &DomainProfile) -> Result<()> {
    profile.validate()?;
    if profile.composition_id.is_some() {
        return Err(DomainPreferenceError::Unsupported(
            "3DMK is a child product, not a registered Veritas composition".into(),
        ));
    }
    for panel in &profile.features.workstation_panels {
        if !PANEL_IDS.contains(&panel.as_str()) {
            return Err(DomainPreferenceError::Unsupported(format!(
                "unknown workstation panel: {panel}"
            )));
        }
    }
    if !profile.features.capability_ids.is_empty() {
        return Err(DomainPreferenceError::Unsupported(
            "capability IDs need a registered 3DMK adapter before they can be selected".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!("3dmk-domain-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn default_is_application_wide_and_survives_restart() {
        let root = temporary_root();
        let store = DomainPreferenceStore::open(&root).unwrap();
        assert_eq!(
            store.state().unwrap().effective.domain_id,
            "general_geometry"
        );
        store.select("general_geometry", "1.0.0").unwrap();
        assert_eq!(
            DomainPreferenceStore::open(&root)
                .unwrap()
                .state()
                .unwrap()
                .active
                .unwrap()
                .domain_id,
            "general_geometry"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_saved_preference_requires_explicit_reselection() {
        let root = temporary_root();
        let store = DomainPreferenceStore::open(&root).unwrap();
        fs::write(root.join("domain-preference.json"), b"{invalid").unwrap();
        let state = store.state().unwrap();
        assert!(state.active.is_none());
        assert!(state.recovery_required);
        store.select("general_geometry", "1.0.0").unwrap();
        assert!(store.state().unwrap().diagnostic.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preview_does_not_install_and_apply_persists_atomically() {
        let root = temporary_root();
        let store = DomainPreferenceStore::open(&root).unwrap();
        let mut profile: serde_json::Value = serde_json::from_str(DEFAULT_PROFILE).unwrap();
        profile["domain"]["id"] = serde_json::json!("survey_review");
        let source = profile.to_string();
        store.preview_profile(&source).unwrap();
        assert_eq!(store.state().unwrap().profiles.len(), 1);
        store
            .apply_profile("survey_review", "1.0.0", Some(&source))
            .unwrap();
        let reopened = DomainPreferenceStore::open(&root).unwrap();
        assert_eq!(
            reopened.state().unwrap().effective.domain_id,
            "survey_review"
        );
        assert_eq!(reopened.state().unwrap().profiles.len(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_profile_content_changes_under_an_existing_identity() {
        let root = temporary_root();
        let store = DomainPreferenceStore::open(&root).unwrap();
        let mut profile: serde_json::Value = serde_json::from_str(DEFAULT_PROFILE).unwrap();
        profile["domain"]["id"] = serde_json::json!("built_environment");
        let source = profile.to_string();
        store
            .apply_profile("built_environment", "1.0.0", Some(&source))
            .unwrap();
        profile["domain"]["label"] = serde_json::json!("Changed label");
        assert!(store
            .apply_profile("built_environment", "1.0.0", Some(&profile.to_string()))
            .is_err());
        let mut saved: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("domain-preference.json")).unwrap())
                .unwrap();
        saved["installed"][0]["profile"]["domain"]["label"] = serde_json::json!("Tampered");
        fs::write(root.join("domain-preference.json"), saved.to_string()).unwrap();
        assert!(store.state().unwrap().recovery_required);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_semantically_invalid_profile_even_with_matching_digest() {
        let root = temporary_root();
        let store = DomainPreferenceStore::open(&root).unwrap();
        let mut profile: serde_json::Value = serde_json::from_str(DEFAULT_PROFILE).unwrap();
        profile["domain"]["id"] = serde_json::json!("built_environment");
        store
            .apply_profile("built_environment", "1.0.0", Some(&profile.to_string()))
            .unwrap();
        let path = root.join("domain-preference.json");
        let mut saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        saved["installed"][0]["profile"]["schema_version"] = serde_json::json!("999.0.0");
        let invalid: DomainProfile =
            serde_json::from_value(saved["installed"][0]["profile"].clone()).unwrap();
        let digest = profile_digest(&invalid).unwrap();
        saved["installed"][0]["sha256"] = serde_json::json!(digest);
        saved["active_sha256"] = saved["installed"][0]["sha256"].clone();
        fs::write(&path, saved.to_string()).unwrap();
        assert!(store.state().unwrap().recovery_required);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_unknown_panels() {
        let root = temporary_root();
        let store = DomainPreferenceStore::open(&root).unwrap();
        let mut profile: serde_json::Value = serde_json::from_str(DEFAULT_PROFILE).unwrap();
        profile["domain"]["id"] = serde_json::json!("built_environment");
        profile["features"]["workstation_panels"] = serde_json::json!(["unknown_panel"]);
        assert!(store.preview_profile(&profile.to_string()).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
