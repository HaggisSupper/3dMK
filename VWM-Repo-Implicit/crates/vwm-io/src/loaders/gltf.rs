use std::path::Path;

use anyhow::Result;
use gltf::import;
use vwm_core::{CanonicalScene, GeometryOrigin};

pub fn load_gltf(path: &Path) -> Result<CanonicalScene> {
    let (doc, buffers, _) = import(path)?;

    let mut vertices = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut texcoords = Vec::<[f32; 2]>::new();
    let mut indices = Vec::<[u32; 3]>::new();

    for mesh in doc.meshes() {
        for primitive in mesh.primitives() {
            let base_vertex = vertices.len() as u32;
            let reader = primitive.reader(|b| Some(&buffers[b.index()]));

            if let Some(pos) = reader.read_positions() {
                vertices.extend(pos.map(|p| [p[0], p[1], p[2]]));
            }

            if let Some(norm) = reader.read_normals() {
                normals.extend(norm.map(|n| [n[0], n[1], n[2]]));
            }

            if let Some(uv) = reader.read_tex_coords(0) {
                texcoords.extend(uv.into_f32().map(|t| [t[0], t[1]]));
            }

            if let Some(idx) = reader.read_indices() {
                let raw: Vec<u32> = idx.into_u32().collect();
                for tri in raw.chunks_exact(3) {
                    indices.push([
                        base_vertex + tri[0],
                        base_vertex + tri[1],
                        base_vertex + tri[2],
                    ]);
                }
            }
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
