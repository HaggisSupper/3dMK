use anyhow::{bail, Context, Result};
use image::{DynamicImage, ImageFormat, RgbImage};
use serde::Serialize;
use std::path::Path;

pub const MAX_REFERENCE_IMAGES: usize = 64;
const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
const MAX_SAMPLE_PIXELS: u64 = 250_000;

#[derive(Debug)]
pub struct UploadedImage {
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImageEvidence {
    pub filename: String,
    pub width: u32,
    pub height: u32,
    pub quality: f64,
    pub usable: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImageRefinement {
    pub source_texture: String,
    pub images_inspected: usize,
    pub usable_reference_images: usize,
    pub reference_images: Vec<ImageEvidence>,
}

pub fn refine_texture(
    texture: UploadedImage,
    references: Vec<UploadedImage>,
    output_path: &Path,
) -> Result<ImageRefinement> {
    if texture.bytes.len() > MAX_IMAGE_BYTES {
        bail!("Texture image is larger than the 32 MB processing limit");
    }
    if references.len() > MAX_REFERENCE_IMAGES {
        bail!("A package can refine from at most {MAX_REFERENCE_IMAGES} reference images at once");
    }

    let texture_image = decode_image(&texture)?;
    let mut reference_images = Vec::with_capacity(references.len());
    for reference in references {
        if reference.bytes.len() > MAX_IMAGE_BYTES {
            continue;
        }
        if let Ok(image) = decode_image(&reference) {
            reference_images.push(score_image(&reference.filename, &image));
        }
    }

    let usable_references = reference_images
        .iter()
        .filter(|image| image.usable)
        .collect::<Vec<_>>();
    let reference_quality = usable_references
        .iter()
        .map(|image| image.quality)
        .sum::<f64>()
        / usable_references.len().max(1) as f64;
    let enhanced = enhance_contrast(&texture_image, 1.04 + reference_quality * 0.08);
    let output_parent = output_path
        .parent()
        .context("Refined texture needs an output directory")?;
    std::fs::create_dir_all(output_parent)?;
    enhanced.save_with_format(output_path, ImageFormat::Png)?;

    Ok(ImageRefinement {
        source_texture: texture.filename,
        images_inspected: reference_images.len() + 1,
        usable_reference_images: usable_references.len(),
        reference_images,
    })
}

fn decode_image(upload: &UploadedImage) -> Result<DynamicImage> {
    image::load_from_memory(&upload.bytes)
        .with_context(|| format!("{} is not a supported PNG or JPEG image", upload.filename))
}

fn score_image(filename: &str, image: &DynamicImage) -> ImageEvidence {
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    let sample_step = (((width as u64 * height as u64) / MAX_SAMPLE_PIXELS).max(1) as f64)
        .sqrt()
        .ceil() as u32;
    let mut count = 0u64;
    let mut sum = 0f64;
    let mut sum_squares = 0f64;
    let mut edge_sum = 0f64;

    for y in (0..height).step_by(sample_step as usize) {
        for x in (0..width).step_by(sample_step as usize) {
            let luma = luminance(rgb.get_pixel(x, y).0);
            sum += luma;
            sum_squares += luma * luma;
            count += 1;
            if x >= sample_step {
                edge_sum += (luma - luminance(rgb.get_pixel(x - sample_step, y).0)).abs();
            }
            if y >= sample_step {
                edge_sum += (luma - luminance(rgb.get_pixel(x, y - sample_step).0)).abs();
            }
        }
    }

    let mean = sum / count.max(1) as f64;
    let contrast = ((sum_squares / count.max(1) as f64) - mean * mean)
        .max(0.0)
        .sqrt();
    let sharpness = edge_sum / count.max(1) as f64;
    let resolution = ((width as f64 * height as f64) / 2_000_000.0)
        .sqrt()
        .min(1.0);
    let quality =
        (contrast / 80.0 * 0.35 + sharpness / 45.0 * 0.5 + resolution * 0.15).clamp(0.0, 1.0);

    ImageEvidence {
        filename: filename.to_string(),
        width,
        height,
        quality,
        usable: width >= 64 && height >= 64 && quality >= 0.12,
    }
}

fn luminance(pixel: [u8; 3]) -> f64 {
    pixel[0] as f64 * 0.2126 + pixel[1] as f64 * 0.7152 + pixel[2] as f64 * 0.0722
}

fn enhance_contrast(image: &DynamicImage, factor: f64) -> RgbImage {
    let mut rgb = image.to_rgb8();
    let mut means = [0f64; 3];
    let pixel_count = (rgb.width() as f64 * rgb.height() as f64).max(1.0);
    for pixel in rgb.pixels() {
        for (channel, mean) in pixel.0.iter().zip(means.iter_mut()) {
            *mean += *channel as f64;
        }
    }
    means.iter_mut().for_each(|mean| *mean /= pixel_count);
    for pixel in rgb.pixels_mut() {
        for (channel, mean) in pixel.0.iter_mut().zip(means) {
            *channel = (mean + (*channel as f64 - mean) * factor)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn png(name: &str, bright: bool) -> UploadedImage {
        let mut image = RgbImage::new(64, 64);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let value = if bright { ((x + y) % 255) as u8 } else { 24 };
            *pixel = image::Rgb([value, value.saturating_add(12), value.saturating_add(24)]);
        }
        let mut bytes = Vec::new();
        DynamicImage::ImageRgb8(image)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        UploadedImage {
            filename: name.to_string(),
            bytes,
        }
    }

    #[test]
    fn enhances_texture_and_scores_reference_images() {
        let output_dir =
            std::env::temp_dir().join(format!("3dmk-image-refinement-{}", std::process::id()));
        let output = output_dir.join("texture.png");
        let result = refine_texture(
            png("texture.png", true),
            vec![png("photo.jpg", true), png("dark.jpg", false)],
            &output,
        )
        .unwrap();

        assert!(output.is_file());
        assert_eq!(result.images_inspected, 3);
        assert_eq!(result.reference_images.len(), 2);
        std::fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn reference_photo_quality_changes_the_texture_pass() {
        let output_dir = std::env::temp_dir().join(format!(
            "3dmk-image-refinement-reference-quality-{}",
            std::process::id()
        ));
        let without_photo = output_dir.join("without-photo.png");
        let with_photo = output_dir.join("with-photo.png");
        refine_texture(png("texture.png", true), vec![], &without_photo).unwrap();
        refine_texture(
            png("texture.png", true),
            vec![png("photo.jpg", true)],
            &with_photo,
        )
        .unwrap();

        assert_ne!(
            std::fs::read(without_photo).unwrap(),
            std::fs::read(with_photo).unwrap()
        );
        std::fs::remove_dir_all(output_dir).unwrap();
    }
}
