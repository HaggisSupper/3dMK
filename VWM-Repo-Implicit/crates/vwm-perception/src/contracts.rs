use serde::{Deserialize, Serialize};
use vwm_core::CanonicalScene;

use crate::{PerceptionError, Result};

pub const INVALID_ELEMENT_ID: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BoundingBox2D {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl BoundingBox2D {
    pub fn right_exclusive(self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom_exclusive(self) -> u32 {
        self.y.saturating_add(self.height)
    }

    pub fn area(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageFrame {
    pub source_id: String,
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
    pub camera: Option<CameraModel>,
    pub projection: Option<ProjectionMap>,
}

impl ImageFrame {
    pub fn new(
        source_id: impl Into<String>,
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
    ) -> Result<Self> {
        let frame = Self {
            source_id: source_id.into(),
            width,
            height,
            rgba8,
            camera: None,
            projection: None,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn solid_rgba(
        source_id: impl Into<String>,
        width: u32,
        height: u32,
        rgba: [u8; 4],
    ) -> Result<Self> {
        let pixel_count = checked_pixel_count(width, height)?;
        let mut bytes = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            bytes.extend_from_slice(&rgba);
        }
        Self::new(source_id, width, height, bytes)
    }

    pub fn validate(&self) -> Result<()> {
        let expected = checked_pixel_count(self.width, self.height)? * 4;
        if self.rgba8.len() != expected {
            return Err(PerceptionError::InvalidImageBuffer {
                expected,
                actual: self.rgba8.len(),
            });
        }
        if let Some(projection) = &self.projection {
            projection.validate_dimensions(self.width, self.height)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImagePatch {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
    pub source_bounds: BoundingBox2D,
}

impl ImagePatch {
    pub fn validate(&self) -> Result<()> {
        let expected = checked_pixel_count(self.width, self.height)? * 4;
        if self.rgba8.len() != expected {
            return Err(PerceptionError::InvalidImageBuffer {
                expected,
                actual: self.rgba8.len(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CameraIntrinsics {
    pub fx: f64,
    pub fy: f64,
    pub cx: f64,
    pub cy: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CameraPose {
    /// Row-major transform from camera coordinates to scene coordinates.
    pub camera_to_scene: [[f64; 4]; 4],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CameraModel {
    pub intrinsics: CameraIntrinsics,
    pub pose: CameraPose,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionMap {
    pub width: u32,
    pub height: u32,
    /// Per-pixel source face ID. INVALID_ELEMENT_ID means no visible face.
    pub face_ids: Option<Vec<u32>>,
    /// Per-pixel source point/vertex ID. INVALID_ELEMENT_ID means no visible point.
    pub point_ids: Option<Vec<u32>>,
    /// Per-pixel metric depth along the camera ray. Non-positive or non-finite values are invalid.
    pub depth_m: Option<Vec<f32>>,
}

impl ProjectionMap {
    pub fn validate_dimensions(&self, width: u32, height: u32) -> Result<()> {
        if self.width != width || self.height != height {
            return Err(PerceptionError::ProjectionDimensionMismatch);
        }
        let expected = checked_pixel_count(width, height)?;
        for actual in [
            self.face_ids.as_ref().map(Vec::len),
            self.point_ids.as_ref().map(Vec::len),
            self.depth_m.as_ref().map(Vec::len),
        ]
        .into_iter()
        .flatten()
        {
            if actual != expected {
                return Err(PerceptionError::ProjectionDimensionMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SegmentProposal {
    pub id: usize,
    pub bbox: BoundingBox2D,
    pub mask: crate::BinaryMask,
    pub class_hint: Option<String>,
    pub confidence: f32,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSlice {
    pub scene: CanonicalScene,
    pub source_face_ids: Vec<u32>,
    pub source_point_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionInput {
    pub frame: ImageFrame,
    pub scene: Option<CanonicalScene>,
    pub candidate_labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceivedObject {
    pub proposal: SegmentProposal,
    pub crop: ImagePatch,
    pub geometry: Option<ObjectSlice>,
    pub features: crate::ObjectFeatures,
    pub classification: crate::ObjectClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionBatch {
    pub source_id: String,
    pub segmenter_model_id: String,
    pub objects: Vec<PerceivedObject>,
}

pub(crate) fn checked_pixel_count(width: u32, height: u32) -> Result<usize> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| PerceptionError::ImageProcessing("image dimensions overflow".into()))?;
    usize::try_from(pixels).map_err(|_| {
        PerceptionError::ImageProcessing("image dimensions exceed address space".into())
    })
}
