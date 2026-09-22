use std::{fs, path::Path};

use crate::{
    export_mesh, parse_survey_csv, resample_trajectory, ConversionError, ConversionRequest,
    ConversionResult, ErrorCode, Mesh,
};

pub fn convert(request: &ConversionRequest) -> Result<ConversionResult, ConversionError> {
    let survey_bytes = fs::read(&request.survey_path).map_err(|_| {
        ConversionError::new(
            ErrorCode::ReadFailed,
            "Unable to read survey CSV",
            None,
            Some(request.survey_path.clone()),
        )
    })?;
    let sections = fs::read_to_string(&request.sections_path).map_err(|_| {
        ConversionError::new(
            ErrorCode::ReadFailed,
            "Unable to read sections CSV",
            None,
            Some(request.sections_path.clone()),
        )
    })?;
    if !sections
        .lines()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .contains("well_id")
    {
        return Err(ConversionError::field(
            ErrorCode::MissingColumn,
            "Sections needs well_id",
            "well_id",
        ));
    }
    if !request.diameter_scale.is_finite() || request.diameter_scale <= 0.0 {
        return Err(ConversionError::field(
            ErrorCode::InvalidInput,
            "Diameter scale must be greater than zero",
            "diameter_scale",
        ));
    }
    let survey = parse_survey_csv(survey_bytes)?;
    let track = resample_trajectory(
        &survey,
        [0.0; 3],
        request.smooth_step_md,
        request.interpolation_method,
    )?;
    let mesh = ribbon(&track, (0.0254 * request.diameter_scale * 4.5 / 2.0) as f32)?;
    let bytes = export_mesh(&[mesh], request.output_format)?;
    publish(Path::new(&request.output_path), &bytes)?;
    Ok(ConversionResult { output_path: request.output_path.clone(), wells: survey.iter().map(|row| &row.well_id).collect::<std::collections::BTreeSet<_>>().len(), formations: usize::from(request.formation_path.is_some()), meshes: 1, warnings: vec!["Sections, BHA, and formation tables are accepted by this first portable build; detailed component segmentation is scheduled for the next core increment.".into()] })
}

fn ribbon(track: &[crate::TrajectoryPoint], radius: f32) -> Result<Mesh, ConversionError> {
    if track.len() < 2 {
        return Err(ConversionError::new(
            ErrorCode::InvalidInput,
            "Survey needs at least two distinct MD stations",
            Some("md".into()),
            None,
        ));
    }
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for point in track {
        let side = [
            (-point.vector[1] as f32) * radius,
            (point.vector[0] as f32) * radius,
            0.0,
        ];
        let p = [
            point.position[0] as f32,
            point.position[1] as f32,
            point.position[2] as f32,
        ];
        positions.push([p[0] + side[0], p[1] + side[1], p[2] + side[2]]);
        positions.push([p[0] - side[0], p[1] - side[1], p[2] - side[2]]);
        normals.extend([[0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]);
    }
    for index in 0..(track.len() - 1) as u32 {
        let a = index * 2;
        indices.extend([a, a + 1, a + 2, a + 1, a + 3, a + 2]);
    }
    Ok(Mesh::new("Well trajectory", positions, normals, indices))
}
fn publish(path: &Path, bytes: &[u8]) -> Result<(), ConversionError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stage = parent.join(format!(
        ".{}.stage",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    fs::write(&stage, bytes).map_err(|_| {
        ConversionError::new(
            ErrorCode::WriteFailed,
            "Unable to stage output",
            None,
            Some(path.display().to_string()),
        )
    })?;
    if let Err(_) = fs::rename(&stage, path) {
        let _ = fs::remove_file(&stage);
        return Err(ConversionError::new(
            ErrorCode::PublishFailed,
            "Unable to publish output",
            None,
            Some(path.display().to_string()),
        ));
    }
    Ok(())
}
