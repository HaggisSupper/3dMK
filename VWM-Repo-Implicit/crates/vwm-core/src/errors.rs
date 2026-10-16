use thiserror::Error;

#[derive(Debug, Error)]
pub enum VwmError {
    #[error("scene has no vertices")]
    EmptyScene,
    #[error("scene is not triangulated")]
    MissingTriangles,
    #[error("patch has fewer than three vertices")]
    PatchTooSmall,
    #[error("degenerate covariance or normal estimate")]
    DegeneratePatch,
    #[error("non-finite geometry encountered")]
    NonFiniteGeometry,
}
