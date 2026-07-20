use std::collections::VecDeque;

use crate::{BinaryMask, BoundingBox2D, ImageFrame, PerceptionError, Result, SegmentProposal};

pub trait InstanceSegmenter: Send + Sync {
    fn model_id(&self) -> &str;
    fn segment(&self, frame: &ImageFrame) -> Result<Vec<SegmentProposal>>;
}

#[derive(Debug, Clone)]
pub struct ColorRegionConfig {
    pub background_rgba: [u8; 4],
    pub channel_tolerance: u8,
    pub minimum_pixels: usize,
    pub eight_connected: bool,
}

impl Default for ColorRegionConfig {
    fn default() -> Self {
        Self {
            background_rgba: [0, 0, 0, 0],
            channel_tolerance: 12,
            minimum_pixels: 32,
            eight_connected: true,
        }
    }
}

/// Deterministic fallback used when no ML model is configured. It extracts connected
/// foreground regions relative to a declared background color. It is not a semantic model.
#[derive(Debug, Clone)]
pub struct ColorRegionSegmenter {
    model_id: String,
    config: ColorRegionConfig,
}

impl ColorRegionSegmenter {
    pub fn new(config: ColorRegionConfig) -> Self {
        Self {
            model_id: "deterministic-color-regions-v1".into(),
            config,
        }
    }

    fn is_foreground(&self, rgba: [u8; 4]) -> bool {
        rgba.iter()
            .zip(self.config.background_rgba)
            .any(|(a, b)| a.abs_diff(b) > self.config.channel_tolerance)
    }
}

impl InstanceSegmenter for ColorRegionSegmenter {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn segment(&self, frame: &ImageFrame) -> Result<Vec<SegmentProposal>> {
        frame.validate()?;
        let width = frame.width as usize;
        let height = frame.height as usize;
        let mut foreground = vec![false; width * height];
        for index in 0..foreground.len() {
            let p = index * 4;
            foreground[index] = self.is_foreground([
                frame.rgba8[p],
                frame.rgba8[p + 1],
                frame.rgba8[p + 2],
                frame.rgba8[p + 3],
            ]);
        }

        let mut visited = vec![false; foreground.len()];
        let mut proposals = Vec::new();
        for seed in 0..foreground.len() {
            if visited[seed] || !foreground[seed] {
                continue;
            }

            let mut queue = VecDeque::from([seed]);
            visited[seed] = true;
            let mut component = Vec::new();
            while let Some(index) = queue.pop_front() {
                component.push(index);
                let x = index % width;
                let y = index / width;
                for dy in -1isize..=1 {
                    for dx in -1isize..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if !self.config.eight_connected && dx != 0 && dy != 0 {
                            continue;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                            continue;
                        }
                        let ni = ny as usize * width + nx as usize;
                        if !visited[ni] && foreground[ni] {
                            visited[ni] = true;
                            queue.push_back(ni);
                        }
                    }
                }
            }

            if component.len() < self.config.minimum_pixels {
                continue;
            }
            let mut data = vec![0u8; width * height];
            for index in component {
                data[index] = 1;
            }
            let mask = BinaryMask::new(frame.width, frame.height, data)?;
            let bbox = mask.bounding_box().ok_or(PerceptionError::EmptyMask)?;
            proposals.push(SegmentProposal {
                id: proposals.len(),
                bbox,
                mask,
                class_hint: None,
                confidence: 1.0,
                model_id: self.model_id.clone(),
            });
        }
        Ok(proposals)
    }
}

pub(crate) fn bbox_iou(a: BoundingBox2D, b: BoundingBox2D) -> f32 {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = a.right_exclusive().min(b.right_exclusive());
    let bottom = a.bottom_exclusive().min(b.bottom_exclusive());
    let intersection =
        u64::from(right.saturating_sub(left)) * u64::from(bottom.saturating_sub(top));
    let union = a.area() + b.area() - intersection;
    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}
