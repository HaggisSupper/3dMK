use std::sync::Arc;

use crate::{
    crop_masked_rgba, extract_object_features, patch_to_png, slice_scene_by_mask,
    ClassificationSource, InstanceSegmenter, ObjectClassification, ObjectClassifier,
    PerceivedObject, PerceptionBatch, PerceptionError, PerceptionInput, Result, SliceConfig,
    VlmHook, VlmObjectRequest,
};

#[derive(Debug, Clone, Copy)]
pub struct PerceptionPipelineConfig {
    pub crop_padding_pixels: u32,
    pub slice: SliceConfig,
    pub require_geometry: bool,
    pub vlm_trigger_confidence: f32,
    pub vlm_minimum_confidence: f32,
    pub prefer_segmenter_hint_above: f32,
}

impl Default for PerceptionPipelineConfig {
    fn default() -> Self {
        Self {
            crop_padding_pixels: 8,
            slice: SliceConfig::default(),
            require_geometry: false,
            vlm_trigger_confidence: 0.55,
            vlm_minimum_confidence: 0.55,
            prefer_segmenter_hint_above: 0.80,
        }
    }
}

pub struct PerceptionPipeline {
    segmenter: Arc<dyn InstanceSegmenter>,
    classifier: Arc<dyn ObjectClassifier>,
    vlm: Option<Arc<dyn VlmHook>>,
    config: PerceptionPipelineConfig,
}

impl PerceptionPipeline {
    pub fn new(
        segmenter: Arc<dyn InstanceSegmenter>,
        classifier: Arc<dyn ObjectClassifier>,
        vlm: Option<Arc<dyn VlmHook>>,
        config: PerceptionPipelineConfig,
    ) -> Self {
        Self {
            segmenter,
            classifier,
            vlm,
            config,
        }
    }

    pub fn process(&self, input: &PerceptionInput) -> Result<PerceptionBatch> {
        input.frame.validate()?;
        let proposals = self.segmenter.segment(&input.frame)?;
        let mut objects = Vec::with_capacity(proposals.len());

        for proposal in proposals {
            if proposal.mask.width != input.frame.width
                || proposal.mask.height != input.frame.height
            {
                return Err(PerceptionError::ProjectionDimensionMismatch);
            }
            let crop = crop_masked_rgba(
                &input.frame,
                &proposal.mask,
                self.config.crop_padding_pixels,
            )?;

            let geometry = match (&input.scene, &input.frame.projection) {
                (Some(scene), Some(projection)) => {
                    match slice_scene_by_mask(scene, projection, &proposal.mask, self.config.slice)
                    {
                        Ok(slice) => Some(slice),
                        Err(error) if !self.config.require_geometry => {
                            log::warn!(
                                "object proposal {} could not be projected into 3D: {}",
                                proposal.id,
                                error
                            );
                            None
                        }
                        Err(error) => return Err(error),
                    }
                }
                _ if self.config.require_geometry => return Err(PerceptionError::EmptyProjection),
                _ => None,
            };

            let features = extract_object_features(&crop, geometry.as_ref())?;
            let mut classification = self.classifier.classify(&crop, &features)?;

            if let Some(class_hint) = proposal.class_hint.as_ref() {
                if proposal.confidence >= self.config.prefer_segmenter_hint_above
                    && proposal.confidence > classification.confidence
                {
                    classification = ObjectClassification::new(
                        class_hint.clone(),
                        proposal.confidence,
                        ClassificationSource::SegmenterHint,
                        &proposal.model_id,
                    );
                }
            }

            if classification.confidence < self.config.vlm_trigger_confidence {
                if let Some(vlm) = &self.vlm {
                    let request = VlmObjectRequest {
                        png_bytes: patch_to_png(&crop)?,
                        geometry_summary: features.clone(),
                        candidate_labels: input.candidate_labels.clone(),
                        local_classification: classification.clone(),
                    };
                    match vlm.classify(&request) {
                        Ok(vlm_result)
                            if vlm_result.confidence >= self.config.vlm_minimum_confidence
                                && vlm_result.confidence >= classification.confidence =>
                        {
                            classification = vlm_result;
                        }
                        Ok(_) => {}
                        Err(error) => {
                            log::warn!(
                                "VLM classification failed for proposal {}: {}",
                                proposal.id,
                                error
                            );
                        }
                    }
                }
            }

            objects.push(PerceivedObject {
                proposal,
                crop,
                geometry,
                features,
                classification,
            });
        }

        Ok(PerceptionBatch {
            source_id: input.frame.source_id.clone(),
            segmenter_model_id: self.segmenter.model_id().to_owned(),
            objects,
        })
    }
}
