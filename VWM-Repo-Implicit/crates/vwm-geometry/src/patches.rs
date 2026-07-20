use anyhow::{anyhow, Result};
use nalgebra::{Point3, UnitVector3};
use std::collections::VecDeque;
use vwm_core::{CanonicalScene, Patch};

use crate::adjacency::MeshAdjacency;
use crate::math::{angle_between_degrees, arr3, centroid, p3};

#[derive(Debug, Clone, Copy)]
pub struct PatchConfig {
    pub normal_angle_threshold_degrees: f64,
    pub min_faces_per_patch: usize,
}

pub fn segment_patches(
    scene: &CanonicalScene,
    adjacency: &MeshAdjacency,
    face_normals: &[UnitVector3<f64>],
    cfg: PatchConfig,
) -> Result<Vec<Patch>> {
    let tris = scene
        .indices
        .as_ref()
        .ok_or_else(|| anyhow!("scene has no triangle indices"))?;
    let mut visited = vec![false; tris.len()];
    let mut patches = Vec::<Patch>::new();

    for seed in 0..tris.len() {
        if visited[seed] {
            continue;
        }

        let mut q = VecDeque::new();
        q.push_back(seed);
        visited[seed] = true;

        let seed_n = face_normals[seed];
        let mut faces = Vec::new();

        while let Some(f) = q.pop_front() {
            faces.push(f);

            for &nb in &adjacency.face_neighbours[f] {
                if visited[nb] {
                    continue;
                }
                let angle = angle_between_degrees(seed_n, face_normals[nb]);
                if angle <= cfg.normal_angle_threshold_degrees {
                    visited[nb] = true;
                    q.push_back(nb);
                }
            }
        }

        if faces.len() < cfg.min_faces_per_patch {
            continue;
        }

        let mut vertex_seen = vec![false; scene.vertices.len()];
        let mut vertex_indices = Vec::<usize>::new();
        for &fi in &faces {
            let tri = tris[fi];
            for &vid in &tri {
                let idx = vid as usize;
                if !vertex_seen[idx] {
                    vertex_seen[idx] = true;
                    vertex_indices.push(idx);
                }
            }
        }

        let points: Vec<Point3<f64>> = vertex_indices
            .iter()
            .map(|&i| p3(scene.vertices[i]))
            .collect();
        let c = centroid(&points);

        let mut acc = nalgebra::Vector3::zeros();
        for &fi in &faces {
            acc += face_normals[fi].into_inner();
        }
        let mean = nalgebra::Unit::try_new(acc, 1e-12)
            .unwrap_or_else(|| nalgebra::Unit::new_normalize(nalgebra::Vector3::y()));

        patches.push(Patch {
            id: patches.len(),
            face_indices: faces,
            vertex_indices,
            centroid: [c.x, c.y, c.z],
            mean_normal: arr3(mean.into_inner()),
            planarity: 1.0,
        });
    }

    Ok(patches)
}
