use vwm_perception::{
    parse_vlm_json, ColorRegionConfig, ColorRegionSegmenter, ImageFrame, InstanceSegmenter,
};

#[test]
fn vlm_json_parser_accepts_a_wrapped_json_object() {
    let result = parse_vlm_json(
        "classification follows: {\"label\":\"cabinet\",\"confidence\":0.84,\"alternatives\":[],\"rationale\":\"rectangular enclosure\"}",
    )
    .unwrap();

    assert_eq!(result.label, "cabinet");
    assert!((result.confidence - 0.84).abs() < f32::EPSILON);
}

#[test]
fn vlm_json_parser_rejects_out_of_range_confidence() {
    let error = parse_vlm_json("{\"label\":\"cabinet\",\"confidence\":1.4}").unwrap_err();
    assert!(error.to_string().contains("confidence"));
}

#[test]
fn deterministic_segmenter_extracts_disconnected_foreground_objects() {
    let mut frame = ImageFrame::solid_rgba("regions", 8, 4, [0, 0, 0, 0]).unwrap();
    for &(x, y) in &[(1u32, 1u32), (1, 2), (6, 1), (6, 2)] {
        let offset = ((y * frame.width + x) * 4) as usize;
        frame.rgba8[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
    }

    let segmenter = ColorRegionSegmenter::new(ColorRegionConfig {
        channel_tolerance: 0,
        minimum_pixels: 2,
        eight_connected: false,
        ..Default::default()
    });
    let objects = segmenter.segment(&frame).unwrap();

    assert_eq!(objects.len(), 2);
    assert!(objects
        .iter()
        .all(|object| object.mask.foreground_count() == 2));
}
