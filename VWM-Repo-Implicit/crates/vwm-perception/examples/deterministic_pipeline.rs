use std::sync::Arc;

use vwm_perception::{
    ColorRegionConfig, ColorRegionSegmenter, ImageFrame, PerceptionInput, PerceptionPipeline,
    PerceptionPipelineConfig, ShapeClassifier,
};

fn main() -> vwm_perception::Result<()> {
    let mut frame = ImageFrame::solid_rgba("synthetic", 64, 48, [0, 0, 0, 0])?;
    for y in 12..36 {
        for x in 18..46 {
            let offset = ((y * frame.width + x) * 4) as usize;
            frame.rgba8[offset..offset + 4].copy_from_slice(&[160, 120, 40, 255]);
        }
    }

    let pipeline = PerceptionPipeline::new(
        Arc::new(ColorRegionSegmenter::new(ColorRegionConfig {
            minimum_pixels: 16,
            ..Default::default()
        })),
        Arc::new(ShapeClassifier),
        None,
        PerceptionPipelineConfig::default(),
    );

    let batch = pipeline.process(&PerceptionInput {
        frame,
        scene: None,
        candidate_labels: Vec::new(),
    })?;

    println!("{} object(s)", batch.objects.len());
    for object in batch.objects {
        println!(
            "id={} label={} confidence={:.3} pixels={}",
            object.proposal.id,
            object.classification.label,
            object.classification.confidence,
            object.features.visible_pixel_count,
        );
    }
    Ok(())
}
