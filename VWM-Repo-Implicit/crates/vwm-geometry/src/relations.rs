use anyhow::Result;
use nalgebra::UnitVector3;
use vwm_core::{PlanePrimitive, PlaneRelation, PlaneRelationKind};

use crate::math::{angle_between_degrees, v3};

#[derive(Debug, Clone, Copy)]
pub struct RelationConfig {
    pub parallel_angle_threshold_degrees: f64,
    pub perpendicular_angle_threshold_degrees: f64,
    pub adjacency_centroid_distance: f64,
    pub coplanar_offset_threshold: f64,
}

pub fn build_plane_relations(
    planes: &[PlanePrimitive],
    cfg: RelationConfig,
) -> Result<Vec<PlaneRelation>> {
    let mut out = Vec::<PlaneRelation>::new();

    for i in 0..planes.len() {
        for j in (i + 1)..planes.len() {
            let a_n = UnitVector3::new_normalize(v3(planes[i].normal));
            let b_n = UnitVector3::new_normalize(v3(planes[j].normal));
            let angle = angle_between_degrees(a_n, b_n);
            let centroid_distance = dist3(planes[i].centroid, planes[j].centroid);
            let offset_difference = (planes[i].offset - planes[j].offset).abs();

            let kind = if angle <= cfg.parallel_angle_threshold_degrees
                || (180.0 - angle) <= cfg.parallel_angle_threshold_degrees
            {
                if offset_difference <= cfg.coplanar_offset_threshold {
                    PlaneRelationKind::Coplanar
                } else {
                    PlaneRelationKind::Parallel
                }
            } else if (angle - 90.0).abs() <= cfg.perpendicular_angle_threshold_degrees {
                if centroid_distance <= cfg.adjacency_centroid_distance {
                    PlaneRelationKind::Adjacent
                } else {
                    PlaneRelationKind::Perpendicular
                }
            } else {
                PlaneRelationKind::Unknown
            };

            let confidence =
                relation_confidence(kind, angle, centroid_distance, offset_difference, cfg);

            out.push(PlaneRelation {
                source_plane_id: planes[i].id,
                target_plane_id: planes[j].id,
                kind,
                measured_angle_degrees: angle,
                centroid_distance,
                offset_difference,
                confidence,
            });
        }
    }

    Ok(out)
}

fn relation_confidence(
    kind: PlaneRelationKind,
    angle: f64,
    dist: f64,
    off: f64,
    cfg: RelationConfig,
) -> f64 {
    match kind {
        PlaneRelationKind::Coplanar => {
            let a = 1.0
                - (angle.min(180.0 - angle) / cfg.parallel_angle_threshold_degrees.max(1.0))
                    .min(1.0);
            let o = 1.0 - (off / cfg.coplanar_offset_threshold.max(1e-6)).min(1.0);
            ((a + o) * 0.5).clamp(0.0, 1.0)
        }
        PlaneRelationKind::Parallel => {
            let a = 1.0
                - (angle.min(180.0 - angle) / cfg.parallel_angle_threshold_degrees.max(1.0))
                    .min(1.0);
            a.clamp(0.0, 1.0)
        }
        PlaneRelationKind::Adjacent | PlaneRelationKind::Perpendicular => {
            let a = 1.0
                - ((angle - 90.0).abs() / cfg.perpendicular_angle_threshold_degrees.max(1.0))
                    .min(1.0);
            let d = 1.0 - (dist / cfg.adjacency_centroid_distance.max(1e-6)).min(1.0);
            ((a + d) * 0.5).clamp(0.0, 1.0)
        }
        _ => 0.1,
    }
}

fn dist3(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}
