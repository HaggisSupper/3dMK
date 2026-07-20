use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tract_onnx::prelude::*;

use crate::segmentation::bbox_iou;
use crate::{
    letterbox_rgb_nchw, unletterbox_box, unletterbox_mask, ClassCandidate, ClassificationSource,
    ImageFrame, ImagePatch, InstanceSegmenter, ObjectClassification, ObjectClassifier,
    ObjectFeatures, PerceptionError, Result, SegmentProposal,
};

type TractPlan = TypedRunnableModel<TypedModel>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageTensorSpec {
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub mean: [f32; 3],
    #[serde(default = "unit_std")]
    pub std: [f32; 3],
}

fn unit_std() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationManifest {
    pub model_id: String,
    pub model_file: String,
    pub input: ImageTensorSpec,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default = "default_confidence")]
    pub confidence_threshold: f32,
    #[serde(default = "default_mask_threshold")]
    pub mask_threshold: f32,
    #[serde(default = "default_nms_threshold")]
    pub nms_iou_threshold: f32,
    pub output: SegmentationOutputSpec,
}

fn default_confidence() -> f32 {
    0.25
}
fn default_mask_threshold() -> f32 {
    0.5
}
fn default_nms_threshold() -> f32 {
    0.45
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SegmentationOutputSpec {
    /// Detection tensor is [N,6] or [1,N,6] with x1,y1,x2,y2,confidence,class_id.
    /// Mask tensor is [N,H,W] or [1,N,H,W]. Coordinates are in model input pixels.
    DirectMasksV1 {
        detections_output: usize,
        masks_output: usize,
    },
    /// Common YOLO segmentation export: predictions plus mask prototypes.
    YoloProtoV1 {
        predictions_output: usize,
        prototypes_output: usize,
        class_count: usize,
        mask_coefficients: usize,
        #[serde(default)]
        has_objectness: bool,
        #[serde(default)]
        prediction_layout: PredictionLayout,
        #[serde(default)]
        coordinates_normalized: bool,
        #[serde(default)]
        scores_are_logits: bool,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PredictionLayout {
    #[default]
    ChannelsFirst,
    ChannelsLast,
}

pub struct TractImageSegmenter {
    manifest: SegmentationManifest,
    model: TractPlan,
}

impl TractImageSegmenter {
    pub fn from_manifest(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let manifest: SegmentationManifest = serde_json::from_slice(&fs::read(path)?)?;
        validate_segmentation_manifest(&manifest)?;
        let model_path = resolve_model_path(path, &manifest.model_file);
        let shape = [
            1usize,
            3usize,
            manifest.input.height as usize,
            manifest.input.width as usize,
        ];
        let model = tract_onnx::onnx()
            .model_for_path(&model_path)
            .map_err(model_error)?
            .with_input_fact(0, f32::fact(&shape).into())
            .map_err(model_error)?
            .into_optimized()
            .map_err(model_error)?
            .into_runnable()
            .map_err(model_error)?;
        Ok(Self { manifest, model })
    }

    fn run(&self, frame: &ImageFrame) -> Result<(TVec<TValue>, crate::LetterboxedImage)> {
        let letterbox = letterbox_rgb_nchw(
            &frame.rgba8,
            frame.width,
            frame.height,
            self.manifest.input.width,
            self.manifest.input.height,
            self.manifest.input.mean,
            self.manifest.input.std,
        )?;
        let tensor = tract_ndarray::Array4::from_shape_vec(
            (
                1,
                3,
                self.manifest.input.height as usize,
                self.manifest.input.width as usize,
            ),
            letterbox.tensor_nchw.clone(),
        )
        .map_err(|e| PerceptionError::ModelInference(e.to_string()))?
        .into_tensor();
        let outputs = self.model.run(tvec!(tensor.into())).map_err(model_error)?;
        Ok((outputs, letterbox))
    }

    fn parse_direct(
        &self,
        frame: &ImageFrame,
        outputs: &TVec<TValue>,
        letterbox: &crate::LetterboxedImage,
        detections_output: usize,
        masks_output: usize,
    ) -> Result<Vec<SegmentProposal>> {
        let detections = output_f32(outputs, detections_output)?;
        let masks = output_f32(outputs, masks_output)?;
        let (detection_count, row_width) = matrix_last_dim(&detections.shape)?;
        if row_width < 6 {
            return Err(PerceptionError::ModelInference(format!(
                "direct detection rows require at least 6 values, got {row_width}"
            )));
        }
        let (mask_count, mask_height, mask_width) = mask_stack_shape(&masks.shape)?;
        if mask_count < detection_count {
            return Err(PerceptionError::ModelInference(format!(
                "mask count {mask_count} is smaller than detection count {detection_count}"
            )));
        }

        let mut proposals = Vec::new();
        for row in 0..detection_count {
            let base = row * row_width;
            let confidence = detections.values[base + 4];
            if !confidence.is_finite() || confidence < self.manifest.confidence_threshold {
                continue;
            }
            let class_id = detections.values[base + 5].round().max(0.0) as usize;
            let bbox = unletterbox_box(
                [
                    detections.values[base],
                    detections.values[base + 1],
                    detections.values[base + 2],
                    detections.values[base + 3],
                ],
                letterbox,
                frame.width,
                frame.height,
            );
            if bbox.width == 0 || bbox.height == 0 {
                continue;
            }
            let mask_plane = mask_width * mask_height;
            let start = row * mask_plane;
            let mask = unletterbox_mask(
                &masks.values[start..start + mask_plane],
                mask_width as u32,
                mask_height as u32,
                letterbox,
                frame.width,
                frame.height,
                self.manifest.mask_threshold,
            )?;
            if mask.foreground_count() == 0 {
                continue;
            }
            proposals.push(SegmentProposal {
                id: proposals.len(),
                bbox,
                mask,
                class_hint: self.manifest.labels.get(class_id).cloned(),
                confidence,
                model_id: self.manifest.model_id.clone(),
            });
        }
        Ok(class_aware_nms(proposals, self.manifest.nms_iou_threshold))
    }

    #[allow(clippy::too_many_arguments)]
    fn parse_yolo_proto(
        &self,
        frame: &ImageFrame,
        outputs: &TVec<TValue>,
        letterbox: &crate::LetterboxedImage,
        predictions_output: usize,
        prototypes_output: usize,
        class_count: usize,
        mask_coefficients: usize,
        has_objectness: bool,
        prediction_layout: PredictionLayout,
        coordinates_normalized: bool,
        scores_are_logits: bool,
    ) -> Result<Vec<SegmentProposal>> {
        let predictions = output_f32(outputs, predictions_output)?;
        let prototypes = output_f32(outputs, prototypes_output)?;
        let required_channels = 4 + has_objectness as usize + class_count + mask_coefficients;
        let (candidate_count, channel_count) =
            prediction_shape(&predictions.shape, prediction_layout)?;
        if channel_count < required_channels {
            return Err(PerceptionError::ModelInference(format!(
                "YOLO prediction has {channel_count} channels; {required_channels} required"
            )));
        }
        let (prototype_count, prototype_height, prototype_width) =
            prototype_shape(&prototypes.shape)?;
        if prototype_count < mask_coefficients {
            return Err(PerceptionError::ModelInference(format!(
                "prototype tensor has {prototype_count} channels; {mask_coefficients} required"
            )));
        }

        let mut candidates = Vec::<YoloCandidate>::new();
        for candidate in 0..candidate_count {
            let get = |channel: usize| -> f32 {
                prediction_value(
                    &predictions.values,
                    candidate,
                    channel,
                    candidate_count,
                    channel_count,
                    prediction_layout,
                )
            };
            let mut cx = get(0);
            let mut cy = get(1);
            let mut width = get(2);
            let mut height = get(3);
            if coordinates_normalized {
                cx *= self.manifest.input.width as f32;
                width *= self.manifest.input.width as f32;
                cy *= self.manifest.input.height as f32;
                height *= self.manifest.input.height as f32;
            }
            if ![cx, cy, width, height].iter().all(|v| v.is_finite())
                || width <= 0.0
                || height <= 0.0
            {
                continue;
            }
            let objectness_offset = 4;
            let objectness = if has_objectness {
                score_value(get(objectness_offset), scores_are_logits)
            } else {
                1.0
            };
            let class_offset = 4 + has_objectness as usize;
            let mut best_class = 0usize;
            let mut best_class_score = 0.0f32;
            for class_id in 0..class_count {
                let score = score_value(get(class_offset + class_id), scores_are_logits);
                if score > best_class_score {
                    best_class = class_id;
                    best_class_score = score;
                }
            }
            let confidence = objectness * best_class_score;
            if confidence < self.manifest.confidence_threshold {
                continue;
            }
            let bbox_model = [
                cx - width * 0.5,
                cy - height * 0.5,
                cx + width * 0.5,
                cy + height * 0.5,
            ];
            let mask_offset = class_offset + class_count;
            let coefficients = (0..mask_coefficients)
                .map(|index| get(mask_offset + index))
                .collect();
            candidates.push(YoloCandidate {
                bbox_model,
                confidence,
                class_id: best_class,
                coefficients,
            });
        }
        candidates = yolo_nms(candidates, self.manifest.nms_iou_threshold);

        let prototype_plane = prototype_height * prototype_width;
        let mut proposals = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let model_width = self.manifest.input.width as usize;
            let model_height = self.manifest.input.height as usize;
            let mut model_mask = vec![0.0f32; model_width * model_height];
            for y in 0..model_height {
                for x in 0..model_width {
                    if (x as f32) < candidate.bbox_model[0]
                        || (x as f32) >= candidate.bbox_model[2]
                        || (y as f32) < candidate.bbox_model[1]
                        || (y as f32) >= candidate.bbox_model[3]
                    {
                        continue;
                    }
                    let px = ((x as f32 / model_width as f32) * prototype_width as f32)
                        .floor()
                        .clamp(0.0, prototype_width.saturating_sub(1) as f32)
                        as usize;
                    let py = ((y as f32 / model_height as f32) * prototype_height as f32)
                        .floor()
                        .clamp(0.0, prototype_height.saturating_sub(1) as f32)
                        as usize;
                    let prototype_index = py * prototype_width + px;
                    let mut logit = 0.0f32;
                    for coefficient in 0..mask_coefficients {
                        logit += candidate.coefficients[coefficient]
                            * prototypes.values[coefficient * prototype_plane + prototype_index];
                    }
                    model_mask[y * model_width + x] = sigmoid(logit);
                }
            }

            let mask = unletterbox_mask(
                &model_mask,
                self.manifest.input.width,
                self.manifest.input.height,
                letterbox,
                frame.width,
                frame.height,
                self.manifest.mask_threshold,
            )?;
            if mask.foreground_count() == 0 {
                continue;
            }
            let bbox = unletterbox_box(candidate.bbox_model, letterbox, frame.width, frame.height);
            proposals.push(SegmentProposal {
                id: proposals.len(),
                bbox,
                mask,
                class_hint: self.manifest.labels.get(candidate.class_id).cloned(),
                confidence: candidate.confidence,
                model_id: self.manifest.model_id.clone(),
            });
        }
        Ok(proposals)
    }
}

impl InstanceSegmenter for TractImageSegmenter {
    fn model_id(&self) -> &str {
        &self.manifest.model_id
    }

    fn segment(&self, frame: &ImageFrame) -> Result<Vec<SegmentProposal>> {
        frame.validate()?;
        let (outputs, letterbox) = self.run(frame)?;
        match self.manifest.output {
            SegmentationOutputSpec::DirectMasksV1 {
                detections_output,
                masks_output,
            } => self.parse_direct(frame, &outputs, &letterbox, detections_output, masks_output),
            SegmentationOutputSpec::YoloProtoV1 {
                predictions_output,
                prototypes_output,
                class_count,
                mask_coefficients,
                has_objectness,
                prediction_layout,
                coordinates_normalized,
                scores_are_logits,
            } => self.parse_yolo_proto(
                frame,
                &outputs,
                &letterbox,
                predictions_output,
                prototypes_output,
                class_count,
                mask_coefficients,
                has_objectness,
                prediction_layout,
                coordinates_normalized,
                scores_are_logits,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationManifest {
    pub model_id: String,
    pub model_file: String,
    pub input: ImageTensorSpec,
    pub labels: Vec<String>,
    #[serde(default)]
    pub output_index: usize,
    #[serde(default)]
    pub output_activation: ClassificationActivation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationActivation {
    #[default]
    Softmax,
    Probabilities,
}

pub struct TractImageClassifier {
    manifest: ClassificationManifest,
    model: TractPlan,
}

impl TractImageClassifier {
    pub fn from_manifest(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let manifest: ClassificationManifest = serde_json::from_slice(&fs::read(path)?)?;
        if manifest.labels.is_empty() {
            return Err(PerceptionError::ModelConfiguration(
                "classification manifest must contain labels".into(),
            ));
        }
        let model_path = resolve_model_path(path, &manifest.model_file);
        let shape = [
            1usize,
            3usize,
            manifest.input.height as usize,
            manifest.input.width as usize,
        ];
        let model = tract_onnx::onnx()
            .model_for_path(&model_path)
            .map_err(model_error)?
            .with_input_fact(0, f32::fact(&shape).into())
            .map_err(model_error)?
            .into_optimized()
            .map_err(model_error)?
            .into_runnable()
            .map_err(model_error)?;
        Ok(Self { manifest, model })
    }
}

impl ObjectClassifier for TractImageClassifier {
    fn model_id(&self) -> &str {
        &self.manifest.model_id
    }

    fn classify(
        &self,
        crop: &ImagePatch,
        _features: &ObjectFeatures,
    ) -> Result<ObjectClassification> {
        crop.validate()?;
        let letterbox = letterbox_rgb_nchw(
            &crop.rgba8,
            crop.width,
            crop.height,
            self.manifest.input.width,
            self.manifest.input.height,
            self.manifest.input.mean,
            self.manifest.input.std,
        )?;
        let tensor = tract_ndarray::Array4::from_shape_vec(
            (
                1,
                3,
                self.manifest.input.height as usize,
                self.manifest.input.width as usize,
            ),
            letterbox.tensor_nchw,
        )
        .map_err(|e| PerceptionError::ModelInference(e.to_string()))?
        .into_tensor();
        let outputs = self.model.run(tvec!(tensor.into())).map_err(model_error)?;
        let output = output_f32(&outputs, self.manifest.output_index)?;
        if output.values.len() < self.manifest.labels.len() {
            return Err(PerceptionError::ModelInference(format!(
                "classifier returned {} scores for {} labels",
                output.values.len(),
                self.manifest.labels.len()
            )));
        }
        let mut scores = output.values[..self.manifest.labels.len()].to_vec();
        if matches!(
            self.manifest.output_activation,
            ClassificationActivation::Softmax
        ) {
            softmax_in_place(&mut scores);
        }
        let mut ranked: Vec<usize> = (0..scores.len()).collect();
        ranked.sort_by(|&a, &b| scores[b].partial_cmp(&scores[a]).unwrap_or(Ordering::Equal));
        let best = ranked[0];
        Ok(ObjectClassification {
            label: self.manifest.labels[best].clone(),
            confidence: scores[best].clamp(0.0, 1.0),
            source: ClassificationSource::LocalClassifier,
            model_id: self.manifest.model_id.clone(),
            alternatives: ranked
                .into_iter()
                .skip(1)
                .take(4)
                .map(|index| ClassCandidate {
                    label: self.manifest.labels[index].clone(),
                    confidence: scores[index].clamp(0.0, 1.0),
                })
                .collect(),
            rationale: None,
        })
    }
}

#[derive(Debug)]
struct FlatTensor {
    shape: Vec<usize>,
    values: Vec<f32>,
}

fn output_f32(outputs: &TVec<TValue>, index: usize) -> Result<FlatTensor> {
    let value = outputs.get(index).ok_or_else(|| {
        PerceptionError::ModelInference(format!("model output index {index} does not exist"))
    })?;
    let view = value.to_array_view::<f32>().map_err(model_error)?;
    Ok(FlatTensor {
        shape: view.shape().to_vec(),
        values: view.iter().copied().collect(),
    })
}

fn matrix_last_dim(shape: &[usize]) -> Result<(usize, usize)> {
    match shape {
        [rows, columns] => Ok((*rows, *columns)),
        [1, rows, columns] => Ok((*rows, *columns)),
        _ => Err(PerceptionError::ModelInference(format!(
            "expected [N,C] or [1,N,C], got {shape:?}"
        ))),
    }
}

fn mask_stack_shape(shape: &[usize]) -> Result<(usize, usize, usize)> {
    match shape {
        [count, height, width] => Ok((*count, *height, *width)),
        [1, count, height, width] => Ok((*count, *height, *width)),
        _ => Err(PerceptionError::ModelInference(format!(
            "expected masks [N,H,W] or [1,N,H,W], got {shape:?}"
        ))),
    }
}

fn prediction_shape(shape: &[usize], layout: PredictionLayout) -> Result<(usize, usize)> {
    match (shape, layout) {
        ([channels, candidates], PredictionLayout::ChannelsFirst) => Ok((*candidates, *channels)),
        ([1, channels, candidates], PredictionLayout::ChannelsFirst) => {
            Ok((*candidates, *channels))
        }
        ([candidates, channels], PredictionLayout::ChannelsLast) => Ok((*candidates, *channels)),
        ([1, candidates, channels], PredictionLayout::ChannelsLast) => Ok((*candidates, *channels)),
        _ => Err(PerceptionError::ModelInference(format!(
            "prediction shape {shape:?} does not match layout {layout:?}"
        ))),
    }
}

fn prediction_value(
    values: &[f32],
    candidate: usize,
    channel: usize,
    candidate_count: usize,
    channel_count: usize,
    layout: PredictionLayout,
) -> f32 {
    match layout {
        PredictionLayout::ChannelsFirst => values[channel * candidate_count + candidate],
        PredictionLayout::ChannelsLast => values[candidate * channel_count + channel],
    }
}

fn prototype_shape(shape: &[usize]) -> Result<(usize, usize, usize)> {
    match shape {
        [channels, height, width] => Ok((*channels, *height, *width)),
        [1, channels, height, width] => Ok((*channels, *height, *width)),
        _ => Err(PerceptionError::ModelInference(format!(
            "expected prototypes [M,H,W] or [1,M,H,W], got {shape:?}"
        ))),
    }
}

fn resolve_model_path(manifest_path: &Path, model_file: &str) -> PathBuf {
    let model = PathBuf::from(model_file);
    if model.is_absolute() {
        model
    } else {
        manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(model)
    }
}

fn validate_segmentation_manifest(manifest: &SegmentationManifest) -> Result<()> {
    if manifest.model_id.trim().is_empty() || manifest.model_file.trim().is_empty() {
        return Err(PerceptionError::ModelConfiguration(
            "model_id and model_file are required".into(),
        ));
    }
    if manifest.input.width == 0 || manifest.input.height == 0 {
        return Err(PerceptionError::ModelConfiguration(
            "input dimensions must be non-zero".into(),
        ));
    }
    for (name, value) in [
        ("confidence_threshold", manifest.confidence_threshold),
        ("mask_threshold", manifest.mask_threshold),
        ("nms_iou_threshold", manifest.nms_iou_threshold),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(PerceptionError::ModelConfiguration(format!(
                "{name} must be finite and in [0,1]"
            )));
        }
    }
    Ok(())
}

fn model_error(error: impl std::fmt::Display) -> PerceptionError {
    PerceptionError::ModelInference(error.to_string())
}

fn score_value(value: f32, is_logit: bool) -> f32 {
    if is_logit {
        sigmoid(value)
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn softmax_in_place(values: &mut [f32]) {
    let maximum = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0;
    for value in values.iter_mut() {
        *value = (*value - maximum).exp();
        sum += *value;
    }
    if sum > 0.0 && sum.is_finite() {
        for value in values {
            *value /= sum;
        }
    }
}

fn class_aware_nms(mut proposals: Vec<SegmentProposal>, threshold: f32) -> Vec<SegmentProposal> {
    proposals.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(Ordering::Equal)
    });
    let mut kept: Vec<SegmentProposal> = Vec::new();
    'candidate: for proposal in proposals {
        for existing in &kept {
            if proposal.class_hint == existing.class_hint
                && bbox_iou(proposal.bbox, existing.bbox) > threshold
            {
                continue 'candidate;
            }
        }
        kept.push(proposal);
    }
    for (id, proposal) in kept.iter_mut().enumerate() {
        proposal.id = id;
    }
    kept
}

#[derive(Debug)]
struct YoloCandidate {
    bbox_model: [f32; 4],
    confidence: f32,
    class_id: usize,
    coefficients: Vec<f32>,
}

fn yolo_nms(mut candidates: Vec<YoloCandidate>, threshold: f32) -> Vec<YoloCandidate> {
    candidates.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(Ordering::Equal)
    });
    let mut kept: Vec<YoloCandidate> = Vec::new();
    'candidate: for candidate in candidates {
        for existing in &kept {
            if candidate.class_id == existing.class_id
                && float_box_iou(candidate.bbox_model, existing.bbox_model) > threshold
            {
                continue 'candidate;
            }
        }
        kept.push(candidate);
    }
    kept
}

fn float_box_iou(a: [f32; 4], b: [f32; 4]) -> f32 {
    let left = a[0].max(b[0]);
    let top = a[1].max(b[1]);
    let right = a[2].min(b[2]);
    let bottom = a[3].min(b[3]);
    let intersection = (right - left).max(0.0) * (bottom - top).max(0.0);
    let area_a = (a[2] - a[0]).max(0.0) * (a[3] - a[1]).max(0.0);
    let area_b = (b[2] - b[0]).max(0.0) * (b[3] - b[1]).max(0.0);
    let union = area_a + area_b - intersection;
    if union <= 0.0 {
        0.0
    } else {
        intersection / union
    }
}
