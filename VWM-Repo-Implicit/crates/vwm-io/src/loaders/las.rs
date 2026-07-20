use std::path::Path;

use anyhow::Result;
use las::{Read, Reader};
use vwm_core::{CanonicalScene, GeometryOrigin};

pub fn load_las(path: &Path) -> Result<CanonicalScene> {
    let mut reader = Reader::from_path(path)?;
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut colors = Vec::<[f32; 3]>::new();

    for point in reader.points() {
        let p = point?;
        vertices.push([p.x as f32, p.y as f32, p.z as f32]);

        if let Some(c) = p.color {
            colors.push([
                c.red as f32 / 65535.0,
                c.green as f32 / 65535.0,
                c.blue as f32 / 65535.0,
            ]);
        }
    }

    let vertex_count = vertices.len();
    Ok(CanonicalScene {
        vertices,
        colors: (colors.len() == vertex_count).then_some(colors),
        mesh: false,
        point_cloud: true,
        origin: GeometryOrigin::Measured,
        ..Default::default()
    })
}
