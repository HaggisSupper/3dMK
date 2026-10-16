use anyhow::Result;
use nalgebra::{Matrix2, Vector2, Vector3};
use vwm_core::{PlaneIntersection, PlanePrimitive};

use crate::math::v3;

pub fn compute_plane_intersections(planes: &[PlanePrimitive]) -> Result<Vec<PlaneIntersection>> {
    let mut out = Vec::new();

    for i in 0..planes.len() {
        for j in (i + 1)..planes.len() {
            let n1 = v3(planes[i].normal);
            let n2 = v3(planes[j].normal);
            let d1 = planes[i].offset;
            let d2 = planes[j].offset;

            let direction = n1.cross(&n2);
            let denom = direction.norm_squared();
            if denom < 1e-12 {
                continue;
            }

            let point = point_on_intersection_line(n1, d1, n2, d2, direction);

            out.push(PlaneIntersection {
                plane_a: planes[i].id,
                plane_b: planes[j].id,
                point_on_line: [point.x, point.y, point.z],
                line_direction: [direction.x, direction.y, direction.z],
            });
        }
    }

    Ok(out)
}

fn point_on_intersection_line(
    n1: Vector3<f64>,
    d1: f64,
    n2: Vector3<f64>,
    d2: f64,
    dir: Vector3<f64>,
) -> Vector3<f64> {
    let abs_dir = [dir.x.abs(), dir.y.abs(), dir.z.abs()];
    let fixed_axis = abs_dir
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(idx, _)| idx)
        .unwrap_or(2);

    match fixed_axis {
        0 => {
            let a = Matrix2::new(n1.y, n1.z, n2.y, n2.z);
            let b = Vector2::new(-d1, -d2);
            if let Some(sol) = a.try_inverse().map(|inv| inv * b) {
                Vector3::new(0.0, sol.x, sol.y)
            } else {
                Vector3::zeros()
            }
        }
        1 => {
            let a = Matrix2::new(n1.x, n1.z, n2.x, n2.z);
            let b = Vector2::new(-d1, -d2);
            if let Some(sol) = a.try_inverse().map(|inv| inv * b) {
                Vector3::new(sol.x, 0.0, sol.y)
            } else {
                Vector3::zeros()
            }
        }
        _ => {
            let a = Matrix2::new(n1.x, n1.y, n2.x, n2.y);
            let b = Vector2::new(-d1, -d2);
            if let Some(sol) = a.try_inverse().map(|inv| inv * b) {
                Vector3::new(sol.x, sol.y, 0.0)
            } else {
                Vector3::zeros()
            }
        }
    }
}
