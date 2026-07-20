use serde::{Deserialize, Serialize};

use crate::{ClassificationSource, ObjectClassification, ObjectFeatures, PerceptionError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmObjectRequest {
    pub png_bytes: Vec<u8>,
    pub geometry_summary: ObjectFeatures,
    pub candidate_labels: Vec<String>,
    pub local_classification: ObjectClassification,
}

pub trait VlmHook: Send + Sync {
    fn model_id(&self) -> &str;
    fn classify(&self, request: &VlmObjectRequest) -> Result<ObjectClassification>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmStructuredClassification {
    pub label: String,
    pub confidence: f32,
    #[serde(default)]
    pub alternatives: Vec<crate::ClassCandidate>,
    #[serde(default)]
    pub rationale: Option<String>,
}

impl VlmStructuredClassification {
    pub fn into_classification(self, model_id: impl Into<String>) -> ObjectClassification {
        ObjectClassification {
            label: self.label,
            confidence: self.confidence.clamp(0.0, 1.0),
            source: ClassificationSource::Vlm,
            model_id: model_id.into(),
            alternatives: self.alternatives,
            rationale: self.rationale,
        }
    }
}

pub fn parse_vlm_json(content: &str) -> Result<VlmStructuredClassification> {
    if let Ok(parsed) = serde_json::from_str::<VlmStructuredClassification>(content) {
        return validate_vlm_result(parsed);
    }
    let start = content.find('{').ok_or_else(|| {
        PerceptionError::VlmResponse("response did not contain a JSON object".into())
    })?;
    let end = content.rfind('}').ok_or_else(|| {
        PerceptionError::VlmResponse("response did not contain a complete JSON object".into())
    })?;
    let parsed = serde_json::from_str::<VlmStructuredClassification>(&content[start..=end])?;
    validate_vlm_result(parsed)
}

fn validate_vlm_result(result: VlmStructuredClassification) -> Result<VlmStructuredClassification> {
    if result.label.trim().is_empty() {
        return Err(PerceptionError::VlmResponse("label was empty".into()));
    }
    if !result.confidence.is_finite() || !(0.0..=1.0).contains(&result.confidence) {
        return Err(PerceptionError::VlmResponse(
            "confidence must be a finite value in [0,1]".into(),
        ));
    }
    Ok(result)
}
