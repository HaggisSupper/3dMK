use vwm_implicit_core::{Aabb3, ImplicitError, OrientedPointSet};

#[test]
fn rejects_inverted_bounds() {
    let error = Aabb3::new([1.0, 0.0, 0.0], [0.0, 1.0, 1.0]).unwrap_err();
    assert!(matches!(error, ImplicitError::InvalidBounds));
}

#[test]
fn rejects_mismatched_points_and_normals() {
    let error = OrientedPointSet::new(vec![[0.0, 0.0, 0.0]], vec![]).unwrap_err();
    assert!(matches!(
        error,
        ImplicitError::PointNormalCountMismatch { .. }
    ));
}

#[test]
fn normalizes_valid_normals() {
    let points = OrientedPointSet::new(vec![[0.0, 0.0, 0.0]], vec![[0.0, 3.0, 4.0]]).unwrap();
    let normal = points.normalized_normals().unwrap()[0];
    assert!((normal[1] - 0.6).abs() < 1.0e-12);
    assert!((normal[2] - 0.8).abs() < 1.0e-12);
}

#[test]
fn builds_oriented_points_from_triangle_mesh_without_imported_normals() {
    use vwm_core::CanonicalScene;

    let scene = CanonicalScene {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: Some(vec![[0, 1, 2]]),
        mesh: true,
        ..CanonicalScene::default()
    };
    let points = OrientedPointSet::from_canonical_scene(&scene).unwrap();
    assert_eq!(points.points.len(), 3);
    assert_eq!(points.normals.len(), 3);
    assert_eq!(points.source_ids.as_ref().unwrap(), &vec![0, 1, 2]);
}
