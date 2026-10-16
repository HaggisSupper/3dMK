use anyhow::Result;
use nalgebra::Point3;
use vwm_core::{CanonicalScene, Patch, PlanePrimitive};

use crate::math::{arr3, orthonormal_plane_basis, p3, plane_from_points, project_to_plane_2d};

#[derive(Debug, Clone, Copy)]
pub struct PlaneConfig {
    pub min_vertices: usize,
}

pub fn fit_planes(
    scene: &CanonicalScene,
    patches: &[Patch],
    cfg: PlaneConfig,
) -> Result<Vec<PlanePrimitive>> {
    let mut out = Vec::<PlanePrimitive>::new();

    for patch in patches {
        if patch.vertex_indices.len() < cfg.min_vertices {
            continue;
        }
        let pts: Vec<Point3<f64>> = patch
            .vertex_indices
            .iter()
            .map(|&i| p3(scene.vertices[i]))
            .collect();

        if let Some((n, d, rmse)) = plane_from_points(&pts) {
            let center = crate::math::centroid(&pts);
            let (u, v) = orthonormal_plane_basis(n);

            let mut boundary_2d: Vec<[f64; 2]> = pts
                .iter()
                .map(|&p| project_to_plane_2d(center, u, v, p))
                .collect();

            boundary_2d = convex_hull_2d(&boundary_2d);

            out.push(PlanePrimitive {
                id: out.len(),
                patch_id: patch.id,
                normal: arr3(n.into_inner()),
                offset: d,
                support_vertices: patch.vertex_indices.clone(),
                residual_rmse: rmse,
                support_area: polygon_area_2d(&boundary_2d),
                centroid: [center.x, center.y, center.z],
                boundary_2d,
                boundary_basis_origin: [center.x, center.y, center.z],
                boundary_basis_u: arr3(u),
                boundary_basis_v: arr3(v),
            });
        }
    }

    Ok(out)
}

fn convex_hull_2d(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut pts = points.to_vec();
    pts.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if pts.len() <= 3 {
        return pts;
    }

    let mut lower = Vec::<[f64; 2]>::new();
    for p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0 {
            lower.pop();
        }
        lower.push(*p);
    }

    let mut upper = Vec::<[f64; 2]>::new();
    for p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0 {
            upper.pop();
        }
        upper.push(*p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn polygon_area_2d(poly: &[[f64; 2]]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        area += a[0] * b[1] - b[0] * a[1];
    }
    area.abs() * 0.5
}
