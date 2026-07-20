use approx::assert_relative_eq;
use vwm_core::{CanonicalScene, GeometryOrigin, LengthUnit, Transform3D};
use vwm_geometry::{analyze_scene, GeometryConfig};

fn rect_xy(z: f32, x0: f32, x1: f32, y0: f32, y1: f32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let base = vec![[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]];
    let idx = vec![[0, 1, 2], [0, 2, 3]];
    (base, idx)
}

fn rect_yz(x: f32, y0: f32, y1: f32, z0: f32, z1: f32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let base = vec![[x, y0, z0], [x, y1, z0], [x, y1, z1], [x, y0, z1]];
    let idx = vec![[0, 1, 2], [0, 2, 3]];
    (base, idx)
}

#[test]
fn synthetic_room_detects_major_planes() {
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut indices = Vec::<[u32; 3]>::new();

    let parts = vec![
        rect_xy(0.0, 0.0, 4.0, 0.0, 3.0), // floor
        rect_xy(2.5, 0.0, 4.0, 0.0, 3.0), // ceiling
        rect_yz(0.0, 0.0, 3.0, 0.0, 2.5), // wall x=0
        rect_yz(4.0, 0.0, 3.0, 0.0, 2.5), // wall x=4
    ];

    for (verts, tris) in parts {
        let base = vertices.len() as u32;
        vertices.extend(verts);
        for tri in tris {
            indices.push([base + tri[0], base + tri[1], base + tri[2]]);
        }
    }

    let scene = CanonicalScene {
        vertices,
        indices: Some(indices),
        mesh: true,
        point_cloud: false,
        units: LengthUnit::Meters,
        transform: Transform3D::default(),
        origin: GeometryOrigin::Measured,
        ..Default::default()
    };

    let analysis = analyze_scene(&scene, GeometryConfig::default()).unwrap();

    assert!(
        analysis.patches.len() >= 4,
        "expected >= 4 patches, got {}",
        analysis.patches.len()
    );
    assert!(
        analysis.planes.len() >= 4,
        "expected >= 4 planes, got {}",
        analysis.planes.len()
    );

    let mut found_horizontal = 0usize;
    let mut found_vertical = 0usize;

    for p in &analysis.planes {
        let ny = p.normal[1].abs();
        if ny > 0.95 {
            found_horizontal += 1;
        } else {
            found_vertical += 1;
        }
    }

    assert!(
        found_horizontal >= 2,
        "expected at least two horizontal planes"
    );
    assert!(found_vertical >= 2, "expected at least two vertical planes");

    let rmse_max = analysis
        .planes
        .iter()
        .map(|p| p.residual_rmse)
        .fold(0.0_f64, f64::max);

    assert_relative_eq!(rmse_max, 0.0, epsilon = 1.0e-8);
}
