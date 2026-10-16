use serde::{Deserialize, Serialize};

use crate::{Aabb3, ImplicitField, ImplicitResult, Point3, Scalar, Vector3};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SphereField {
    pub center: Point3,
    pub radius: Scalar,
    pub domain: Aabb3,
}

impl SphereField {
    pub fn new(center: Point3, radius: Scalar, padding: Scalar) -> ImplicitResult<Self> {
        if !radius.is_finite() || radius <= 0.0 || !padding.is_finite() || padding < 0.0 {
            return Err(crate::ImplicitError::InvalidBounds);
        }
        let extent = radius + padding;
        let domain = Aabb3::new(
            [center[0] - extent, center[1] - extent, center[2] - extent],
            [center[0] + extent, center[1] + extent, center[2] + extent],
        )?;
        Ok(Self {
            center,
            radius,
            domain,
        })
    }
}

impl ImplicitField for SphereField {
    fn bounds(&self) -> Aabb3 {
        self.domain
    }

    fn value(&self, point: Point3) -> ImplicitResult<Scalar> {
        let delta = [
            point[0] - self.center[0],
            point[1] - self.center[1],
            point[2] - self.center[2],
        ];
        Ok((delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt() - self.radius)
    }

    fn gradient(&self, point: Point3) -> ImplicitResult<Vector3> {
        let delta = [
            point[0] - self.center[0],
            point[1] - self.center[1],
            point[2] - self.center[2],
        ];
        let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        if length <= f64::EPSILON {
            Ok([0.0, 1.0, 0.0])
        } else {
            Ok([delta[0] / length, delta[1] / length, delta[2] / length])
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PlaneField {
    pub point: Point3,
    pub normal: Vector3,
    pub domain: Aabb3,
}

impl PlaneField {
    pub fn new(point: Point3, normal: Vector3, domain: Aabb3) -> ImplicitResult<Self> {
        domain.validate()?;
        let length = normal
            .iter()
            .map(|value| value * value)
            .sum::<Scalar>()
            .sqrt();
        if !length.is_finite() || length <= f64::EPSILON {
            return Err(crate::ImplicitError::InvalidRayDirection);
        }
        Ok(Self {
            point,
            normal: [normal[0] / length, normal[1] / length, normal[2] / length],
            domain,
        })
    }
}

impl ImplicitField for PlaneField {
    fn bounds(&self) -> Aabb3 {
        self.domain
    }

    fn value(&self, point: Point3) -> ImplicitResult<Scalar> {
        Ok((point[0] - self.point[0]) * self.normal[0]
            + (point[1] - self.point[1]) * self.normal[1]
            + (point[2] - self.point[2]) * self.normal[2])
    }

    fn gradient(&self, _point: Point3) -> ImplicitResult<Vector3> {
        Ok(self.normal)
    }
}
