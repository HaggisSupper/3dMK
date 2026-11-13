use crate::{ConversionError, ConversionRequest, ConversionResult, ErrorCode};

/// The legacy CSV engine did not interpret sections, BHA or formations and
/// merged multiple wells into a single fictitious ribbon. Fail closed instead
/// of emitting an engineering-looking result with fabricated counts/diameters.
/// GLBAdapter is the shared supported conversion engine; 3DMK consumes its GLB.
pub fn convert(_request: &ConversionRequest) -> Result<ConversionResult, ConversionError> {
    Err(ConversionError::new(
        ErrorCode::InvalidInput,
        "Legacy oilwell CSV conversion is retired: it cannot faithfully interpret sections, BHA, formations or multiple wells. Use GLBAdapter with a supported WellForge exchange, explicit EDM mapping or tabulated spatial input, then open its GLB in 3DMK. No output was generated.",
        None,
        None,
    ))
}
