use serde::{Deserialize, Serialize};

use crate::datum::{LengthUnit, SceneDatum};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Transform3D {
    pub translation: [f64; 3],
    pub rotation_xyzw: [f64; 4],
    pub scale: [f64; 3],
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub enum GeometryOrigin {
    #[default]
    Measured,
    Derived,
    Regularized,
    Inferred,
    Generated,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanonicalScene {
    /// Original glTF JSON: hierarchy, materials, extensions and engineering extras.
    /// Immutable source bytes remain the authoritative round-trip asset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_document: Option<serde_json::Value>,
    #[serde(default)]
    pub primitives: Vec<ScenePrimitive>,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub indices: Option<Vec<[u32; 3]>>,
    pub colors: Option<Vec<[f32; 3]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub material_ids: Option<Vec<u32>>,
    pub mesh: bool,
    pub point_cloud: bool,
    pub units: LengthUnit,
    pub transform: Transform3D,
    pub datum: SceneDatum,
    pub origin: GeometryOrigin,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScenePrimitive {
    #[serde(default)]
    pub normals: Option<Vec<[f32; 3]>>,
    #[serde(default)]
    pub texcoords: std::collections::BTreeMap<u32, Vec<[f32; 2]>>,
    #[serde(default)]
    pub colors_rgba: Option<Vec<[f32; 4]>>,
    pub node_index: usize,
    pub mesh_index: usize,
    pub primitive_index: usize,
    pub vertex_start: usize,
    pub vertex_count: usize,
    pub face_start: usize,
    pub face_count: usize,
    pub material_index: Option<usize>,
    pub world_transform: [[f64; 4]; 4],
}
