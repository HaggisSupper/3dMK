use std::sync::Arc;

use vwm_perception::{
    BinaryMask, BoundingBox2D, ClassificationSource, ImageFrame, ImagePatch, InstanceSegmenter,
    ObjectClassification, ObjectClassifier, ObjectFeatures, PerceptionInput, PerceptionPipeline,
    PerceptionPipelineConfig, SegmentProposal, VlmHook, VlmObjectRequest,
};

struct OneSegment;
impl InstanceSegmenter for OneSegment {
    fn model_id(&self) -> &str {
        "test-segmenter"
    }

    fn segment(&self, frame: &ImageFrame) -> vwm_perception::Result<Vec<SegmentProposal>> {
        Ok(vec![SegmentProposal {
            id: 7,
            bbox: BoundingBox2D {
                x: 0,
                y: 0,
                width: frame.width,
                height: frame.height,
            },
            mask: BinaryMask::filled(frame.width, frame.height, true)?,
            class_hint: None,
            confidence: 0.9,
            model_id: self.model_id().to_owned(),
        }])
    }
}

struct LowConfidenceClassifier;
impl ObjectClassifier for LowConfidenceClassifier {
    fn model_id(&self) -> &str {
        "test-classifier"
    }

    fn classify(
        &self,
        _crop: &ImagePatch,
        _features: &ObjectFeatures,
    ) -> vwm_perception::Result<ObjectClassification> {
        Ok(ObjectClassification::new(
            "unknown_object",
            0.3,
            ClassificationSource::LocalClassifier,
            self.model_id(),
        ))
    }
}

struct FakeVlm;
impl VlmHook for FakeVlm {
    fn model_id(&self) -> &str {
        "fake-vlm"
    }

    fn classify(
        &self,
        _request: &VlmObjectRequest,
    ) -> vwm_perception::Result<ObjectClassification> {
        Ok(ObjectClassification::new(
            "industrial_battery",
            0.88,
            ClassificationSource::Vlm,
            self.model_id(),
        ))
    }
}

#[test]
fn low_confidence_local_result_is_adjudicated_by_vlm() {
    let pipeline = PerceptionPipeline::new(
        Arc::new(OneSegment),
        Arc::new(LowConfidenceClassifier),
        Some(Arc::new(FakeVlm)),
        PerceptionPipelineConfig {
            vlm_trigger_confidence: 0.6,
            ..Default::default()
        },
    );

    let input = PerceptionInput {
        frame: ImageFrame::solid_rgba("test", 3, 2, [80, 90, 100, 255]).unwrap(),
        scene: None,
        candidate_labels: vec!["industrial_battery".into(), "cabinet".into()],
    };

    let batch = pipeline.process(&input).unwrap();
    assert_eq!(batch.objects.len(), 1);
    assert_eq!(batch.objects[0].classification.label, "industrial_battery");
    assert_eq!(
        batch.objects[0].classification.source,
        ClassificationSource::Vlm
    );
}
