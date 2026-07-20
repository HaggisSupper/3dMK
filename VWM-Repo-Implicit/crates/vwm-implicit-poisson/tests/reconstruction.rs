use std::f64::consts::PI;

use vwm_implicit_core::{ImplicitField, OrientedPointSet};
use vwm_implicit_poisson::{PoissonConfig, PoissonReconstructor};

#[test]
fn rejects_invalid_configuration() {
    let config = PoissonConfig {
        density_estimation_depth: 6,
        max_depth: 5,
        ..PoissonConfig::default()
    };
    assert!(config.validate().is_err());
}

#[test]
fn reconstructs_low_resolution_sphere_field() {
    let mut points = Vec::new();
    let mut normals = Vec::new();
    for latitude in 1..8 {
        let phi = PI * latitude as f64 / 8.0;
        for longitude in 0..16 {
            let theta = 2.0 * PI * longitude as f64 / 16.0;
            let point = [phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()];
            points.push(point);
            normals.push(point);
        }
    }
    points.push([0.0, 1.0, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    points.push([0.0, -1.0, 0.0]);
    normals.push([0.0, -1.0, 0.0]);

    let input = OrientedPointSet::new(points, normals).unwrap();
    let field = PoissonReconstructor
        .reconstruct(
            &input,
            PoissonConfig {
                screening: 1.0,
                density_estimation_depth: 3,
                max_depth: 4,
                max_relaxation_iterations: 6,
            },
        )
        .unwrap();
    let center = field.bounds().center();
    assert!(field.value(center).unwrap().is_finite());
    let mesh = field.reconstruct_mesh().unwrap();
    assert!(!mesh.positions.is_empty());
    assert!(!mesh.triangles.is_empty());
}
