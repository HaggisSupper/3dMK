use vwm_core::{CanonicalScene, GeometryOrigin};
use vwm_perception::{
    crop_masked_rgba, slice_scene_by_mask, BinaryMask, ImageFrame, ProjectionMap, SliceConfig,
    INVALID_ELEMENT_ID,
};

#[test]
fn binary_mask_reports_tight_bounds_and_crop_preserves_alpha() {
    let mask = BinaryMask::from_predicate(4, 3, |x, y| x >= 1 && x <= 2 && y >= 1).unwrap();
    let bounds = mask.bounding_box().unwrap();
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (1, 1, 2, 2)
    );

    let frame = ImageFrame::solid_rgba("synthetic", 4, 3, [10, 20, 30, 255]).unwrap();
    let crop = crop_masked_rgba(&frame, &mask, 0).unwrap();
    assert_eq!((crop.width, crop.height), (2, 2));
    assert!(crop.rgba8.chunks_exact(4).all(|p| p[3] == 255));
}

#[test]
fn slicing_selects_only_faces_supported_by_the_mask() {
    let scene = CanonicalScene {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
        ],
        indices: Some(vec![[0, 1, 2], [3, 4, 5]]),
        mesh: true,
        origin: GeometryOrigin::Measured,
        ..Default::default()
    };

    let projection = ProjectionMap {
        width: 4,
        height: 2,
        face_ids: Some(vec![0, 0, 1, 1, 0, 0, 1, 1]),
        point_ids: None,
        depth_m: None,
    };
    let mask = BinaryMask::from_predicate(4, 2, |x, _| x < 2).unwrap();

    let slice = slice_scene_by_mask(
        &scene,
        &projection,
        &mask,
        SliceConfig {
            minimum_visible_pixels: 1,
            minimum_mask_coverage: 0.75,
        },
    )
    .unwrap();

    assert_eq!(slice.scene.indices.as_ref().unwrap().len(), 1);
    assert_eq!(slice.scene.vertices.len(), 3);
    assert_eq!(slice.source_face_ids, vec![0]);
    assert!(slice.source_point_ids.is_empty());
}

#[test]
fn invalid_projection_ids_are_ignored() {
    let scene = CanonicalScene {
        vertices: vec![[0.0, 0.0, 0.0]],
        point_cloud: true,
        origin: GeometryOrigin::Measured,
        ..Default::default()
    };
    let projection = ProjectionMap {
        width: 2,
        height: 1,
        face_ids: None,
        point_ids: Some(vec![0, INVALID_ELEMENT_ID]),
        depth_m: None,
    };
    let mask = BinaryMask::filled(2, 1, true).unwrap();

    let slice = slice_scene_by_mask(&scene, &projection, &mask, SliceConfig::default()).unwrap();
    assert_eq!(slice.scene.vertices.len(), 1);
    assert_eq!(slice.source_point_ids, vec![0]);
}
