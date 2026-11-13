//! Bounded, Rust-owned placement and review state. This document references immutable
//! project assets; it never owns or rewrites source geometry. All distances are metres.
//!
//! `SceneState::apply` stages changes on a clone and publishes only a validated result.
//! A ProjectStore caller MUST hold its project/store lock across load, apply, and save.
//! The file helpers deliberately do not introduce another independently locked store.

use crate::projects::LengthUnit;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use uuid::Uuid;

pub const SCENE_SCHEMA_VERSION: u32 = 1;
pub const SCENE_FILE_NAME: &str = "scene_document.json";
pub const MAX_HISTORY_STEPS: usize = 32;
pub const MAX_SCENE_OBJECTS: usize = 4096;
pub const MAX_NAMED_VIEWS: usize = 64;
pub const MAX_COMMAND_RECEIPTS: usize = 10000;
pub const MAX_OBJECT_IDENTITIES: usize = 16384;
pub const MAX_SCENE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_HIERARCHY_DEPTH: usize = 64;
const UNIT_TOLERANCE: f64 = 1.0e-6;
const MIN_SCALE: f64 = 1.0e-12;

type Result<T> = std::result::Result<T, SceneError>;

#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    #[error("invalid scene document: {0}")]
    Invalid(String),
    #[error("unsupported scene schema version {0}")]
    UnsupportedSchema(u32),
    #[error("scene document belongs to a different project")]
    ProjectMismatch,
    #[error("stale scene revision: expected {expected}, current {actual}")]
    StaleRevision { expected: u64, actual: u64 },
    #[error("command ID {0} was already used for a different request")]
    CommandConflict(Uuid),
    #[error("scene object {0} was not found")]
    MissingObject(Uuid),
    #[error("source asset {0} is not present in this project")]
    MissingAsset(Uuid),
    #[error("object {object_id} refers to missing parent {parent_id}")]
    MissingParent { object_id: Uuid, parent_id: Uuid },
    #[error("scene graph contains a cycle through object {0}")]
    GraphCycle(Uuid),
    #[error("object {0} is locked; unlock it before changing placement or membership")]
    LockedObject(Uuid),
    #[error("object ID {0} already has an immutable source identity and cannot be reused")]
    ObjectIdentityConflict(Uuid),
    #[error("view {0} was not found")]
    MissingView(Uuid),
    #[error("view {view_id} refers to missing scene object {object_id}")]
    MissingViewObject { view_id: Uuid, object_id: Uuid },
    #[error("scene document limit reached: {0}")]
    Limit(String),
    #[error("there is no scene command to {0}")]
    EmptyHistory(&'static str),
    #[error("scene document filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("scene document JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameKind {
    #[default]
    ProjectLocal,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Handedness {
    #[default]
    Right,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpAxis {
    X,
    #[default]
    Y,
    Z,
}

/// CRS/datum strings are declarations only. No survey conversion is inferred.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrameMetadata {
    pub kind: FrameKind,
    pub handedness: Handedness,
    pub up_axis: UpAxis,
    pub origin_metres: [f64; 3],
    pub horizontal_crs: Option<String>,
    pub vertical_datum: Option<String>,
}

/// Quaternion component order is x, y, z, w. Translation is parent-local metres.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalTransform {
    pub translation: [f64; 3],
    pub rotation: [f64; 4],
    pub scale: [f64; 3],
}
impl Default for LocalTransform {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneObjectKind {
    Group,
    Instance,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneObjectRole {
    #[default]
    Model,
    Reference,
    Plan,
    Actual,
    Top,
    Section,
    Slice,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneObject {
    pub object_id: Uuid,
    pub name: String,
    pub kind: SceneObjectKind,
    #[serde(default)]
    pub role: SceneObjectRole,
    pub parent_id: Option<Uuid>,
    pub asset_id: Option<Uuid>,
    /// Immutable source selector: None renders the whole asset; Some renders only
    /// the indexed GLB node's own primitives, excluding its child nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_node_index: Option<u32>,
    /// Optional source subtree normalization; immutable source/node transforms remain unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_up_axis: Option<UpAxis>,
    pub transform: LocalTransform,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f64,
}
impl SceneObject {
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            object_id: Uuid::new_v4(),
            name: name.into(),
            kind: SceneObjectKind::Group,
            role: SceneObjectRole::Model,
            parent_id: None,
            asset_id: None,
            asset_node_index: None,
            source_up_axis: None,
            transform: LocalTransform::default(),
            visible: true,
            locked: false,
            opacity: 1.0,
        }
    }
    pub fn instance(name: impl Into<String>, asset_id: Uuid) -> Self {
        Self {
            kind: SceneObjectKind::Instance,
            asset_id: Some(asset_id),
            ..Self::group(name)
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CameraProjection {
    Perspective {
        vertical_fov_degrees: f64,
        near: f64,
        far: f64,
    },
    Orthographic {
        height_metres: f64,
        near: f64,
        far: f64,
    },
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ViewCamera {
    pub projection: CameraProjection,
    pub position: [f64; 3],
    pub rotation: [f64; 4],
    pub target: [f64; 3],
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClipPlane {
    /// Project-local unit normal; kept half-space is normal dot point + offset >= 0.
    pub normal: [f64; 3],
    pub offset_metres: f64,
}
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClippingState {
    pub enabled: bool,
    pub planes: Vec<ClipPlane>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SectionState {
    pub name: String,
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    pub up: [f64; 3],
    pub thickness_metres: f64,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NamedView {
    pub view_id: Uuid,
    pub name: String,
    pub camera: ViewCamera,
    pub visibility: BTreeMap<Uuid, bool>,
    pub clipping: ClippingState,
    pub section: Option<SectionState>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneDocument {
    pub schema_version: u32,
    pub project_id: Uuid,
    pub revision: u64,
    pub units: LengthUnit,
    pub frame: FrameMetadata,
    pub objects: BTreeMap<Uuid, SceneObject>,
    pub views: BTreeMap<Uuid, NamedView>,
    pub active_view_id: Option<Uuid>,
    /// Current camera/clipping snapshot, independent of whether a named view is active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_view: Option<NamedView>,
}
impl SceneDocument {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            schema_version: SCENE_SCHEMA_VERSION,
            project_id,
            revision: 0,
            units: LengthUnit::Metres,
            frame: FrameMetadata::default(),
            objects: BTreeMap::new(),
            views: BTreeMap::new(),
            active_view_id: None,
            camera_view: None,
        }
    }
    pub fn validate(&self, allowed_assets: &HashSet<Uuid>) -> Result<()> {
        if self.schema_version != SCENE_SCHEMA_VERSION {
            return Err(SceneError::UnsupportedSchema(self.schema_version));
        }
        valid_id(self.project_id, "project ID")?;
        finite(&self.frame.origin_metres, "frame origin")?;
        for (label, declaration) in [
            ("horizontal CRS", &self.frame.horizontal_crs),
            ("vertical datum", &self.frame.vertical_datum),
        ] {
            if let Some(value) = declaration {
                valid_label(value, label, 256)?;
            }
        }
        validate_snapshot(&SceneSnapshot::from(self), allowed_assets)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCommandRequest {
    pub command_id: Uuid,
    pub expected_revision: u64,
    pub command: SceneCommand,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneCommand {
    AddObject {
        object: SceneObject,
    },
    /// Insert a complete placement hierarchy atomically, in any vector order.
    AddObjects {
        objects: Vec<SceneObject>,
    },
    RemoveObject {
        object_id: Uuid,
    },
    SetTransform {
        object_id: Uuid,
        transform: LocalTransform,
    },
    SetSourceUpAxis {
        object_id: Uuid,
        up_axis: UpAxis,
    },
    SetParent {
        object_id: Uuid,
        parent_id: Option<Uuid>,
    },
    RenameObject {
        object_id: Uuid,
        name: String,
    },
    SetRole {
        object_id: Uuid,
        role: SceneObjectRole,
    },
    SetDisplay {
        object_id: Uuid,
        visible: bool,
        opacity: f64,
    },
    SetLocked {
        object_id: Uuid,
        locked: bool,
    },
    SaveView {
        view: NamedView,
    },
    RemoveView {
        view_id: Uuid,
    },
    ActivateView {
        view_id: Uuid,
        /// Actual pre-activation camera supplied by the client for durable Undo.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous_view: Option<NamedView>,
    },
    Undo {},
    Redo {},
}
#[derive(Clone, Debug, Serialize)]
pub struct SceneCommandResult {
    pub document: SceneDocument,
    pub duplicate: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SceneSnapshot {
    objects: BTreeMap<Uuid, SceneObject>,
    views: BTreeMap<Uuid, NamedView>,
    active_view_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    camera_view: Option<NamedView>,
}
impl From<&SceneDocument> for SceneSnapshot {
    fn from(doc: &SceneDocument) -> Self {
        Self {
            objects: doc.objects.clone(),
            views: doc.views.clone(),
            active_view_id: doc.active_view_id,
            camera_view: doc.camera_view.clone(),
        }
    }
}
impl SceneSnapshot {
    fn restore(self, doc: &mut SceneDocument) {
        doc.objects = self.objects;
        doc.views = self.views;
        doc.active_view_id = self.active_view_id;
        doc.camera_view = self.camera_view;
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ObjectIdentity {
    kind: SceneObjectKind,
    asset_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    asset_node_index: Option<u32>,
}
impl From<&SceneObject> for ObjectIdentity {
    fn from(object: &SceneObject) -> Self {
        Self {
            kind: object.kind,
            asset_id: object.asset_id,
            asset_node_index: object.asset_node_index,
        }
    }
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CommandReceipt {
    fingerprint: [u8; 32],
    applied_revision: u64,
}

/// Persist this envelope, not just `document`: it retains undo/redo and replay safety.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneState {
    pub document: SceneDocument,
    undo_history: Vec<SceneSnapshot>,
    redo_history: Vec<SceneSnapshot>,
    object_identities: BTreeMap<Uuid, ObjectIdentity>,
    command_receipts: BTreeMap<Uuid, CommandReceipt>,
}
impl SceneState {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            document: SceneDocument::new(project_id),
            undo_history: vec![],
            redo_history: vec![],
            object_identities: BTreeMap::new(),
            command_receipts: BTreeMap::new(),
        }
    }
    /// Bootstrap revision-zero placements from an existing project asset inventory.
    pub fn from_document(document: SceneDocument, allowed_assets: &HashSet<Uuid>) -> Result<Self> {
        if document.revision != 0 {
            return Err(SceneError::Invalid(
                "bootstrap document revision must be zero".into(),
            ));
        }
        let mut state = Self::new(document.project_id);
        state.object_identities = document
            .objects
            .iter()
            .map(|(id, object)| (*id, object.into()))
            .collect();
        state.document = document;
        state.validate(allowed_assets)?;
        Ok(state)
    }
    pub fn can_undo(&self) -> bool {
        !self.undo_history.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo_history.is_empty()
    }
    pub fn undo_depth(&self) -> usize {
        self.undo_history.len()
    }
    pub fn redo_depth(&self) -> usize {
        self.redo_history.len()
    }
    /// Immutable selectors include objects retained only in Undo/Redo history.
    pub fn source_node_selectors(&self) -> BTreeMap<Uuid, Vec<u32>> {
        let mut result: BTreeMap<Uuid, Vec<u32>> = BTreeMap::new();
        for identity in self.object_identities.values() {
            if let (Some(asset), Some(node)) = (identity.asset_id, identity.asset_node_index) {
                let nodes = result.entry(asset).or_default();
                if !nodes.contains(&node) {
                    nodes.push(node);
                }
            }
        }
        result
    }
    pub fn response(&self, duplicate: bool) -> SceneCommandResult {
        SceneCommandResult {
            document: self.document.clone(),
            duplicate,
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
        }
    }

    pub fn validate(&self, allowed_assets: &HashSet<Uuid>) -> Result<()> {
        self.document.validate(allowed_assets)?;
        if self.undo_history.len() > MAX_HISTORY_STEPS
            || self.redo_history.len() > MAX_HISTORY_STEPS
        {
            return Err(SceneError::Limit("undo/redo history".into()));
        }
        if self.command_receipts.len() > MAX_COMMAND_RECEIPTS
            || self.object_identities.len() > MAX_OBJECT_IDENTITIES
        {
            return Err(SceneError::Limit(
                "command receipts or object identities".into(),
            ));
        }
        if self.document.revision != self.command_receipts.len() as u64 {
            return Err(SceneError::Invalid(
                "revision and command receipt count do not agree".into(),
            ));
        }
        let mut revisions = HashSet::new();
        for (id, receipt) in &self.command_receipts {
            valid_id(*id, "command ID")?;
            if receipt.applied_revision == 0
                || receipt.applied_revision > self.document.revision
                || !revisions.insert(receipt.applied_revision)
            {
                return Err(SceneError::Invalid(
                    "command receipt revisions are inconsistent".into(),
                ));
            }
        }
        for (id, identity) in &self.object_identities {
            valid_id(*id, "object identity")?;
            validate_identity(
                identity.kind,
                identity.asset_id,
                identity.asset_node_index,
                allowed_assets,
            )?;
        }
        self.validate_identities(&self.document.objects)?;
        for snapshot in self.undo_history.iter().chain(&self.redo_history) {
            validate_snapshot(snapshot, allowed_assets)?;
            self.validate_identities(&snapshot.objects)?;
        }
        if serialized_size(self)? > MAX_SCENE_BYTES {
            return Err(SceneError::Limit(
                "serialized scene state exceeds 32 MiB".into(),
            ));
        }
        Ok(())
    }
    fn validate_identities(&self, objects: &BTreeMap<Uuid, SceneObject>) -> Result<()> {
        for (id, object) in objects {
            if self.object_identities.get(id) != Some(&ObjectIdentity::from(object)) {
                return Err(SceneError::ObjectIdentityConflict(*id));
            }
        }
        Ok(())
    }

    /// Caller must serialize this operation with project-local load/save and asset changes.
    /// A duplicate returns CURRENT state and `duplicate: true`, not a stale old snapshot.
    pub fn apply(
        &mut self,
        request: &SceneCommandRequest,
        allowed_assets: &HashSet<Uuid>,
    ) -> Result<SceneCommandResult> {
        self.validate(allowed_assets)?;
        valid_id(request.command_id, "command ID")?;
        let fingerprint: [u8; 32] = Sha256::digest(serde_json::to_vec(request)?).into();
        if let Some(receipt) = self.command_receipts.get(&request.command_id) {
            if receipt.fingerprint != fingerprint {
                return Err(SceneError::CommandConflict(request.command_id));
            }
            return Ok(self.response(true));
        }
        if request.expected_revision != self.document.revision {
            return Err(SceneError::StaleRevision {
                expected: request.expected_revision,
                actual: self.document.revision,
            });
        }
        if self.command_receipts.len() >= MAX_COMMAND_RECEIPTS {
            return Err(SceneError::Limit(
                "10000 accepted commands; create a new document before continuing".into(),
            ));
        }
        let mut candidate = self.clone();
        if let SceneCommand::ActivateView {
            previous_view: Some(previous),
            ..
        } = &request.command
        {
            // The current camera may have been orbited since the last named view. Stage
            // the real pre-command pose before history capture, without adding a preset.
            candidate.document.camera_view = Some(previous.clone());
            candidate.document.validate(allowed_assets)?;
        }
        let before = SceneSnapshot::from(&candidate.document);
        match &request.command {
            SceneCommand::Undo {} => {
                let previous = candidate
                    .undo_history
                    .pop()
                    .ok_or(SceneError::EmptyHistory("undo"))?;
                candidate.redo_history.push(before);
                previous.restore(&mut candidate.document);
            }
            SceneCommand::Redo {} => {
                let next = candidate
                    .redo_history
                    .pop()
                    .ok_or(SceneError::EmptyHistory("redo"))?;
                candidate.undo_history.push(before);
                next.restore(&mut candidate.document);
            }
            command => {
                candidate.apply_edit(command)?;
                candidate.undo_history.push(before);
                candidate.redo_history.clear();
            }
        }
        trim_history(&mut candidate.undo_history);
        trim_history(&mut candidate.redo_history);
        candidate.document.revision = self
            .document
            .revision
            .checked_add(1)
            .ok_or_else(|| SceneError::Limit("scene revision overflow".into()))?;
        candidate.command_receipts.insert(
            request.command_id,
            CommandReceipt {
                fingerprint,
                applied_revision: candidate.document.revision,
            },
        );
        // Large documents retain fewer snapshots rather than violating the byte budget.
        while serialized_size(&candidate)? > MAX_SCENE_BYTES {
            if !candidate.undo_history.is_empty() {
                candidate.undo_history.remove(0);
            } else if !candidate.redo_history.is_empty() {
                candidate.redo_history.remove(0);
            } else {
                return Err(SceneError::Limit(
                    "serialized scene state exceeds 32 MiB".into(),
                ));
            }
        }
        candidate.validate(allowed_assets)?;
        *self = candidate;
        Ok(self.response(false))
    }

    fn apply_edit(&mut self, command: &SceneCommand) -> Result<()> {
        match command {
            SceneCommand::AddObject { object } => {
                self.add_objects(std::slice::from_ref(object))?;
            }
            SceneCommand::AddObjects { objects } => {
                self.add_objects(objects)?;
            }
            SceneCommand::RemoveObject { object_id } => {
                let affected = self.ensure_movable_subtree(*object_id)?;
                for id in affected {
                    self.document.objects.remove(&id);
                    if let Some(camera) = &mut self.document.camera_view {
                        camera.visibility.remove(&id);
                    }
                }
                // Validation rejects dangling view references instead of silently erasing them.
            }
            SceneCommand::SetTransform {
                object_id,
                transform,
            } => {
                self.ensure_movable_subtree(*object_id)?;
                self.object_mut(*object_id)?.transform = transform.clone();
            }
            SceneCommand::SetSourceUpAxis { object_id, up_axis } => {
                self.ensure_movable_subtree(*object_id)?;
                let object = self.object_mut(*object_id)?;
                if object.kind != SceneObjectKind::Group {
                    return Err(SceneError::Invalid(
                        "source up-axis belongs to a source group".into(),
                    ));
                }
                object.source_up_axis = Some(*up_axis);
            }
            SceneCommand::SetParent {
                object_id,
                parent_id,
            } => {
                self.ensure_movable_subtree(*object_id)?;
                if let Some(parent) = parent_id {
                    self.ensure_parent(*object_id, *parent)?;
                    self.ensure_unlocked(*parent)?;
                }
                self.object_mut(*object_id)?.parent_id = *parent_id;
            }
            SceneCommand::RenameObject { object_id, name } => {
                self.ensure_unlocked(*object_id)?;
                self.object_mut(*object_id)?.name = name.clone();
            }
            SceneCommand::SetRole { object_id, role } => {
                self.ensure_unlocked(*object_id)?;
                self.object_mut(*object_id)?.role = *role;
            }
            SceneCommand::SetDisplay {
                object_id,
                visible,
                opacity,
            } => {
                let object = self.object_mut(*object_id)?;
                object.visible = *visible;
                object.opacity = *opacity;
            }
            SceneCommand::SetLocked { object_id, locked } => {
                if let Some(parent) = self.object(*object_id)?.parent_id {
                    self.ensure_unlocked(parent)?;
                }
                self.object_mut(*object_id)?.locked = *locked;
            }
            SceneCommand::SaveView { view } => {
                self.document.views.insert(view.view_id, view.clone());
            }
            SceneCommand::RemoveView { view_id } => {
                self.document
                    .views
                    .remove(view_id)
                    .ok_or(SceneError::MissingView(*view_id))?;
                if self.document.active_view_id == Some(*view_id) {
                    self.document.active_view_id = None;
                }
            }
            SceneCommand::ActivateView { view_id, .. } => {
                let view = self
                    .document
                    .views
                    .get(view_id)
                    .ok_or(SceneError::MissingView(*view_id))?
                    .clone();
                for (id, visible) in &view.visibility {
                    self.document
                        .objects
                        .get_mut(id)
                        .ok_or(SceneError::MissingViewObject {
                            view_id: *view_id,
                            object_id: *id,
                        })?
                        .visible = *visible;
                }
                self.document.active_view_id = Some(*view_id);
                self.document.camera_view = Some(view);
            }
            SceneCommand::Undo {} | SceneCommand::Redo {} => {
                unreachable!("history commands are dispatched by apply")
            }
        }
        Ok(())
    }
    /// Called only on the unpublished candidate. Reserve identities before checking
    /// parents so a batch can reference groups appearing later in the same batch.
    fn add_objects(&mut self, objects: &[SceneObject]) -> Result<()> {
        if objects.is_empty() {
            return Err(SceneError::Invalid("object batch must not be empty".into()));
        }
        if objects.len() > MAX_SCENE_OBJECTS.saturating_sub(self.document.objects.len()) {
            return Err(SceneError::Limit("object count".into()));
        }
        if objects.len() > MAX_OBJECT_IDENTITIES.saturating_sub(self.object_identities.len()) {
            return Err(SceneError::Limit("object identity count".into()));
        }
        let mut ids = HashSet::with_capacity(objects.len());
        for object in objects {
            if self.object_identities.contains_key(&object.object_id)
                || !ids.insert(object.object_id)
            {
                return Err(SceneError::ObjectIdentityConflict(object.object_id));
            }
        }
        for object in objects {
            self.object_identities
                .insert(object.object_id, object.into());
            self.document
                .objects
                .insert(object.object_id, object.clone());
        }
        // Check every immediate parent first for stable missing-parent diagnostics.
        for object in objects {
            if let Some(parent) = object.parent_id {
                self.ensure_parent(object.object_id, parent)?;
            }
        }
        for object in objects {
            if let Some(parent) = object.parent_id {
                self.ensure_unlocked(parent)?;
            }
        }
        Ok(())
    }
    fn object(&self, id: Uuid) -> Result<&SceneObject> {
        self.document
            .objects
            .get(&id)
            .ok_or(SceneError::MissingObject(id))
    }
    fn object_mut(&mut self, id: Uuid) -> Result<&mut SceneObject> {
        self.document
            .objects
            .get_mut(&id)
            .ok_or(SceneError::MissingObject(id))
    }
    fn ensure_parent(&self, object_id: Uuid, parent_id: Uuid) -> Result<()> {
        let parent = self
            .document
            .objects
            .get(&parent_id)
            .ok_or(SceneError::MissingParent {
                object_id,
                parent_id,
            })?;
        if parent.kind != SceneObjectKind::Group {
            return Err(SceneError::Invalid(
                "only group objects can be parents".into(),
            ));
        }
        Ok(())
    }
    fn ensure_unlocked(&self, object_id: Uuid) -> Result<()> {
        let mut next = Some(object_id);
        let mut ancestors = HashSet::new();
        while let Some(id) = next {
            if !ancestors.insert(id) {
                return Err(SceneError::GraphCycle(id));
            }
            if ancestors.len() > MAX_HIERARCHY_DEPTH {
                return Err(SceneError::Limit("scene hierarchy depth exceeds 64".into()));
            }
            let object = self.object(id)?;
            if object.locked {
                return Err(SceneError::LockedObject(id));
            }
            next = object.parent_id;
        }
        Ok(())
    }
    fn ensure_movable_subtree(&self, object_id: Uuid) -> Result<Vec<Uuid>> {
        self.ensure_unlocked(object_id)?;
        let mut affected = vec![object_id];
        let mut index = 0;
        while index < affected.len() {
            let parent_id = affected[index];
            for object in self
                .document
                .objects
                .values()
                .filter(|object| object.parent_id == Some(parent_id))
            {
                if object.locked {
                    return Err(SceneError::LockedObject(object.object_id));
                }
                affected.push(object.object_id);
            }
            index += 1;
        }
        Ok(affected)
    }
}

fn trim_history(history: &mut Vec<SceneSnapshot>) {
    if history.len() > MAX_HISTORY_STEPS {
        history.drain(..history.len() - MAX_HISTORY_STEPS);
    }
}
fn valid_id(id: Uuid, label: &str) -> Result<()> {
    if id.is_nil() {
        return Err(SceneError::Invalid(format!("{label} must not be nil")));
    }
    Ok(())
}
fn valid_label(value: &str, label: &str, max: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(SceneError::Invalid(format!(
            "{label} must be nonempty, at most {max} bytes, and contain no control characters"
        )));
    }
    Ok(())
}
fn finite(values: &[f64], label: &str) -> Result<()> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(SceneError::Invalid(format!(
            "{label} contains a non-finite number"
        )));
    }
    Ok(())
}
fn unit_vector(values: &[f64], label: &str) -> Result<()> {
    finite(values, label)?;
    let squared_length: f64 = values.iter().map(|value| value * value).sum();
    if (squared_length - 1.0).abs() > UNIT_TOLERANCE {
        return Err(SceneError::Invalid(format!("{label} must be normalized")));
    }
    Ok(())
}
fn validate_identity(
    kind: SceneObjectKind,
    asset_id: Option<Uuid>,
    asset_node_index: Option<u32>,
    allowed_assets: &HashSet<Uuid>,
) -> Result<()> {
    match (kind, asset_id, asset_node_index) {
        (SceneObjectKind::Group, None, None) => Ok(()),
        (SceneObjectKind::Instance, Some(id), _) => {
            valid_id(id, "asset ID")?;
            if !allowed_assets.contains(&id) {
                return Err(SceneError::MissingAsset(id));
            }
            Ok(())
        }
        _ => Err(SceneError::Invalid(
            "groups must not reference geometry; instances must reference one project asset".into(),
        )),
    }
}
fn validate_snapshot(snapshot: &SceneSnapshot, allowed_assets: &HashSet<Uuid>) -> Result<()> {
    if snapshot.objects.len() > MAX_SCENE_OBJECTS || snapshot.views.len() > MAX_NAMED_VIEWS {
        return Err(SceneError::Limit("object or named view count".into()));
    }
    for (id, object) in &snapshot.objects {
        valid_id(*id, "object ID")?;
        if *id != object.object_id {
            return Err(SceneError::Invalid(
                "object map key and object ID differ".into(),
            ));
        }
        valid_label(&object.name, "object name", 256)?;
        validate_identity(
            object.kind,
            object.asset_id,
            object.asset_node_index,
            allowed_assets,
        )?;
        if object.source_up_axis.is_some() && object.kind != SceneObjectKind::Group {
            return Err(SceneError::Invalid(
                "source up-axis belongs to a source group".into(),
            ));
        }
        finite(&object.transform.translation, "translation")?;
        unit_vector(&object.transform.rotation, "rotation quaternion")?;
        finite(&object.transform.scale, "scale")?;
        if object
            .transform
            .scale
            .iter()
            .any(|value| value.abs() < MIN_SCALE)
        {
            return Err(SceneError::Invalid(
                "scale must be nonsingular (absolute components >= 1e-12)".into(),
            ));
        }
        finite(&[object.opacity], "opacity")?;
        if !(0.0..=1.0).contains(&object.opacity) {
            return Err(SceneError::Invalid("opacity must be in [0,1]".into()));
        }
        let mut ancestors = HashSet::from([*id]);
        let mut current = object.parent_id;
        while let Some(parent_id) = current {
            if !ancestors.insert(parent_id) {
                return Err(SceneError::GraphCycle(parent_id));
            }
            if ancestors.len() > MAX_HIERARCHY_DEPTH {
                return Err(SceneError::Limit("scene hierarchy depth exceeds 64".into()));
            }
            let parent = snapshot
                .objects
                .get(&parent_id)
                .ok_or(SceneError::MissingParent {
                    object_id: *id,
                    parent_id,
                })?;
            if parent.kind != SceneObjectKind::Group {
                return Err(SceneError::Invalid(
                    "only group objects can be parents".into(),
                ));
            }
            current = parent.parent_id;
        }
    }
    for (id, view) in snapshot.views.iter().chain(
        snapshot
            .camera_view
            .as_ref()
            .map(|view| (&view.view_id, view)),
    ) {
        valid_id(*id, "view ID")?;
        if *id != view.view_id {
            return Err(SceneError::Invalid(
                "view map key and view ID differ".into(),
            ));
        }
        valid_label(&view.name, "view name", 256)?;
        finite(&view.camera.position, "camera position")?;
        finite(&view.camera.target, "camera target")?;
        unit_vector(&view.camera.rotation, "camera quaternion")?;
        if view.camera.position == view.camera.target {
            return Err(SceneError::Invalid(
                "camera position and target must differ".into(),
            ));
        }
        let (near, far) = match view.camera.projection {
            CameraProjection::Perspective {
                vertical_fov_degrees,
                near,
                far,
            } => {
                finite(&[vertical_fov_degrees], "vertical FOV")?;
                if !(0.0..180.0).contains(&vertical_fov_degrees) || vertical_fov_degrees == 0.0 {
                    return Err(SceneError::Invalid(
                        "vertical FOV must be between 0 and 180 degrees".into(),
                    ));
                }
                if near <= 0.0 {
                    return Err(SceneError::Invalid(
                        "perspective near distance must be positive".into(),
                    ));
                }
                (near, far)
            }
            CameraProjection::Orthographic {
                height_metres,
                near,
                far,
            } => {
                finite(&[height_metres], "orthographic height")?;
                if height_metres <= 0.0 {
                    return Err(SceneError::Invalid(
                        "orthographic height must be positive".into(),
                    ));
                }
                (near, far)
            }
        };
        finite(&[near, far], "camera clipping distances")?;
        if far <= near {
            return Err(SceneError::Invalid(
                "camera clipping distances require near < far".into(),
            ));
        }
        for object_id in view.visibility.keys() {
            if !snapshot.objects.contains_key(object_id) {
                return Err(SceneError::MissingViewObject {
                    view_id: *id,
                    object_id: *object_id,
                });
            }
        }
        if view.clipping.planes.len() > 6 {
            return Err(SceneError::Limit(
                "at most six clipping planes per view".into(),
            ));
        }
        for plane in &view.clipping.planes {
            unit_vector(&plane.normal, "clipping normal")?;
            finite(&[plane.offset_metres], "clipping offset")?;
        }
        if let Some(section) = &view.section {
            valid_label(&section.name, "section name", 256)?;
            finite(&section.origin, "section origin")?;
            unit_vector(&section.normal, "section normal")?;
            unit_vector(&section.up, "section up")?;
            finite(&[section.thickness_metres], "section thickness")?;
            let dot: f64 = section
                .normal
                .iter()
                .zip(section.up)
                .map(|(normal, up)| normal * up)
                .sum();
            if dot.abs() > UNIT_TOLERANCE || section.thickness_metres <= 0.0 {
                return Err(SceneError::Invalid(
                    "section requires orthogonal normal/up and positive thickness".into(),
                ));
            }
        }
    }
    if let Some(id) = snapshot.active_view_id {
        if !snapshot.views.contains_key(&id) {
            return Err(SceneError::MissingView(id));
        }
    }
    Ok(())
}

fn serialized_size(value: &impl Serialize) -> Result<u64> {
    struct Counter(u64);
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len() as u64);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value)?;
    Ok(counter.0)
}

/// Missing optional scene state returns revision-zero empty state; malformed state fails closed.
/// The caller holds the same project lock used by mutations and asset publication.
pub fn load_scene_state(
    project_dir: &Path,
    project_id: Uuid,
    allowed_assets: &HashSet<Uuid>,
) -> Result<SceneState> {
    let path = project_dir.join(SCENE_FILE_NAME);
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let state = SceneState::new(project_id);
            state.validate(allowed_assets)?;
            return Ok(state);
        }
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_SCENE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SCENE_BYTES {
        return Err(SceneError::Limit("scene file exceeds 32 MiB".into()));
    }
    let state: SceneState = serde_json::from_slice(&bytes)?;
    if state.document.project_id != project_id {
        return Err(SceneError::ProjectMismatch);
    }
    state.validate(allowed_assets)?;
    Ok(state)
}

/// Validate, write and flush a unique project-local temporary file, then atomically replace.
/// No source asset is changed. Caller must hold its project lock for load/apply/save.
/// A failure before rename leaves the previously published file untouched.
pub fn save_scene_state(
    project_dir: &Path,
    state: &SceneState,
    allowed_assets: &HashSet<Uuid>,
) -> Result<()> {
    state.validate(allowed_assets)?;
    let destination = project_dir.join(SCENE_FILE_NAME);
    let temporary = project_dir.join(format!(".scene_document.{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        serde_json::to_writer(&mut output, state)?;
        output.flush()?;
        output.sync_all()?;
        drop(output);
        fs::rename(&temporary, &destination)?;
        #[cfg(unix)]
        File::open(project_dir)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
