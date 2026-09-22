use crate::{ConversionError, ErrorCode, InterpolationMethod, SurveyStation};

#[derive(Clone, Debug)]
pub struct TrajectoryPoint {
    pub md: f64,
    pub position: [f64; 3],
    pub vector: [f64; 3],
}

pub fn resample_trajectory(
    rows: &[SurveyStation],
    surface: [f64; 3],
    step: f64,
    method: InterpolationMethod,
) -> Result<Vec<TrajectoryPoint>, ConversionError> {
    if !step.is_finite() || step <= 0.0 {
        return Err(ConversionError::field(
            ErrorCode::InvalidInput,
            "Smooth step must be greater than zero",
            "smooth_step_md",
        ));
    }
    let mut rows = rows.to_vec();
    rows.sort_by(|a, b| a.md.total_cmp(&b.md));
    for pair in rows.windows(2) {
        if pair[1].md <= pair[0].md {
            return Err(ConversionError::new(
                ErrorCode::NonIncreasingMd,
                "Survey MD must strictly increase",
                Some("md".into()),
                None,
            ));
        }
    }
    let first = rows.first().ok_or_else(|| {
        ConversionError::new(
            ErrorCode::InvalidInput,
            "Survey has no stations",
            None,
            None,
        )
    })?;
    let mut out = vec![TrajectoryPoint {
        md: 0.0,
        position: surface,
        vector: direction(first.inc_deg, first.azi_deg),
    }];
    for row in rows {
        let target = row.md;
        let current = direction(row.inc_deg, row.azi_deg);
        if target == 0.0 {
            out[0].vector = current;
            continue;
        }
        let start = out.last().unwrap().clone();
        let span = target - start.md;
        let count = (span / step).ceil().max(1.0) as usize;
        for index in 1..=count {
            let f = index as f64 / count as f64;
            let next_md = start.md + span * f;
            let next = match method {
                InterpolationMethod::MinimumCurvature => slerp(start.vector, current, f),
                InterpolationMethod::LinearDirectionBlend => {
                    unit(add(scale(start.vector, 1.0 - f), scale(current, f)))
                }
            };
            let prior = out.last().unwrap();
            let delta = next_md - prior.md;
            let displacement = match method {
                InterpolationMethod::MinimumCurvature => {
                    minimum_curvature(prior.vector, next, delta)
                }
                InterpolationMethod::LinearDirectionBlend => {
                    scale(unit(add(prior.vector, next)), delta)
                }
            };
            out.push(TrajectoryPoint {
                md: next_md,
                position: add(prior.position, displacement),
                vector: next,
            });
        }
    }
    Ok(out)
}

fn direction(inc: f64, azi: f64) -> [f64; 3] {
    let (inc, azi) = (inc.to_radians(), azi.to_radians());
    [inc.sin() * azi.cos(), inc.sin() * azi.sin(), -inc.cos()]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f64; 3], value: f64) -> [f64; 3] {
    [a[0] * value, a[1] * value, a[2] * value]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn unit(a: [f64; 3]) -> [f64; 3] {
    let magnitude = dot(a, a).sqrt();
    if magnitude > 0.0 {
        scale(a, 1.0 / magnitude)
    } else {
        [0.0, 0.0, -1.0]
    }
}
fn slerp(a: [f64; 3], b: [f64; 3], f: f64) -> [f64; 3] {
    let theta = dot(a, b).clamp(-1.0, 1.0).acos();
    if theta < 1e-8 {
        return unit(add(scale(a, 1.0 - f), scale(b, f)));
    }
    let sine = theta.sin();
    unit(add(
        scale(a, ((1.0 - f) * theta).sin() / sine),
        scale(b, (f * theta).sin() / sine),
    ))
}
fn minimum_curvature(a: [f64; 3], b: [f64; 3], delta: f64) -> [f64; 3] {
    let dogleg = dot(a, b).clamp(-1.0, 1.0).acos();
    let ratio = if dogleg < 1e-8 {
        1.0
    } else {
        2.0 * (dogleg / 2.0).tan() / dogleg
    };
    scale(add(a, b), delta * 0.5 * ratio)
}
