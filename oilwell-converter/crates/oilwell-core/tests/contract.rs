use oilwell_core::{ConversionError, ErrorCode, InterpolationMethod, OutputFormat};

#[test]
fn serializes_stable_public_enums_and_errors() {
    assert_eq!(
        serde_json::to_string(&OutputFormat::Glb).unwrap(),
        "\"glb\""
    );
    assert_eq!(
        serde_json::to_string(&InterpolationMethod::MinimumCurvature).unwrap(),
        "\"minimum-curvature\""
    );
    let error = ConversionError::new(
        ErrorCode::MissingColumn,
        "Survey needs md",
        Some("md".to_owned()),
        None,
    );
    assert_eq!(
        serde_json::to_value(error).unwrap()["code"],
        "missing_column"
    );
}
