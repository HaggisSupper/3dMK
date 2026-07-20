use std::path::Path;

use anyhow::Result;
use vwm_core::{CanonicalScene, GeometryOrigin};

pub fn load_obj(path: &Path) -> Result<CanonicalScene> {
    let (models, _) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )?;

    let mut vertices = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut texcoords = Vec::<[f32; 2]>::new();
    let mut indices = Vec::<[u32; 3]>::new();

    for model in models {
        let mesh = model.mesh;
        let base_vertex = vertices.len() as u32;

        for v in mesh.positions.chunks_exact(3) {
            vertices.push([v[0], v[1], v[2]]);
        }

        for n in mesh.normals.chunks_exact(3) {
            normals.push([n[0], n[1], n[2]]);
        }

        for t in mesh.texcoords.chunks_exact(2) {
            texcoords.push([t[0], t[1]]);
        }

        for tri in mesh.indices.chunks_exact(3) {
            indices.push([
                base_vertex + tri[0],
                base_vertex + tri[1],
                base_vertex + tri[2],
            ]);
        }
    }

    Ok(CanonicalScene {
        vertices,
        normals: if normals.is_empty() {
            None
        } else {
            Some(normals)
        },
        indices: if indices.is_empty() {
            None
        } else {
            Some(indices)
        },
        uvs: if texcoords.is_empty() {
            None
        } else {
            Some(texcoords)
        },
        mesh: true,
        point_cloud: false,
        origin: GeometryOrigin::Measured,
        ..Default::default()
    })
}
