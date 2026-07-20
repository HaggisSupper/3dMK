use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use ply_rs::parser::Parser;
use ply_rs::ply::{DefaultElement, Property};
use vwm_core::{CanonicalScene, GeometryOrigin};

fn as_f32(prop: &Property) -> Option<f32> {
    match prop {
        Property::Float(v) => Some(*v),
        Property::Double(v) => Some(*v as f32),
        Property::Int(v) => Some(*v as f32),
        Property::UInt(v) => Some(*v as f32),
        Property::Short(v) => Some(*v as f32),
        Property::UShort(v) => Some(*v as f32),
        Property::Char(v) => Some(*v as f32),
        Property::UChar(v) => Some(*v as f32),
        _ => None,
    }
}

pub fn load_ply(path: &Path) -> Result<CanonicalScene> {
    let mut file = File::open(path).with_context(|| "failed to open PLY file")?;
    let parser = Parser::<DefaultElement>::new();
    let ply = parser
        .read_ply(&mut file)
        .with_context(|| "failed to parse PLY file")?;

    let mut vertices = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut colors = Vec::<[f32; 3]>::new();
    let mut indices = Vec::<[u32; 3]>::new();

    if let Some(vertex_list) = ply.payload.get("vertex") {
        for v in vertex_list {
            let x = v.get("x").and_then(as_f32).unwrap_or(0.0);
            let y = v.get("y").and_then(as_f32).unwrap_or(0.0);
            let z = v.get("z").and_then(as_f32).unwrap_or(0.0);
            vertices.push([x, y, z]);

            if let (Some(nx), Some(ny), Some(nz)) = (
                v.get("nx").and_then(as_f32),
                v.get("ny").and_then(as_f32),
                v.get("nz").and_then(as_f32),
            ) {
                normals.push([nx, ny, nz]);
            }

            if let (Some(r), Some(g), Some(b)) = (
                v.get("red").and_then(as_f32),
                v.get("green").and_then(as_f32),
                v.get("blue").and_then(as_f32),
            ) {
                colors.push([r / 255.0, g / 255.0, b / 255.0]);
            }
        }
    }

    if let Some(face_list) = ply.payload.get("face") {
        for f in face_list {
            let Some(property) = f.get("vertex_indices") else {
                continue;
            };
            let triangle = match property {
                Property::ListChar(list) if list.len() == 3 => {
                    Some([list[0] as u32, list[1] as u32, list[2] as u32])
                }
                Property::ListUChar(list) if list.len() == 3 => {
                    Some([list[0] as u32, list[1] as u32, list[2] as u32])
                }
                Property::ListShort(list) if list.len() == 3 => {
                    Some([list[0] as u32, list[1] as u32, list[2] as u32])
                }
                Property::ListUShort(list) if list.len() == 3 => {
                    Some([list[0] as u32, list[1] as u32, list[2] as u32])
                }
                Property::ListInt(list) if list.len() == 3 => {
                    Some([list[0] as u32, list[1] as u32, list[2] as u32])
                }
                Property::ListUInt(list) if list.len() == 3 => Some([list[0], list[1], list[2]]),
                _ => None,
            };
            if let Some(triangle) = triangle {
                indices.push(triangle);
            }
        }
    }

    let is_mesh = !indices.is_empty();
    let vertex_count = vertices.len();
    Ok(CanonicalScene {
        vertices,
        normals: (normals.len() == vertex_count).then_some(normals),
        indices: is_mesh.then_some(indices),
        colors: (colors.len() == vertex_count).then_some(colors),
        mesh: is_mesh,
        point_cloud: !is_mesh,
        origin: GeometryOrigin::Measured,
        ..Default::default()
    })
}
