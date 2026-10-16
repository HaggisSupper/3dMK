use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    MissingColumn,
    InvalidNumber,
    NonIncreasingMd,
    InvalidDimensions,
    UnsupportedFormat,
    ReadFailed,
    WriteFailed,
    ValidationFailed,
    PublishFailed,
}

#[derive(Clone, Debug, Deserialize, Serialize, Error)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct ConversionError {
    pub code: ErrorCode,
    pub message: String,
    pub field: Option<String>,
    pub source_path: Option<String>,
}

impl ConversionError {
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        field: Option<String>,
        source_path: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            field,
            source_path,
        }
    }

    pub fn field(code: ErrorCode, message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::new(code, message, Some(field.into()), None)
    }
}
