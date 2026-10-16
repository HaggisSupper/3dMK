use serde::{Deserialize, Serialize};

use crate::{ImagePatch, ObjectSlice, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectFeatures {
    pub visible_pixel_count: usize,
    pub alpha_coverage: f32,
    pub mean_rgb: [f32; 3],
    pub image_aspect_ratio: f32,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub centroid: Option<[f64; 3]>,
    pub aabb_min: Option<[f64; 3]>,
    pub aabb_max: Option<[f64; 3]>,
    pub dimensions: Option<[f64; 3]>,
    pub normalized_dimensions: Option<[f64; 3]>,
}

impl ObjectFeatures {
    pub fn compact_summary(&self) -> String {
        format!(
            "pixels={}, alpha_coverage={:.3}, mean_rgb=({:.3},{:.3},{:.3}), \
             image_aspect={:.3}, vertices={}, triangles={}, dimensions={:?}, normalized_dimensions={:?}",
            self.visible_pixel_count,
            self.alpha_coverage,
            self.mean_rgb[0],
            self.mean_rgb[1],
            self.mean_rgb[2],
            self.image_aspect_ratio,
            self.vertex_count,
            self.triangle_count,
            self.dimensions,
            self.normalized_dimensions,
        )
    }
}

pub fn extract_object_features(
    crop: &ImagePatch,
    geometry: Option<&ObjectSlice>,
) -> Result<ObjectFeatures> {
    crop.validate()?;
    let mut visible = 0usize;
    let mut sums = [0u64; 3];
    for px in crop.rgba8.chunks_exact(4) {
        if px[3] != 0 {
            visible += 1;
            sums[0] += u64::from(px[0]);
            sums[1] += u64::from(px[1]);
            sums[2] += u64::from(px[2]);
        }
    }
    let pixel_count = (crop.width as usize).saturating_mul(crop.height as usize);
    let mean_rgb = if visible == 0 {
        [0.0; 3]
    } else {
        [
            sums[0] as f32 / visible as f32 / 255.0,
            sums[1] as f32 / visible as f32 / 255.0,
            sums[2] as f32 / visible as f32 / 255.0,
        ]
    };

    let (vertex_count, triangle_count, centroid, aabb_min, aabb_max, dimensions, normalized) =
        if let Some(slice) = geometry {
            let vertices = &slice.scene.vertices;
            if vertices.is_empty() {
                (0, 0, None, None, None, None, None)
            } else {
                let mut min = [f64::INFINITY; 3];
                let mut max = [f64::NEG_INFINITY; 3];
                let mut sum = [0.0f64; 3];
                for point in vertices {
                    for axis in 0..3 {
                        let value = f64::from(point[axis]);
                        min[axis] = min[axis].min(value);
                        max[axis] = max[axis].max(value);
                        sum[axis] += value;
                    }
                }
                let centroid = [
                    sum[0] / vertices.len() as f64,
                    sum[1] / vertices.len() as f64,
                    sum[2] / vertices.len() as f64,
                ];
                let dimensions = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
                let largest = dimensions.into_iter().fold(0.0f64, f64::max);
                let normalized = if largest > 0.0 {
                    Some([
                        dimensions[0] / largest,
                        dimensions[1] / largest,
                        dimensions[2] / largest,
                    ])
                } else {
                    Some([0.0; 3])
                };
                (
                    vertices.len(),
                    slice.scene.indices.as_ref().map_or(0, Vec::len),
                    Some(centroid),
                    Some(min),
                    Some(max),
                    Some(dimensions),
                    normalized,
                )
            }
        } else {
            (0, 0, None, None, None, None, None)
        };

    Ok(ObjectFeatures {
        visible_pixel_count: visible,
        alpha_coverage: if pixel_count == 0 {
            0.0
        } else {
            visible as f32 / pixel_count as f32
        },
        mean_rgb,
        image_aspect_ratio: if crop.height == 0 {
            0.0
        } else {
            crop.width as f32 / crop.height as f32
        },
        vertex_count,
        triangle_count,
        centroid,
        aabb_min,
        aabb_max,
        dimensions,
        normalized_dimensions: normalized,
    })
}
