use serde::{Deserialize, Serialize};

use crate::{ImagePatch, ObjectFeatures, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassCandidate {
    pub label: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClassificationSource {
    DeterministicShape,
    LocalClassifier,
    Vlm,
    SegmenterHint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectClassification {
    pub label: String,
    pub confidence: f32,
    pub source: ClassificationSource,
    pub model_id: String,
    pub alternatives: Vec<ClassCandidate>,
    pub rationale: Option<String>,
}

impl ObjectClassification {
    pub fn new(
        label: impl Into<String>,
        confidence: f32,
        source: ClassificationSource,
        model_id: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            confidence: confidence.clamp(0.0, 1.0),
            source,
            model_id: model_id.into(),
            alternatives: Vec::new(),
            rationale: None,
        }
    }
}

pub trait ObjectClassifier: Send + Sync {
    fn model_id(&self) -> &str;
    fn classify(
        &self,
        crop: &ImagePatch,
        features: &ObjectFeatures,
    ) -> Result<ObjectClassification>;
}

#[derive(Debug, Default)]
pub struct ShapeClassifier;

impl ObjectClassifier for ShapeClassifier {
    fn model_id(&self) -> &str {
        "deterministic-shape-v1"
    }

    fn classify(
        &self,
        _crop: &ImagePatch,
        features: &ObjectFeatures,
    ) -> Result<ObjectClassification> {
        let (label, confidence, rationale) = match features.normalized_dimensions {
            Some(mut d) => {
                d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                if d[2] <= f64::EPSILON {
                    (
                        "degenerate_geometry",
                        0.95,
                        "all geometric dimensions are zero",
                    )
                } else if d[0] < 0.05 && d[1] > 0.35 {
                    (
                        "planar_object",
                        0.82,
                        "one dimension is much smaller than the other two",
                    )
                } else if d[0] < 0.15 && d[1] < 0.35 {
                    (
                        "elongated_object",
                        0.78,
                        "two dimensions are small relative to the longest",
                    )
                } else if d[0] > 0.45 {
                    (
                        "compact_object",
                        0.72,
                        "all three dimensions are comparable",
                    )
                } else {
                    (
                        "irregular_object",
                        0.55,
                        "shape does not match a stronger deterministic class",
                    )
                }
            }
            None if features.visible_pixel_count > 0 => (
                "image_object",
                0.35,
                "no 3D geometry was available; classification is image-only",
            ),
            None => (
                "unknown_object",
                0.0,
                "no visible pixels or geometry were available",
            ),
        };

        let mut result = ObjectClassification::new(
            label,
            confidence,
            ClassificationSource::DeterministicShape,
            self.model_id(),
        );
        result.rationale = Some(rationale.into());
        Ok(result)
    }
}
