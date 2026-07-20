use std::io::Cursor;

use image::{imageops, DynamicImage, ImageBuffer, ImageFormat, Rgba};

use crate::contracts::{checked_pixel_count, BoundingBox2D, ImageFrame, ImagePatch};
use crate::{BinaryMask, PerceptionError, Result};

#[derive(Debug, Clone)]
pub struct LetterboxedImage {
    pub tensor_nchw: Vec<f32>,
    pub target_width: u32,
    pub target_height: u32,
    pub resized_width: u32,
    pub resized_height: u32,
    pub pad_left: u32,
    pub pad_top: u32,
    pub scale: f32,
}

pub fn crop_masked_rgba(frame: &ImageFrame, mask: &BinaryMask, padding: u32) -> Result<ImagePatch> {
    frame.validate()?;
    mask.validate()?;
    if frame.width != mask.width || frame.height != mask.height {
        return Err(PerceptionError::ProjectionDimensionMismatch);
    }

    let bounds = mask.bounding_box().ok_or(PerceptionError::EmptyMask)?;
    let x0 = bounds.x.saturating_sub(padding);
    let y0 = bounds.y.saturating_sub(padding);
    let x1 = bounds
        .right_exclusive()
        .saturating_add(padding)
        .min(frame.width);
    let y1 = bounds
        .bottom_exclusive()
        .saturating_add(padding)
        .min(frame.height);
    let out_bounds = BoundingBox2D {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    };

    let mut rgba8 =
        Vec::with_capacity(checked_pixel_count(out_bounds.width, out_bounds.height)? * 4);
    for y in y0..y1 {
        for x in x0..x1 {
            let source = ((y * frame.width + x) * 4) as usize;
            let mut px = [
                frame.rgba8[source],
                frame.rgba8[source + 1],
                frame.rgba8[source + 2],
                frame.rgba8[source + 3],
            ];
            if !mask.contains(x, y) {
                px[3] = 0;
            }
            rgba8.extend_from_slice(&px);
        }
    }

    Ok(ImagePatch {
        width: out_bounds.width,
        height: out_bounds.height,
        rgba8,
        source_bounds: out_bounds,
    })
}

pub fn patch_to_png(patch: &ImagePatch) -> Result<Vec<u8>> {
    patch.validate()?;
    let image =
        ImageBuffer::<Rgba<u8>, _>::from_raw(patch.width, patch.height, patch.rgba8.clone())
            .ok_or_else(|| PerceptionError::ImageProcessing("failed to build RGBA image".into()))?;
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| PerceptionError::ImageProcessing(e.to_string()))?;
    Ok(cursor.into_inner())
}

pub fn letterbox_rgb_nchw(
    rgba8: &[u8],
    width: u32,
    height: u32,
    target_width: u32,
    target_height: u32,
    mean: [f32; 3],
    std: [f32; 3],
) -> Result<LetterboxedImage> {
    let expected = checked_pixel_count(width, height)? * 4;
    if rgba8.len() != expected {
        return Err(PerceptionError::InvalidImageBuffer {
            expected,
            actual: rgba8.len(),
        });
    }
    if target_width == 0 || target_height == 0 || width == 0 || height == 0 {
        return Err(PerceptionError::ImageProcessing(
            "letterbox dimensions must be non-zero".into(),
        ));
    }
    if std.iter().any(|v| !v.is_finite() || *v == 0.0) {
        return Err(PerceptionError::ModelConfiguration(
            "normalization standard deviations must be finite and non-zero".into(),
        ));
    }

    let source = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba8.to_vec())
        .ok_or_else(|| PerceptionError::ImageProcessing("failed to build source image".into()))?;
    let scale = (target_width as f32 / width as f32).min(target_height as f32 / height as f32);
    let resized_width = ((width as f32 * scale).round() as u32).clamp(1, target_width);
    let resized_height = ((height as f32 * scale).round() as u32).clamp(1, target_height);
    let resized = imageops::resize(
        &source,
        resized_width,
        resized_height,
        imageops::FilterType::Triangle,
    );
    let pad_left = (target_width - resized_width) / 2;
    let pad_top = (target_height - resized_height) / 2;

    let plane = checked_pixel_count(target_width, target_height)?;
    let mut tensor_nchw = vec![0.0f32; plane * 3];
    for y in 0..resized_height {
        for x in 0..resized_width {
            let px = resized.get_pixel(x, y).0;
            let tx = x + pad_left;
            let ty = y + pad_top;
            let offset = (ty * target_width + tx) as usize;
            for c in 0..3 {
                tensor_nchw[c * plane + offset] = (px[c] as f32 / 255.0 - mean[c]) / std[c];
            }
        }
    }

    Ok(LetterboxedImage {
        tensor_nchw,
        target_width,
        target_height,
        resized_width,
        resized_height,
        pad_left,
        pad_top,
        scale,
    })
}

pub fn unletterbox_box(
    bbox: [f32; 4],
    transform: &LetterboxedImage,
    source_width: u32,
    source_height: u32,
) -> BoundingBox2D {
    let x1 =
        ((bbox[0] - transform.pad_left as f32) / transform.scale).clamp(0.0, source_width as f32);
    let y1 =
        ((bbox[1] - transform.pad_top as f32) / transform.scale).clamp(0.0, source_height as f32);
    let x2 =
        ((bbox[2] - transform.pad_left as f32) / transform.scale).clamp(0.0, source_width as f32);
    let y2 =
        ((bbox[3] - transform.pad_top as f32) / transform.scale).clamp(0.0, source_height as f32);

    let left = x1.floor() as u32;
    let top = y1.floor() as u32;
    let right = x2.ceil() as u32;
    let bottom = y2.ceil() as u32;
    BoundingBox2D {
        x: left,
        y: top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    }
}

pub fn unletterbox_mask(
    model_mask: &[f32],
    model_width: u32,
    model_height: u32,
    transform: &LetterboxedImage,
    source_width: u32,
    source_height: u32,
    threshold: f32,
) -> Result<BinaryMask> {
    let expected = checked_pixel_count(model_width, model_height)?;
    if model_mask.len() != expected {
        return Err(PerceptionError::InvalidMaskBuffer {
            expected,
            actual: model_mask.len(),
        });
    }

    let mut output = Vec::with_capacity(checked_pixel_count(source_width, source_height)?);
    for sy in 0..source_height {
        for sx in 0..source_width {
            let mx = (sx as f32 * transform.scale + transform.pad_left as f32)
                .round()
                .clamp(0.0, model_width.saturating_sub(1) as f32) as u32;
            let my = (sy as f32 * transform.scale + transform.pad_top as f32)
                .round()
                .clamp(0.0, model_height.saturating_sub(1) as f32) as u32;
            output.push(u8::from(
                model_mask[(my * model_width + mx) as usize] >= threshold,
            ));
        }
    }
    BinaryMask::new(source_width, source_height, output)
}
