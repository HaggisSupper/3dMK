use std::path::Path;

use anyhow::Result;
use las::{Read, Reader};
use vwm_core::{CanonicalScene, GeometryOrigin};

pub fn load_las(path: &Path) -> Result<CanonicalScene> {
    let mut reader = Reader::from_path(path)?;
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut colors = Vec::<[f32; 3]>::new();
    let mut origin = None;

    for point in reader.points() {
        let p = point?;
        if ![p.x, p.y, p.z].iter().all(|value| value.is_finite()) {
            anyhow::bail!("LAS contains non-finite coordinates");
        }
        let origin = origin.get_or_insert([p.x, p.y, p.z]);
        vertices.push([
            (p.x - origin[0]) as f32,
            (p.y - origin[1]) as f32,
            (p.z - origin[2]) as f32,
        ]);

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
        datum: vwm_core::SceneDatum {
            origin: origin.unwrap_or([0.0; 3]),
            source: vwm_core::DatumSource::Metadata,
            ..Default::default()
        },
        vertices,
        colors: (colors.len() == vertex_count).then_some(colors),
        mesh: false,
        point_cloud: true,
        origin: GeometryOrigin::Measured,
        ..Default::default()
    })
}
