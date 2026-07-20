use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImplicitError {
    #[error("axis-aligned bounds contain non-finite values")]
    NonFiniteBounds,
    #[error("axis-aligned bounds must have min <= max on every axis")]
    InvalidBounds,
    #[error("point set is empty")]
    EmptyPointSet,
    #[error("point count {points} does not match normal count {normals}")]
    PointNormalCountMismatch { points: usize, normals: usize },
    #[error("point or normal at index {index} contains a non-finite value")]
    NonFinitePoint { index: usize },
    #[error("normal at index {index} has zero length")]
    ZeroNormal { index: usize },
    #[error("confidence count {confidence} does not match point count {points}")]
    ConfidenceCountMismatch { confidence: usize, points: usize },
    #[error("grid dimensions must each be at least 2")]
    InvalidGridDimensions,
    #[error("grid spacing must be finite and greater than zero")]
    InvalidGridSpacing,
    #[error("grid value count {actual} does not match expected count {expected}")]
    GridValueCountMismatch { actual: usize, expected: usize },
    #[error("grid dimensions overflow addressable memory")]
    GridSizeOverflow,
    #[error("query point lies outside the field domain")]
    OutsideDomain,
    #[error("field evaluation returned a non-finite value")]
    NonFiniteFieldValue,
    #[error("ray direction must be finite and nonzero")]
    InvalidRayDirection,
    #[error("ray intersection configuration is invalid: {0}")]
    InvalidRayConfiguration(String),
    #[error("mesh index buffer contains an invalid triangle index")]
    InvalidMeshIndex,
    #[error("backend error: {0}")]
    Backend(String),
}

pub type ImplicitResult<T> = Result<T, ImplicitError>;
