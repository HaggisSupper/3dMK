use oilwell_core::{parse_survey_csv, resample_trajectory, ErrorCode, InterpolationMethod};

#[test]
fn accepts_bom_comments_and_one_md_zero_station() {
    let rows = parse_survey_csv(
        b"\xEF\xBB\xBFwell_id,md,inc_deg,azi_deg\n# note\n\nA1,0,0,0\nA1,100,10,90\n",
    )
    .unwrap();
    let points =
        resample_trajectory(&rows, [0.0; 3], 25.0, InterpolationMethod::MinimumCurvature).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(points.iter().filter(|point| point.md == 0.0).count(), 1);
    assert!(points
        .iter()
        .flat_map(|point| point.position)
        .all(f64::is_finite));
}

#[test]
fn rejects_duplicate_measured_depth() {
    let rows =
        parse_survey_csv(b"well_id,md,inc_deg,azi_deg\nA1,0,0,0\nA1,100,10,90\nA1,100,20,90\n")
            .unwrap();
    let error = resample_trajectory(&rows, [0.0; 3], 25.0, InterpolationMethod::MinimumCurvature)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NonIncreasingMd);
}
