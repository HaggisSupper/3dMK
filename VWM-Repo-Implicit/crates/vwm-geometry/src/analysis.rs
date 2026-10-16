use anyhow::{anyhow, Result};
use vwm_core::{CanonicalScene, GeometryAnalysis, VwmError};

use crate::adjacency::build_mesh_adjacency;
use crate::intersections::compute_plane_intersections;
use crate::normals::compute_face_normals;
use crate::patches::{segment_patches, PatchConfig};
use crate::planes::{fit_planes, PlaneConfig};
use crate::relations::{build_plane_relations, RelationConfig};

#[derive(Debug, Clone, Copy)]
pub struct GeometryConfig {
    pub patch_normal_angle_threshold_degrees: f64,
    pub min_faces_per_patch: usize,
    pub plane_min_vertices: usize,
    pub parallel_angle_threshold_degrees: f64,
    pub perpendicular_angle_threshold_degrees: f64,
    pub adjacency_centroid_distance: f64,
    pub coplanar_offset_threshold: f64,
}

impl Default for GeometryConfig {
    fn default() -> Self {
        Self {
            patch_normal_angle_threshold_degrees: 12.0,
            min_faces_per_patch: 2,
            plane_min_vertices: 3,
            parallel_angle_threshold_degrees: 8.0,
            perpendicular_angle_threshold_degrees: 10.0,
            adjacency_centroid_distance: 1.25,
            coplanar_offset_threshold: 0.05,
        }
    }
}

pub fn analyze_scene(scene: &CanonicalScene, cfg: GeometryConfig) -> Result<GeometryAnalysis> {
    if scene.vertices.is_empty() {
        return Err(VwmError::EmptyScene.into());
    }
    if scene.indices.is_none() {
        return Err(VwmError::MissingTriangles.into());
    }

    let adjacency = build_mesh_adjacency(scene)?;
    let face_normals = compute_face_normals(scene)?;
    let patches = segment_patches(
        scene,
        &adjacency,
        &face_normals,
        PatchConfig {
            normal_angle_threshold_degrees: cfg.patch_normal_angle_threshold_degrees,
            min_faces_per_patch: cfg.min_faces_per_patch,
        },
    )?;

    if patches.is_empty() {
        return Err(anyhow!("no patches survived segmentation"));
    }

    let planes = fit_planes(
        scene,
        &patches,
        PlaneConfig {
            min_vertices: cfg.plane_min_vertices,
        },
    )?;

    let relations = build_plane_relations(
        &planes,
        RelationConfig {
            parallel_angle_threshold_degrees: cfg.parallel_angle_threshold_degrees,
            perpendicular_angle_threshold_degrees: cfg.perpendicular_angle_threshold_degrees,
            adjacency_centroid_distance: cfg.adjacency_centroid_distance,
            coplanar_offset_threshold: cfg.coplanar_offset_threshold,
        },
    )?;

    let intersections = compute_plane_intersections(&planes)?;

    Ok(GeometryAnalysis {
        patches,
        planes,
        relations,
        intersections,
    })
}
