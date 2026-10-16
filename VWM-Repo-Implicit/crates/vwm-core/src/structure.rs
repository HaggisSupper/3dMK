use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PlaneRelationKind {
    Parallel,
    Perpendicular,
    Adjacent,
    Coplanar,
    Intersecting,
    FreeAngle,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Patch {
    pub id: usize,
    pub face_indices: Vec<usize>,
    pub vertex_indices: Vec<usize>,
    pub centroid: [f64; 3],
    pub mean_normal: [f64; 3],
    pub planarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanePrimitive {
    pub id: usize,
    pub patch_id: usize,
    pub normal: [f64; 3],
    pub offset: f64,
    pub support_vertices: Vec<usize>,
    pub residual_rmse: f64,
    pub support_area: f64,
    pub centroid: [f64; 3],
    pub boundary_2d: Vec<[f64; 2]>,
    pub boundary_basis_origin: [f64; 3],
    pub boundary_basis_u: [f64; 3],
    pub boundary_basis_v: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaneRelation {
    pub source_plane_id: usize,
    pub target_plane_id: usize,
    pub kind: PlaneRelationKind,
    pub measured_angle_degrees: f64,
    pub centroid_distance: f64,
    pub offset_difference: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaneIntersection {
    pub plane_a: usize,
    pub plane_b: usize,
    pub point_on_line: [f64; 3],
    pub line_direction: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GeometryAnalysis {
    pub patches: Vec<Patch>,
    pub planes: Vec<PlanePrimitive>,
    pub relations: Vec<PlaneRelation>,
    pub intersections: Vec<PlaneIntersection>,
}
