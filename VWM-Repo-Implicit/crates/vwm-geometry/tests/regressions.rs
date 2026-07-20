use vwm_core::{CanonicalScene, PlanePrimitive, PlaneRelationKind};
use vwm_geometry::{
    analyze_scene,
    relations::{build_plane_relations, RelationConfig},
    GeometryConfig,
};

fn plane(id: usize, normal: [f64; 3], offset: f64) -> PlanePrimitive {
    PlanePrimitive {
        id,
        patch_id: id,
        normal,
        offset,
        support_vertices: vec![0, 1, 2],
        residual_rmse: 0.0,
        support_area: 1.0,
        centroid: [2.0, 0.0, 0.0],
        boundary_2d: vec![],
        boundary_basis_origin: [0.0; 3],
        boundary_basis_u: [0.0; 3],
        boundary_basis_v: [0.0; 3],
    }
}

#[test]
fn opposite_plane_normals_can_still_be_coplanar() {
    let relations = build_plane_relations(
        &[
            plane(0, [1.0, 0.0, 0.0], -2.0),
            plane(1, [-1.0, 0.0, 0.0], 2.0),
        ],
        RelationConfig {
            parallel_angle_threshold_degrees: 8.0,
            perpendicular_angle_threshold_degrees: 10.0,
            adjacency_centroid_distance: 1.25,
            coplanar_offset_threshold: 0.05,
        },
    )
    .unwrap();

    assert_eq!(relations[0].kind, PlaneRelationKind::Coplanar);
    assert_eq!(relations[0].offset_difference, 0.0);
}

#[test]
fn invalid_triangle_indices_return_an_error_instead_of_panicking() {
    let scene = CanonicalScene {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: Some(vec![[0, 1, 3]]),
        mesh: true,
        ..Default::default()
    };

    assert!(analyze_scene(&scene, GeometryConfig::default()).is_err());
}
