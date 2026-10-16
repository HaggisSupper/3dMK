use crate::{ConversionError, ErrorCode};

#[derive(Clone, Debug, PartialEq)]
pub struct SurveyStation {
    pub well_id: String,
    pub md: f64,
    pub inc_deg: f64,
    pub azi_deg: f64,
}

pub fn parse_survey_csv(input: impl AsRef<[u8]>) -> Result<Vec<SurveyStation>, ConversionError> {
    let text = std::str::from_utf8(input.as_ref()).map_err(|_| {
        ConversionError::new(
            ErrorCode::ReadFailed,
            "Survey CSV must be UTF-8",
            None,
            None,
        )
    })?;
    let filtered = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(filtered.as_bytes());
    let headers = reader
        .headers()
        .map_err(|_| {
            ConversionError::new(
                ErrorCode::InvalidInput,
                "Survey CSV has no readable header",
                None,
                None,
            )
        })?
        .iter()
        .map(|field| field.trim_start_matches('\u{feff}').to_ascii_lowercase())
        .collect::<Vec<_>>();
    let required = ["well_id", "md", "inc_deg", "azi_deg"];
    for name in required {
        if !headers.iter().any(|header| header == name) {
            return Err(ConversionError::field(
                ErrorCode::MissingColumn,
                format!("Survey needs {name}"),
                name,
            ));
        }
    }
    let column = |name: &str| {
        headers
            .iter()
            .position(|header| header == name)
            .expect("required columns checked")
    };
    let mut stations = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|_| {
            ConversionError::new(
                ErrorCode::InvalidInput,
                "Survey CSV contains an invalid row",
                None,
                None,
            )
        })?;
        stations.push(SurveyStation {
            well_id: record.get(column("well_id")).unwrap_or_default().to_owned(),
            md: finite(record.get(column("md")).unwrap_or_default(), "md")?,
            inc_deg: finite(record.get(column("inc_deg")).unwrap_or_default(), "inc_deg")?,
            azi_deg: finite(record.get(column("azi_deg")).unwrap_or_default(), "azi_deg")?,
        });
    }
    if stations.is_empty() {
        return Err(ConversionError::new(
            ErrorCode::InvalidInput,
            "Survey CSV has no data rows",
            None,
            None,
        ));
    }
    Ok(stations)
}

fn finite(value: &str, field: &str) -> Result<f64, ConversionError> {
    let parsed = value.parse::<f64>().map_err(|_| {
        ConversionError::field(
            ErrorCode::InvalidNumber,
            format!("{field} must be numeric"),
            field,
        )
    })?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(ConversionError::field(
            ErrorCode::InvalidNumber,
            format!("{field} must be finite"),
            field,
        ))
    }
}
