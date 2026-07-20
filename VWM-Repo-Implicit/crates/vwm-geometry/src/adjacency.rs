use ahash::{AHashMap, AHashSet};
use anyhow::{anyhow, Result};
use vwm_core::CanonicalScene;

#[derive(Debug, Clone)]
pub struct MeshAdjacency {
    pub face_neighbours: Vec<Vec<usize>>,
    pub vertex_to_faces: Vec<Vec<usize>>,
}

pub fn build_mesh_adjacency(scene: &CanonicalScene) -> Result<MeshAdjacency> {
    let tris = scene
        .indices
        .as_ref()
        .ok_or_else(|| anyhow!("scene has no triangle indices"))?;
    let mut edge_to_face: AHashMap<(u32, u32), usize> = AHashMap::new();
    let mut face_neighbours: Vec<AHashSet<usize>> = vec![AHashSet::new(); tris.len()];
    let mut vertex_to_faces = vec![Vec::<usize>::new(); scene.vertices.len()];

    for (fi, tri) in tris.iter().enumerate() {
        for &vid in tri {
            let idx = vid as usize;
            if idx < vertex_to_faces.len() {
                vertex_to_faces[idx].push(fi);
            }
        }

        let edges = [
            sorted_edge(tri[0], tri[1]),
            sorted_edge(tri[1], tri[2]),
            sorted_edge(tri[2], tri[0]),
        ];

        for e in edges {
            if let Some(other) = edge_to_face.insert(e, fi) {
                face_neighbours[fi].insert(other);
                face_neighbours[other].insert(fi);
            }
        }
    }

    Ok(MeshAdjacency {
        face_neighbours: face_neighbours
            .into_iter()
            .map(|s| s.into_iter().collect())
            .collect(),
        vertex_to_faces,
    })
}

fn sorted_edge(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}
