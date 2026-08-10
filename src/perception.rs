use std::sync::Arc;

use anyhow::{Context, Result};
use vwm_perception::{
    ColorRegionConfig, ColorRegionSegmenter, ImageFrame, PerceptionBatch, PerceptionInput,
    PerceptionPipeline, PerceptionPipelineConfig, ShapeClassifier,
};

/// Run the canonical VWM perception contract locally. This deterministic path is the fallback
/// until a packaged ONNX manifest is supplied; model-backed inference uses the same contracts.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VwmPerceptionOptions {
    pub background_rgba: [u8; 4],
    pub channel_tolerance: u8,
    pub minimum_pixels: usize,
    pub eight_connected: bool,
    pub crop_padding_pixels: u32,
}

impl Default for VwmPerceptionOptions {
    fn default() -> Self {
        let segmentation = ColorRegionConfig::default();
        let pipeline = PerceptionPipelineConfig::default();
        Self {
            background_rgba: segmentation.background_rgba,
            channel_tolerance: segmentation.channel_tolerance,
            minimum_pixels: segmentation.minimum_pixels,
            eight_connected: segmentation.eight_connected,
            crop_padding_pixels: pipeline.crop_padding_pixels,
        }
    }
}

impl VwmPerceptionOptions {
    fn validate(self) -> Result<Self> {
        if self.minimum_pixels == 0 || self.minimum_pixels > 10_000_000 {
            anyhow::bail!("VWM minimum_pixels must be between 1 and 10000000");
        }
        if self.crop_padding_pixels > 512 {
            anyhow::bail!("VWM crop_padding_pixels must not exceed 512");
        }
        Ok(self)
    }
}

pub fn recognize_image_with_options(
    source_id: &str,
    bytes: &[u8],
    options: VwmPerceptionOptions,
) -> Result<PerceptionBatch> {
    let options = options.validate()?;
    let rgba = image::load_from_memory(bytes)
        .context("VWM perception could not decode the supplied image")?
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    let frame = ImageFrame::new(source_id, width, height, rgba.into_raw())
        .context("VWM perception received an invalid image frame")?;
    let pipeline = PerceptionPipeline::new(
        Arc::new(ColorRegionSegmenter::new(ColorRegionConfig {
            background_rgba: options.background_rgba,
            channel_tolerance: options.channel_tolerance,
            minimum_pixels: options.minimum_pixels,
            eight_connected: options.eight_connected,
        })),
        Arc::new(ShapeClassifier),
        None,
        PerceptionPipelineConfig {
            crop_padding_pixels: options.crop_padding_pixels,
            ..PerceptionPipelineConfig::default()
        },
    );
    Ok(pipeline.process(&PerceptionInput {
        frame,
        scene: None,
        candidate_labels: Vec::new(),
    })?)
}

pub fn recognize_image(source_id: &str, bytes: &[u8]) -> Result<PerceptionBatch> {
    recognize_image_with_options(source_id, bytes, VwmPerceptionOptions::default())
}
