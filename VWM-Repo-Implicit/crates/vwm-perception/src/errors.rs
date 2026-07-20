use thiserror::Error;

pub type Result<T> = std::result::Result<T, PerceptionError>;

#[derive(Debug, Error)]
pub enum PerceptionError {
    #[error("invalid image buffer: expected {expected} bytes, got {actual}")]
    InvalidImageBuffer { expected: usize, actual: usize },

    #[error("invalid mask buffer: expected {expected} values, got {actual}")]
    InvalidMaskBuffer { expected: usize, actual: usize },

    #[error("projection dimensions do not match the image/mask")]
    ProjectionDimensionMismatch,

    #[error("projection map has no usable face or point identifiers")]
    EmptyProjection,

    #[error("scene contains no geometry compatible with the projection")]
    IncompatibleScene,

    #[error("no pixels belong to the proposed object")]
    EmptyMask,

    #[error("model configuration error: {0}")]
    ModelConfiguration(String),

    #[error("model inference failed: {0}")]
    ModelInference(String),

    #[error("VLM request failed: {0}")]
    VlmRequest(String),

    #[error("VLM returned invalid structured output: {0}")]
    VlmResponse(String),

    #[error("image processing failed: {0}")]
    ImageProcessing(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
