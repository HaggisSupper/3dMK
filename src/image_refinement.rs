use anyhow::{bail, Context, Result};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbImage};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufWriter, Cursor, Write},
    path::Path,
};
use vwm_core::CanonicalScene;

use crate::packages::CanonicalCamera;

pub const MAX_REFERENCE_IMAGES: usize = 64;
pub const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_DECODED_PIXELS_PER_IMAGE: u64 = 32_000_000;
pub const MAX_TOTAL_DECODED_PIXELS: u64 = 128_000_000;
pub const MAX_PROJECTION_VERTICES: usize = 2_000_000;
pub const MAX_PROJECTION_FACES: usize = 4_000_000;
pub const MAX_PROJECTION_WORK_UNITS: u64 = 250_000_000;
const MAX_SAMPLE_PIXELS: u64 = 250_000;
const MAX_OCCLUSION_GRID_EDGE: usize = 1024;

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

#[derive(Debug)]
pub struct CalibratedPhoto {
    pub camera: CanonicalCamera,
    pub image: UploadedImage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PhotoProjectionOptions {
    pub max_images: usize,
    pub min_image_quality: f64,
    pub min_observations: usize,
    pub occlusion_tolerance_metres: f64,
    pub original_color_weight: f64,
}

impl Default for PhotoProjectionOptions {
    fn default() -> Self {
        Self {
            max_images: 16,
            min_image_quality: 0.12,
            min_observations: 1,
            occlusion_tolerance_metres: 0.03,
            original_color_weight: 0.15,
        }
    }
}

impl PhotoProjectionOptions {
    fn normalized(mut self) -> Self {
        self.max_images = self.max_images.clamp(1, MAX_REFERENCE_IMAGES);
        self.min_image_quality = self.min_image_quality.clamp(0.0, 1.0);
        self.min_observations = self.min_observations.clamp(1, 16);
        self.occlusion_tolerance_metres = self.occlusion_tolerance_metres.clamp(0.001, 1.0);
        self.original_color_weight = self.original_color_weight.clamp(0.0, 1.0);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectionPhotoEvidence {
    pub camera_id: String,
    pub image_path: String,
    pub filename: String,
    pub quality: f64,
    pub accepted_samples: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectionPhotoRejection {
    pub camera_id: String,
    pub image_path: String,
    pub filename: String,
    pub reason: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct PhotoProjectionReport {
    pub photos_received: usize,
    pub photos_used: usize,
    pub photos_rejected: usize,
    pub photos_without_visible_samples: usize,
    pub vertices_total: usize,
    pub vertices_colored: usize,
    pub coverage: f64,
    pub mean_observations: f64,
    pub options: PhotoProjectionOptions,
    pub geometry_mode: &'static str,
    pub visibility_method: &'static str,
    pub normal_weighting: &'static str,
    pub photo_evidence: Vec<ProjectionPhotoEvidence>,
    pub rejected_photos: Vec<ProjectionPhotoRejection>,
}

struct DecodedCalibratedPhoto {
    camera: CanonicalCamera,
    filename: String,
    image: RgbImage,
    quality: f64,
}

pub fn project_photos_to_geometry(
    input_path: &Path,
    photos: Vec<CalibratedPhoto>,
    options: PhotoProjectionOptions,
    output_path: &Path,
) -> Result<PhotoProjectionReport> {
    if photos.is_empty() {
        bail!("At least one calibrated camera/image pair is required");
    }
    if photos.len() > MAX_REFERENCE_IMAGES {
        bail!("Use at most {MAX_REFERENCE_IMAGES} calibrated images per projection");
    }
    let options = options.normalized();
    let photos_received = photos.len();
    let scene = vwm_io::load_scene(input_path)
        .with_context(|| format!("Could not load projection geometry: {}", input_path.display()))?;
    if scene.vertices.is_empty() {
        bail!("Projection geometry contains no vertices");
    }
    if scene.vertices.len() > MAX_PROJECTION_VERTICES {
        bail!("Projection geometry exceeds the {MAX_PROJECTION_VERTICES} vertex budget");
    }
    let face_count = scene.indices.as_ref().map_or(0, Vec::len);
    if face_count > MAX_PROJECTION_FACES {
        bail!("Projection geometry exceeds the {MAX_PROJECTION_FACES} face budget");
    }
    let is_mesh = face_count > 0;
    let mesh_normals = if is_mesh {
        Some(mesh_projection_normals(&scene)?)
    } else {
        None
    };

    let mut decoded = Vec::with_capacity(photos.len());
    let mut rejected_photos = Vec::new();
    let mut decoded_pixels = 0_u64;
    for photo in photos {
        let invalid_camera_reason = if !photo.camera.calibration_valid
            || !crate::packages::camera_calibration_is_valid(&photo.camera)
        {
            Some("invalid_camera_calibration")
        } else if !photo.camera.image_present {
            Some("camera_image_missing")
        } else if !photo.camera.camera_json_present {
            Some("camera_json_missing")
        } else if photo.image.bytes.len() > MAX_IMAGE_BYTES {
            Some("compressed_image_budget_exceeded")
        } else {
            None
        };
        if let Some(reason) = invalid_camera_reason {
            rejected_photos.push(photo_rejection(&photo, reason));
            continue;
        }
        let dimensions = match encoded_image_dimensions(&photo.image) {
            Ok(dimensions) => dimensions,
            Err(_) => {
                rejected_photos.push(photo_rejection(&photo, "malformed_image_header"));
                continue;
            }
        };
        decoded_pixels = checked_decoded_pixel_total(decoded_pixels, dimensions.0, dimensions.1)?;
        if dimensions != (photo.camera.dimensions[0], photo.camera.dimensions[1]) {
            rejected_photos.push(photo_rejection(
                &photo,
                "image_dimensions_do_not_match_calibration",
            ));
            continue;
        }
        let Ok(image) = decode_image(&photo.image) else {
            rejected_photos.push(photo_rejection(&photo, "image_decode_failed"));
            continue;
        };
        if image.dimensions() != dimensions {
            rejected_photos.push(photo_rejection(&photo, "image_dimensions_changed_during_decode"));
            continue;
        }
        let evidence = score_image(&photo.image.filename, &image);
        if evidence.width < 64 || evidence.height < 64 {
            rejected_photos.push(photo_rejection(&photo, "image_resolution_too_low"));
            continue;
        }
        if evidence.quality < options.min_image_quality {
            rejected_photos.push(photo_rejection(&photo, "image_quality_below_threshold"));
            continue;
        }
        decoded.push(DecodedCalibratedPhoto {
            camera: photo.camera,
            filename: photo.image.filename,
            image: image.to_rgb8(),
            quality: evidence.quality,
        });
    }
    decoded.sort_by(|left, right| {
        right
            .quality
            .total_cmp(&left.quality)
            .then_with(|| left.camera.index.cmp(&right.camera.index))
    });
    if decoded.len() > options.max_images {
        for photo in decoded.drain(options.max_images..) {
            rejected_photos.push(ProjectionPhotoRejection {
                camera_id: photo.camera.id,
                image_path: photo.camera.image_path,
                filename: photo.filename,
                reason: "max_images_limit",
            });
        }
    }
    if decoded.is_empty() {
        let summary = rejected_photos
            .iter()
            .map(|photo| format!("{}:{}", photo.filename, photo.reason))
            .collect::<Vec<_>>()
            .join(", ");
        bail!("No calibrated photos passed projection validation: {summary}");
    }
    let vertex_count = scene.vertices.len();
    let mut weighted_colors = vec![[0.0_f64; 3]; vertex_count];
    let mut total_weights = vec![0.0_f64; vertex_count];
    let mut observations = vec![0_usize; vertex_count];
    let mut photo_evidence = Vec::with_capacity(decoded.len());
    let mut work_units = 0_u64;

    for photo in decoded {
        let (grid_width, grid_height) =
            occlusion_grid_dimensions(photo.image.width(), photo.image.height());
        let nearest_depth = if let Some(indices) = scene.indices.as_deref().filter(|v| !v.is_empty()) {
            triangle_rasterized_depth(
                &scene,
                indices,
                &photo.camera,
                &photo.image,
                grid_width,
                grid_height,
                &mut work_units,
            )?
        } else {
            point_bucket_depth(
                &scene,
                &photo.camera,
                &photo.image,
                grid_width,
                grid_height,
                &mut work_units,
            )?
        };

        let mut accepted_samples = 0_usize;
        consume_projection_work(&mut work_units, scene.vertices.len() as u64)?;
        for (index, vertex) in scene.vertices.iter().enumerate() {
            let Some((pixel, depth)) =
                projected_image_sample(&photo.camera, *vertex, &photo.image)
            else {
                continue;
            };
            let grid_index = occlusion_grid_index(
                pixel,
                photo.image.width(),
                photo.image.height(),
                grid_width,
                grid_height,
            );
            if !nearest_depth[grid_index].is_finite()
                || depth > nearest_depth[grid_index] + options.occlusion_tolerance_metres
            {
                continue;
            }
            let x = pixel[0].round().clamp(0.0, photo.image.width() as f64 - 1.0) as u32;
            let y = pixel[1].round().clamp(0.0, photo.image.height() as f64 - 1.0) as u32;
            let pixel_color = photo.image.get_pixel(x, y).0;
            let angle_weight = if let Some(normals) = mesh_normals.as_ref() {
                let Some(weight) = signed_mesh_view_angle_weight(
                    &normals[index],
                    *vertex,
                    &photo.camera,
                ) else {
                    continue;
                };
                weight
            } else {
                point_cloud_view_angle_weight(
                    scene.normals.as_ref().and_then(|normals| normals.get(index)),
                    *vertex,
                    &photo.camera,
                )
            };
            let depth_weight = 1.0 / (1.0 + depth * 0.02);
            let weight = (0.05 + photo.quality) * angle_weight * depth_weight;
            for channel in 0..3 {
                weighted_colors[index][channel] += pixel_color[channel] as f64 / 255.0 * weight;
            }
            total_weights[index] += weight;
            observations[index] += 1;
            accepted_samples += 1;
        }
        photo_evidence.push(ProjectionPhotoEvidence {
            camera_id: photo.camera.id,
            image_path: photo.camera.image_path,
            filename: photo.filename,
            quality: photo.quality,
            accepted_samples,
        });
    }

    let mut output_colors = Vec::with_capacity(vertex_count);
    let mut vertices_colored = 0_usize;
    let mut observation_total = 0_usize;
    for index in 0..vertex_count {
        let original = scene
            .colors
            .as_ref()
            .and_then(|colors| colors.get(index))
            .copied()
            .unwrap_or([0.65, 0.65, 0.65]);
        if observations[index] >= options.min_observations && total_weights[index] > 0.0 {
            let projected = [
                (weighted_colors[index][0] / total_weights[index]) as f32,
                (weighted_colors[index][1] / total_weights[index]) as f32,
                (weighted_colors[index][2] / total_weights[index]) as f32,
            ];
            let original_weight = options.original_color_weight as f32;
            output_colors.push([
                projected[0] * (1.0 - original_weight) + original[0] * original_weight,
                projected[1] * (1.0 - original_weight) + original[1] * original_weight,
                projected[2] * (1.0 - original_weight) + original[2] * original_weight,
            ]);
            vertices_colored += 1;
            observation_total += observations[index];
        } else {
            output_colors.push(original);
        }
    }
    write_colored_ply(output_path, &scene, &output_colors)?;

    let photos_used = photo_evidence
        .iter()
        .filter(|photo| photo.accepted_samples > 0)
        .count();
    let photos_without_visible_samples = photo_evidence.len().saturating_sub(photos_used);
    Ok(PhotoProjectionReport {
        photos_received,
        photos_used,
        photos_rejected: rejected_photos.len(),
        photos_without_visible_samples,
        vertices_total: vertex_count,
        vertices_colored,
        coverage: vertices_colored as f64 / vertex_count as f64,
        mean_observations: observation_total as f64 / vertices_colored.max(1) as f64,
        options,
        geometry_mode: if is_mesh { "mesh" } else { "point_cloud" },
        visibility_method: if is_mesh {
            "triangle_rasterized_depth"
        } else {
            "projected_point_depth_buckets"
        },
        normal_weighting: if is_mesh {
            "signed_front_face"
        } else {
            "unsigned_when_point_normals_exist"
        },
        photo_evidence,
        rejected_photos,
    })
}

fn photo_rejection(photo: &CalibratedPhoto, reason: &'static str) -> ProjectionPhotoRejection {
    ProjectionPhotoRejection {
        camera_id: photo.camera.id.clone(),
        image_path: photo.camera.image_path.clone(),
        filename: photo.image.filename.clone(),
        reason,
    }
}

fn encoded_image_dimensions(upload: &UploadedImage) -> Result<(u32, u32)> {
    let format = image::guess_format(&upload.bytes).context("Unsupported image format")?;
    image::ImageReader::with_format(Cursor::new(upload.bytes.as_slice()), format)
        .into_dimensions()
        .context("Could not read encoded image dimensions")
}

fn checked_decoded_pixel_total(current: u64, width: u32, height: u32) -> Result<u64> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .context("Decoded image dimensions overflowed")?;
    if pixels == 0 || pixels > MAX_DECODED_PIXELS_PER_IMAGE {
        bail!("Decoded image exceeds the per-image pixel budget");
    }
    let total = current
        .checked_add(pixels)
        .context("Decoded image pixel total overflowed")?;
    if total > MAX_TOTAL_DECODED_PIXELS {
        bail!("Calibrated photos exceed the aggregate decoded pixel budget");
    }
    Ok(total)
}

fn projected_image_sample(
    camera: &CanonicalCamera,
    vertex: [f32; 3],
    image: &RgbImage,
) -> Option<([f64; 2], f64)> {
    let (pixel, depth) = projected_image_coordinates(camera, vertex)?;
    (pixel[0].is_finite()
        && pixel[1].is_finite()
        && pixel[0] >= 0.0
        && pixel[1] >= 0.0
        && pixel[0] < image.width() as f64
        && pixel[1] < image.height() as f64)
        .then_some((pixel, depth))
}

fn projected_image_coordinates(
    camera: &CanonicalCamera,
    vertex: [f32; 3],
) -> Option<([f64; 2], f64)> {
    let (pixel, depth) = camera.project_world_with_depth(vertex.map(f64::from))?;
    (pixel[0].is_finite() && pixel[1].is_finite() && depth.is_finite())
        .then_some((pixel, depth))
}

fn occlusion_grid_dimensions(width: u32, height: u32) -> (usize, usize) {
    let longest = width.max(height).max(1) as f64;
    let scale = (MAX_OCCLUSION_GRID_EDGE as f64 / longest).min(1.0);
    (
        (width as f64 * scale).ceil().max(1.0) as usize,
        (height as f64 * scale).ceil().max(1.0) as usize,
    )
}

fn occlusion_grid_index(
    pixel: [f64; 2],
    image_width: u32,
    image_height: u32,
    grid_width: usize,
    grid_height: usize,
) -> usize {
    let x = (pixel[0] / image_width.max(1) as f64 * grid_width as f64)
        .floor()
        .clamp(0.0, grid_width.saturating_sub(1) as f64) as usize;
    let y = (pixel[1] / image_height.max(1) as f64 * grid_height as f64)
        .floor()
        .clamp(0.0, grid_height.saturating_sub(1) as f64) as usize;
    y * grid_width + x
}

fn point_cloud_view_angle_weight(
    normal: Option<&[f32; 3]>,
    vertex: [f32; 3],
    camera: &CanonicalCamera,
) -> f64 {
    let Some(normal) = normal else {
        return 1.0;
    };
    let camera_center = [
        camera.world_from_camera[0][3],
        camera.world_from_camera[1][3],
        camera.world_from_camera[2][3],
    ];
    let view = [
        camera_center[0] - vertex[0] as f64,
        camera_center[1] - vertex[1] as f64,
        camera_center[2] - vertex[2] as f64,
    ];
    let view_length = (view[0] * view[0] + view[1] * view[1] + view[2] * view[2]).sqrt();
    let normal_length = ((normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
        as f64)
        .sqrt();
    if view_length <= 1e-9 || normal_length <= 1e-9 {
        return 1.0;
    }
    ((normal[0] as f64 * view[0]
        + normal[1] as f64 * view[1]
        + normal[2] as f64 * view[2])
        / (normal_length * view_length))
        .abs()
        .clamp(0.1, 1.0)
}

fn signed_mesh_view_angle_weight(
    normal: &[f64; 3],
    vertex: [f32; 3],
    camera: &CanonicalCamera,
) -> Option<f64> {
    let camera_center = [
        camera.world_from_camera[0][3],
        camera.world_from_camera[1][3],
        camera.world_from_camera[2][3],
    ];
    let view = [
        camera_center[0] - f64::from(vertex[0]),
        camera_center[1] - f64::from(vertex[1]),
        camera_center[2] - f64::from(vertex[2]),
    ];
    let view_length = (view[0] * view[0] + view[1] * view[1] + view[2] * view[2]).sqrt();
    let normal_length =
        (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if view_length <= 1.0e-9 || normal_length <= 1.0e-9 {
        return None;
    }
    let cosine = (normal[0] * view[0] + normal[1] * view[1] + normal[2] * view[2])
        / (normal_length * view_length);
    (cosine > 1.0e-6).then(|| cosine.clamp(0.05, 1.0))
}

fn mesh_projection_normals(scene: &CanonicalScene) -> Result<Vec<[f64; 3]>> {
    if let Some(normals) = scene
        .normals
        .as_ref()
        .filter(|normals| normals.len() == scene.vertices.len())
    {
        let converted = normals
            .iter()
            .map(|normal| [f64::from(normal[0]), f64::from(normal[1]), f64::from(normal[2])])
            .collect::<Vec<_>>();
        if converted.iter().all(|normal| {
            normal.iter().all(|value| value.is_finite())
                && (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
                    > 1.0e-18
        }) {
            return Ok(converted);
        }
    }

    let indices = scene.indices.as_ref().context("Mesh has no triangle indices")?;
    let mut normals = vec![[0.0_f64; 3]; scene.vertices.len()];
    for face in indices {
        let a = *scene
            .vertices
            .get(face[0] as usize)
            .context("Mesh face index is out of range")?;
        let b = *scene
            .vertices
            .get(face[1] as usize)
            .context("Mesh face index is out of range")?;
        let c = *scene
            .vertices
            .get(face[2] as usize)
            .context("Mesh face index is out of range")?;
        let ab = [
            f64::from(b[0] - a[0]),
            f64::from(b[1] - a[1]),
            f64::from(b[2] - a[2]),
        ];
        let ac = [
            f64::from(c[0] - a[0]),
            f64::from(c[1] - a[1]),
            f64::from(c[2] - a[2]),
        ];
        let normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for index in face {
            let target = &mut normals[*index as usize];
            target[0] += normal[0];
            target[1] += normal[1];
            target[2] += normal[2];
        }
    }
    Ok(normals)
}

fn point_bucket_depth(
    scene: &CanonicalScene,
    camera: &CanonicalCamera,
    image: &RgbImage,
    grid_width: usize,
    grid_height: usize,
    work_units: &mut u64,
) -> Result<Vec<f64>> {
    consume_projection_work(work_units, scene.vertices.len() as u64)?;
    let mut nearest_depth = vec![f64::INFINITY; grid_width * grid_height];
    for vertex in &scene.vertices {
        let Some((pixel, depth)) = projected_image_sample(camera, *vertex, image) else {
            continue;
        };
        let index = occlusion_grid_index(
            pixel,
            image.width(),
            image.height(),
            grid_width,
            grid_height,
        );
        nearest_depth[index] = nearest_depth[index].min(depth);
    }
    Ok(nearest_depth)
}

fn triangle_rasterized_depth(
    scene: &CanonicalScene,
    indices: &[[u32; 3]],
    camera: &CanonicalCamera,
    image: &RgbImage,
    grid_width: usize,
    grid_height: usize,
    work_units: &mut u64,
) -> Result<Vec<f64>> {
    let mut nearest_depth = vec![f64::INFINITY; grid_width * grid_height];
    for face in indices {
        consume_projection_work(work_units, 1)?;
        let mut projected = [([0.0_f64; 2], 0.0_f64); 3];
        let mut visible = true;
        for (slot, index) in face.iter().enumerate() {
            let vertex = *scene
                .vertices
                .get(*index as usize)
                .context("Mesh face index is out of range")?;
            let Some(sample) = projected_image_coordinates(camera, vertex) else {
                visible = false;
                break;
            };
            projected[slot] = sample;
        }
        if !visible {
            continue;
        }
        // Pixel-center rasterization can legitimately miss cells containing an exact
        // triangle vertex, especially for small projected triangles. Seed those cells
        // so visibility queries at the source vertices always have a surface depth.
        for (pixel, depth) in projected {
            let index = occlusion_grid_index(
                pixel,
                image.width(),
                image.height(),
                grid_width,
                grid_height,
            );
            nearest_depth[index] = nearest_depth[index].min(depth);
        }
        let points = projected.map(|(pixel, _)| {
            [
                pixel[0] / f64::from(image.width().max(1)) * grid_width as f64,
                pixel[1] / f64::from(image.height().max(1)) * grid_height as f64,
            ]
        });
        let area = edge_function(points[1], points[2], points[0]);
        if !area.is_finite() || area.abs() <= 1.0e-12 {
            continue;
        }
        let raw_min_x = points.iter().map(|point| point[0]).fold(f64::INFINITY, f64::min);
        let raw_max_x = points
            .iter()
            .map(|point| point[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let raw_min_y = points.iter().map(|point| point[1]).fold(f64::INFINITY, f64::min);
        let raw_max_y = points
            .iter()
            .map(|point| point[1])
            .fold(f64::NEG_INFINITY, f64::max);
        if raw_max_x < 0.0
            || raw_max_y < 0.0
            || raw_min_x >= grid_width as f64
            || raw_min_y >= grid_height as f64
        {
            continue;
        }
        let min_x = raw_min_x.floor().max(0.0) as usize;
        let max_x = raw_max_x
            .ceil()
            .min(grid_width.saturating_sub(1) as f64) as usize;
        let min_y = raw_min_y.floor().max(0.0) as usize;
        let max_y = raw_max_y
            .ceil()
            .min(grid_height.saturating_sub(1) as f64) as usize;
        let raster_work = (max_x - min_x + 1) as u64 * (max_y - min_y + 1) as u64;
        consume_projection_work(work_units, raster_work)?;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let point = [x as f64 + 0.5, y as f64 + 0.5];
                let weights = [
                    edge_function(points[1], points[2], point) / area,
                    edge_function(points[2], points[0], point) / area,
                    edge_function(points[0], points[1], point) / area,
                ];
                if weights.iter().any(|weight| *weight < -1.0e-9) {
                    continue;
                }
                let inverse_depth = weights[0] / projected[0].1
                    + weights[1] / projected[1].1
                    + weights[2] / projected[2].1;
                if inverse_depth <= 0.0 || !inverse_depth.is_finite() {
                    continue;
                }
                let index = y * grid_width + x;
                nearest_depth[index] = nearest_depth[index].min(1.0 / inverse_depth);
            }
        }
    }
    Ok(nearest_depth)
}

fn edge_function(left: [f64; 2], right: [f64; 2], point: [f64; 2]) -> f64 {
    (point[0] - left[0]) * (right[1] - left[1])
        - (point[1] - left[1]) * (right[0] - left[0])
}

fn consume_projection_work(current: &mut u64, amount: u64) -> Result<()> {
    *current = current
        .checked_add(amount)
        .context("Projection work counter overflowed")?;
    if *current > MAX_PROJECTION_WORK_UNITS {
        bail!("Calibrated projection exceeds the work budget");
    }
    Ok(())
}

fn write_colored_ply(
    output_path: &Path,
    scene: &CanonicalScene,
    colors: &[[f32; 3]],
) -> Result<()> {
    if colors.len() != scene.vertices.len() {
        bail!("Projected color count does not match scene vertex count");
    }
    let output_parent = output_path
        .parent()
        .context("Projected geometry needs an output directory")?;
    std::fs::create_dir_all(output_parent)?;
    let mut writer = BufWriter::new(File::create(output_path)?);
    let normals = scene
        .normals
        .as_ref()
        .filter(|normals| normals.len() == scene.vertices.len());
    let uvs = scene
        .uvs
        .as_ref()
        .filter(|uvs| uvs.len() == scene.vertices.len());
    writeln!(writer, "ply")?;
    writeln!(writer, "format ascii 1.0")?;
    writeln!(writer, "comment 3DMk calibrated photo projection")?;
    writeln!(writer, "element vertex {}", scene.vertices.len())?;
    writeln!(writer, "property float x")?;
    writeln!(writer, "property float y")?;
    writeln!(writer, "property float z")?;
    if normals.is_some() {
        writeln!(writer, "property float nx")?;
        writeln!(writer, "property float ny")?;
        writeln!(writer, "property float nz")?;
    }
    writeln!(writer, "property uchar red")?;
    writeln!(writer, "property uchar green")?;
    writeln!(writer, "property uchar blue")?;
    if uvs.is_some() {
        writeln!(writer, "property float s")?;
        writeln!(writer, "property float t")?;
    }
    if let Some(indices) = &scene.indices {
        writeln!(writer, "element face {}", indices.len())?;
        writeln!(writer, "property list uchar uint vertex_indices")?;
    }
    writeln!(writer, "end_header")?;
    for (index, vertex) in scene.vertices.iter().enumerate() {
        write!(writer, "{} {} {}", vertex[0], vertex[1], vertex[2])?;
        if let Some(normals) = normals {
            let normal = normals[index];
            write!(writer, " {} {} {}", normal[0], normal[1], normal[2])?;
        }
        let color = colors[index].map(|channel| {
            (channel.clamp(0.0, 1.0) * 255.0).round() as u8
        });
        write!(writer, " {} {} {}", color[0], color[1], color[2])?;
        if let Some(uvs) = uvs {
            write!(writer, " {} {}", uvs[index][0], uvs[index][1])?;
        }
        writeln!(writer)?;
    }
    if let Some(indices) = &scene.indices {
        for face in indices {
            writeln!(writer, "3 {} {} {}", face[0], face[1], face[2])?;
        }
    }
    writer.flush()?;
    Ok(())
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
    use crate::packages::CanonicalCamera;
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

    fn red_detail_png(name: &str) -> UploadedImage {
        let mut image = RgbImage::new(64, 64);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let red = if (x + y) % 2 == 0 { 255 } else { 128 };
            *pixel = image::Rgb([red, 0, 0]);
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

    fn calibrated_camera(image_path: &str) -> CanonicalCamera {
        CanonicalCamera {
            id: "camera-1".to_owned(),
            index: 1,
            image_path: image_path.to_owned(),
            camera_json_path: "RawImages/1.json".to_owned(),
            dimensions: [64, 64],
            focal_pixels: [50.0, 50.0],
            principal_point_pixels: [32.0, 32.0],
            world_from_camera: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            image_present: true,
            camera_json_present: true,
            calibration_valid: true,
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

    #[test]
    fn calibrated_photo_projects_vertex_color_without_changing_topology() {
        let output_dir =
            std::env::temp_dir().join(format!("3dmk-photo-project-visible-{}", std::process::id()));
        std::fs::create_dir_all(&output_dir).unwrap();
        let input = output_dir.join("input.ply");
        let output = output_dir.join("output.ply");
        std::fs::write(
            &input,
            "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 -2\n0.5 0 -2\n0 0.5 -2\n3 0 1 2\n",
        )
        .unwrap();
        let report = project_photos_to_geometry(
            &input,
            vec![CalibratedPhoto {
                camera: calibrated_camera("RawImages/1.jpg"),
                image: red_detail_png("1.jpg"),
            }],
            PhotoProjectionOptions {
                min_image_quality: 0.0,
                original_color_weight: 0.0,
                ..Default::default()
            },
            &output,
        )
        .unwrap();
        let scene = vwm_io::load_scene(&output).unwrap();
        let colors = scene.colors.unwrap();

        assert_eq!(scene.vertices.len(), 3);
        assert_eq!(scene.indices.unwrap(), vec![[0, 1, 2]]);
        assert_eq!(report.vertices_colored, 3);
        assert_eq!(report.photos_used, 1);
        assert_eq!(report.visibility_method, "triangle_rasterized_depth");
        assert_eq!(report.normal_weighting, "signed_front_face");
        assert!(colors.iter().all(|color| color[0] > 0.45 && color[1] < 0.05 && color[2] < 0.05));
        std::fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn calibrated_photo_occlusion_rejects_farther_vertex_at_same_pixel() {
        let output_dir =
            std::env::temp_dir().join(format!("3dmk-photo-project-occlusion-{}", std::process::id()));
        std::fs::create_dir_all(&output_dir).unwrap();
        let input = output_dir.join("input.ply");
        let output = output_dir.join("output.ply");
        std::fs::write(
            &input,
            "ply\nformat ascii 1.0\nelement vertex 2\nproperty float x\nproperty float y\nproperty float z\nend_header\n0 0 -1\n0 0 -2\n",
        )
        .unwrap();
        let report = project_photos_to_geometry(
            &input,
            vec![CalibratedPhoto {
                camera: calibrated_camera("RawImages/1.jpg"),
                image: red_detail_png("1.jpg"),
            }],
            PhotoProjectionOptions {
                min_image_quality: 0.0,
                occlusion_tolerance_metres: 0.01,
                original_color_weight: 0.0,
                ..Default::default()
            },
            &output,
        )
        .unwrap();
        let scene = vwm_io::load_scene(&output).unwrap();
        let colors = scene.colors.unwrap();

        assert_eq!(report.vertices_colored, 1);
        assert_eq!(report.visibility_method, "projected_point_depth_buckets");
        assert!(colors[0][0] > 0.45);
        assert!((colors[1][0] - 0.65).abs() < 0.02);
        std::fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn calibrated_projection_enforces_decoded_pixel_and_work_budgets() {
        assert!(checked_decoded_pixel_total(0, 1, 1).is_ok());
        assert!(checked_decoded_pixel_total(0, u32::MAX, u32::MAX).is_err());
        assert!(checked_decoded_pixel_total(MAX_TOTAL_DECODED_PIXELS, 1, 1).is_err());

        let mut work = MAX_PROJECTION_WORK_UNITS - 1;
        assert!(consume_projection_work(&mut work, 1).is_ok());
        assert!(consume_projection_work(&mut work, 1).is_err());
    }

    #[test]
    fn calibrated_mesh_uses_triangle_raster_depth_for_occlusion() {
        let output_dir = std::env::temp_dir().join(format!(
            "3dmk-photo-project-triangle-occlusion-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&output_dir).unwrap();
        let input = output_dir.join("input.ply");
        let output = output_dir.join("output.ply");
        std::fs::write(
            &input,
            "ply\nformat ascii 1.0\nelement vertex 6\nproperty float x\nproperty float y\nproperty float z\nelement face 2\nproperty list uchar int vertex_indices\nend_header\n-0.5 -0.5 -1\n0.5 -0.5 -1\n0 0.5 -1\n-0.2 -0.2 -2\n0.2 -0.2 -2\n0 0.2 -2\n3 0 1 2\n3 3 4 5\n",
        )
        .unwrap();
        let report = project_photos_to_geometry(
            &input,
            vec![CalibratedPhoto {
                camera: calibrated_camera("RawImages/1.jpg"),
                image: red_detail_png("1.jpg"),
            }],
            PhotoProjectionOptions {
                min_image_quality: 0.0,
                occlusion_tolerance_metres: 0.01,
                original_color_weight: 0.0,
                ..Default::default()
            },
            &output,
        )
        .unwrap();
        let colors = vwm_io::load_scene(&output).unwrap().colors.unwrap();

        assert_eq!(report.vertices_colored, 3);
        assert!(colors[..3].iter().all(|color| color[0] > 0.45));
        assert!(colors[3..]
            .iter()
            .all(|color| (color[0] - 0.65).abs() < 0.02));
        std::fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn calibrated_mesh_signed_weighting_rejects_backfaces() {
        let output_dir = std::env::temp_dir().join(format!(
            "3dmk-photo-project-backface-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&output_dir).unwrap();
        let input = output_dir.join("input.ply");
        let output = output_dir.join("output.ply");
        std::fs::write(
            &input,
            "ply\nformat ascii 1.0\nelement vertex 6\nproperty float x\nproperty float y\nproperty float z\nelement face 2\nproperty list uchar int vertex_indices\nend_header\n-0.8 -0.3 -2\n-0.2 -0.3 -2\n-0.5 0.3 -2\n0.2 -0.3 -2\n0.8 -0.3 -2\n0.5 0.3 -2\n3 0 1 2\n3 3 5 4\n",
        )
        .unwrap();
        let report = project_photos_to_geometry(
            &input,
            vec![CalibratedPhoto {
                camera: calibrated_camera("RawImages/1.jpg"),
                image: red_detail_png("1.jpg"),
            }],
            PhotoProjectionOptions {
                min_image_quality: 0.0,
                original_color_weight: 0.0,
                ..Default::default()
            },
            &output,
        )
        .unwrap();
        let colors = vwm_io::load_scene(&output).unwrap().colors.unwrap();

        assert_eq!(report.vertices_colored, 3);
        assert!(colors[..3].iter().all(|color| color[0] > 0.45));
        assert!(colors[3..]
            .iter()
            .all(|color| (color[0] - 0.65).abs() < 0.02));
        std::fs::remove_dir_all(output_dir).unwrap();
    }
}
