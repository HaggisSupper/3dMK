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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum GeometryOrigin {
    Measured,
    Derived,
    Regularized,
    Inferred,
    Generated,
}

impl Default for GeometryOrigin {
    fn default() -> Self {
        Self::Measured
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanonicalScene {
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
