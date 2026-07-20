use std::sync::Arc;

use anyhow::{Context, Result};
use vwm_perception::{
    ColorRegionConfig, ColorRegionSegmenter, ImageFrame, PerceptionBatch, PerceptionInput,
    PerceptionPipeline, PerceptionPipelineConfig, ShapeClassifier,
};

/// Run the canonical VWM perception contract locally. This deterministic path is the fallback
/// until a packaged ONNX manifest is supplied; model-backed inference uses the same contracts.
pub fn recognize_image(source_id: &str, bytes: &[u8]) -> Result<PerceptionBatch> {
    let rgba = image::load_from_memory(bytes)
        .context("VWM perception could not decode the supplied image")?
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    let frame = ImageFrame::new(source_id, width, height, rgba.into_raw())
        .context("VWM perception received an invalid image frame")?;
    let pipeline = PerceptionPipeline::new(
        Arc::new(ColorRegionSegmenter::new(ColorRegionConfig::default())),
        Arc::new(ShapeClassifier),
        None,
        PerceptionPipelineConfig::default(),
    );
    Ok(pipeline.process(&PerceptionInput {
        frame,
        scene: None,
        candidate_labels: Vec::new(),
    })?)
}
