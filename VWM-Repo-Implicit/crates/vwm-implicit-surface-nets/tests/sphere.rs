use vwm_implicit_core::{sample_field_to_grid, GridSamplingConfig, ImplicitField, SphereField};
use vwm_implicit_surface_nets::{SurfaceNetsConfig, SurfaceNetsExtractor};

#[test]
fn extracts_mesh_from_sampled_sphere() {
    let sphere = SphereField::new([0.0, 0.0, 0.0], 1.0, 0.5).unwrap();
    let grid = sample_field_to_grid(
        &sphere,
        GridSamplingConfig {
            bounds: sphere.bounds(),
            dimensions: [25, 25, 25],
        },
    )
    .unwrap();
    let mesh = SurfaceNetsExtractor
        .extract(&grid, SurfaceNetsConfig::default())
        .unwrap();
    assert!(!mesh.positions.is_empty());
    assert!(!mesh.triangles.is_empty());
    assert_eq!(mesh.positions.len(), mesh.normals.len());
}
