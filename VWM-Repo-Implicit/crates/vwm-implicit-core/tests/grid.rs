use vwm_implicit_core::{
    sample_field_to_grid, Aabb3, DenseScalarGrid, GridSamplingConfig, ImplicitField, SphereField,
};

#[test]
fn trilinear_interpolation_recovers_linear_field() {
    let values = vec![0.0, 1.0, 1.0, 2.0, 1.0, 2.0, 2.0, 3.0];
    let grid =
        DenseScalarGrid::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [2, 2, 2], values, 0.0).unwrap();
    let value = grid.value([0.5, 0.5, 0.5]).unwrap();
    assert!((value - 1.5).abs() < 1.0e-6);
}

#[test]
fn samples_analytical_field_to_requested_dimensions() {
    let sphere = SphereField::new([0.0, 0.0, 0.0], 1.0, 0.5).unwrap();
    let grid = sample_field_to_grid(
        &sphere,
        GridSamplingConfig {
            bounds: Aabb3::new([-1.5; 3], [1.5; 3]).unwrap(),
            dimensions: [9, 9, 9],
        },
    )
    .unwrap();
    assert_eq!(grid.values().len(), 9 * 9 * 9);
    assert!(grid.value([0.0, 0.0, 0.0]).unwrap() < 0.0);
}
