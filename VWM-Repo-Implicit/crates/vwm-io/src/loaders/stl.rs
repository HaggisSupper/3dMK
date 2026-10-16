use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use vwm_core::{CanonicalScene, GeometryOrigin};

pub fn load_stl(path: &Path) -> Result<CanonicalScene> {
    let mut file = File::open(path).with_context(|| "failed to open STL file")?;
    let mesh = stl_io::read_stl(&mut file).with_context(|| "failed to parse STL file")?;

    let mut vertices = Vec::<[f32; 3]>::new();
    let mut indices = Vec::<[u32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();

    for face in mesh.faces {
        let base = vertices.len() as u32;
        for idx in face.vertices {
            let v = mesh.vertices[idx];
            vertices.push([v[0], v[1], v[2]]);
        }
        indices.push([base, base + 1, base + 2]);
        let normal = [face.normal[0], face.normal[1], face.normal[2]];
        normals.extend([normal, normal, normal]);
    }

    Ok(CanonicalScene {
        vertices,
        normals: Some(normals),
        indices: Some(indices),
        mesh: true,
        point_cloud: false,
        origin: GeometryOrigin::Measured,
        ..Default::default()
    })
}
