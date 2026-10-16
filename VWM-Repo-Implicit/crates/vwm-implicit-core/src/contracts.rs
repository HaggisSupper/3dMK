use serde::{Deserialize, Serialize};
use vwm_core::{CanonicalScene, GeometryOrigin};

use crate::{compute_vertex_normals, ImplicitError, ImplicitResult};

pub type Scalar = f64;
pub type Point3 = [Scalar; 3];
pub type Vector3 = [Scalar; 3];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Aabb3 {
    pub min: Point3,
    pub max: Point3,
}

impl Aabb3 {
    pub fn new(min: Point3, max: Point3) -> ImplicitResult<Self> {
        let bounds = Self { min, max };
        bounds.validate()?;
        Ok(bounds)
    }

    pub fn validate(&self) -> ImplicitResult<()> {
        if !self
            .min
            .iter()
            .chain(self.max.iter())
            .all(|v| v.is_finite())
        {
            return Err(ImplicitError::NonFiniteBounds);
        }
        if (0..3).any(|axis| self.min[axis] > self.max[axis]) {
            return Err(ImplicitError::InvalidBounds);
        }
        Ok(())
    }

    pub fn extent(&self) -> Vector3 {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    pub fn center(&self) -> Point3 {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }

    pub fn contains(&self, point: Point3) -> bool {
        (0..3).all(|axis| point[axis] >= self.min[axis] && point[axis] <= self.max[axis])
    }

    pub fn expanded(&self, margin: Scalar) -> ImplicitResult<Self> {
        if !margin.is_finite() || margin < 0.0 {
            return Err(ImplicitError::InvalidBounds);
        }
        Self::new(
            [
                self.min[0] - margin,
                self.min[1] - margin,
                self.min[2] - margin,
            ],
            [
                self.max[0] + margin,
                self.max[1] + margin,
                self.max[2] + margin,
            ],
        )
    }

    pub fn from_points(points: &[Point3]) -> ImplicitResult<Self> {
        if points.is_empty() {
            return Err(ImplicitError::EmptyPointSet);
        }
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for (index, point) in points.iter().enumerate() {
            if !point.iter().all(|v| v.is_finite()) {
                return Err(ImplicitError::NonFinitePoint { index });
            }
            for axis in 0..3 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }
        Self::new(min, max)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrientedPointSet {
    pub points: Vec<Point3>,
    pub normals: Vec<Vector3>,
    pub confidence: Option<Vec<Scalar>>,
    pub source_ids: Option<Vec<u64>>,
}

impl OrientedPointSet {
    pub fn new(points: Vec<Point3>, normals: Vec<Vector3>) -> ImplicitResult<Self> {
        let set = Self {
            points,
            normals,
            confidence: None,
            source_ids: None,
        };
        set.validate()?;
        Ok(set)
    }

    pub fn validate(&self) -> ImplicitResult<()> {
        if self.points.is_empty() {
            return Err(ImplicitError::EmptyPointSet);
        }
        if self.points.len() != self.normals.len() {
            return Err(ImplicitError::PointNormalCountMismatch {
                points: self.points.len(),
                normals: self.normals.len(),
            });
        }
        if let Some(confidence) = &self.confidence {
            if confidence.len() != self.points.len() {
                return Err(ImplicitError::ConfidenceCountMismatch {
                    confidence: confidence.len(),
                    points: self.points.len(),
                });
            }
            if !confidence
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
            {
                return Err(ImplicitError::NonFiniteFieldValue);
            }
        }
        if let Some(source_ids) = &self.source_ids {
            if source_ids.len() != self.points.len() {
                return Err(ImplicitError::Backend(
                    "source ID count does not match point count".to_string(),
                ));
            }
        }
        for (index, (point, normal)) in self.points.iter().zip(&self.normals).enumerate() {
            if !point.iter().chain(normal.iter()).all(|v| v.is_finite()) {
                return Err(ImplicitError::NonFinitePoint { index });
            }
            let norm_sq = normal.iter().map(|v| v * v).sum::<Scalar>();
            if norm_sq <= f64::EPSILON {
                return Err(ImplicitError::ZeroNormal { index });
            }
        }
        Ok(())
    }

    pub fn normalized_normals(&self) -> ImplicitResult<Vec<Vector3>> {
        self.validate()?;
        Ok(self
            .normals
            .iter()
            .map(|normal| {
                let length = normal.iter().map(|v| v * v).sum::<Scalar>().sqrt();
                [normal[0] / length, normal[1] / length, normal[2] / length]
            })
            .collect())
    }

    pub fn bounds(&self) -> ImplicitResult<Aabb3> {
        Aabb3::from_points(&self.points)
    }

    pub fn from_canonical_scene(scene: &CanonicalScene) -> ImplicitResult<Self> {
        if scene.vertices.is_empty() {
            return Err(ImplicitError::EmptyPointSet);
        }
        let normals = match &scene.normals {
            Some(normals) if normals.len() == scene.vertices.len() => normals.clone(),
            Some(normals) => {
                return Err(ImplicitError::PointNormalCountMismatch {
                    points: scene.vertices.len(),
                    normals: normals.len(),
                })
            }
            None => {
                let triangles = scene.indices.as_ref().ok_or_else(|| {
                    ImplicitError::Backend(
                        "canonical scene needs normals or triangle indices for implicit reconstruction"
                            .to_string(),
                    )
                })?;
                compute_vertex_normals(&scene.vertices, triangles)?
            }
        };
        let points = scene
            .vertices
            .iter()
            .map(|point| [point[0] as f64, point[1] as f64, point[2] as f64])
            .collect::<Vec<_>>();
        let normals = normals
            .iter()
            .map(|normal| [normal[0] as f64, normal[1] as f64, normal[2] as f64])
            .collect::<Vec<_>>();
        let mut result = Self::new(points, normals)?;
        result.source_ids = Some((0..scene.vertices.len() as u64).collect());
        Ok(result)
    }
}

pub trait ImplicitField: Send + Sync {
    fn bounds(&self) -> Aabb3;
    fn iso_value(&self) -> Scalar {
        0.0
    }
    fn value(&self, point: Point3) -> ImplicitResult<Scalar>;

    fn gradient(&self, point: Point3) -> ImplicitResult<Vector3> {
        let bounds = self.bounds();
        let extent = bounds.extent();
        let characteristic = extent.into_iter().fold(0.0_f64, f64::max).max(1.0);
        let epsilon = characteristic * 1.0e-5;
        let center_value = self.value(point)?;
        let mut gradient = [0.0; 3];
        for axis in 0..3 {
            let mut minus = point;
            let mut plus = point;
            minus[axis] -= epsilon;
            plus[axis] += epsilon;
            gradient[axis] = if bounds.contains(minus) && bounds.contains(plus) {
                (self.value(plus)? - self.value(minus)?) / (2.0 * epsilon)
            } else if bounds.contains(plus) {
                (self.value(plus)? - center_value) / epsilon
            } else if bounds.contains(minus) {
                (center_value - self.value(minus)?) / epsilon
            } else {
                0.0
            };
        }
        Ok(gradient)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub source_backend: String,
}

impl FieldMesh {
    pub fn validate(&self) -> ImplicitResult<()> {
        if !self
            .positions
            .iter()
            .flatten()
            .all(|value| value.is_finite())
            || !self.normals.iter().flatten().all(|value| value.is_finite())
        {
            return Err(ImplicitError::NonFiniteFieldValue);
        }
        if !self.normals.is_empty() && self.normals.len() != self.positions.len() {
            return Err(ImplicitError::Backend(
                "mesh normal count does not match vertex count".to_string(),
            ));
        }
        let vertex_count = self.positions.len() as u32;
        if self
            .triangles
            .iter()
            .flatten()
            .any(|index| *index >= vertex_count)
        {
            return Err(ImplicitError::InvalidMeshIndex);
        }
        Ok(())
    }

    pub fn ensure_normals(&mut self) -> ImplicitResult<()> {
        if self.normals.len() != self.positions.len() {
            self.normals = compute_vertex_normals(&self.positions, &self.triangles)?;
        }
        Ok(())
    }

    pub fn into_canonical_scene(mut self) -> ImplicitResult<CanonicalScene> {
        self.ensure_normals()?;
        self.validate()?;
        Ok(CanonicalScene {
            vertices: self.positions,
            normals: Some(self.normals),
            indices: Some(self.triangles),
            mesh: true,
            point_cloud: false,
            origin: GeometryOrigin::Generated,
            ..CanonicalScene::default()
        })
    }

    pub fn into_canonical_scene_with_reference(
        mut self,
        reference: &CanonicalScene,
    ) -> ImplicitResult<CanonicalScene> {
        self.ensure_normals()?;
        self.validate()?;
        Ok(CanonicalScene {
            vertices: self.positions,
            normals: Some(self.normals),
            indices: Some(self.triangles),
            mesh: true,
            point_cloud: false,
            units: reference.units,
            transform: reference.transform,
            datum: reference.datum.clone(),
            origin: GeometryOrigin::Generated,
            ..CanonicalScene::default()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldPointCloud {
    pub points: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub confidence: Option<Vec<f32>>,
    pub source_backend: String,
}

impl FieldPointCloud {
    pub fn into_canonical_scene(self) -> ImplicitResult<CanonicalScene> {
        self.validate_attributes()?;
        Ok(CanonicalScene {
            vertices: self.points,
            normals: self.normals,
            mesh: false,
            point_cloud: true,
            origin: GeometryOrigin::Generated,
            ..CanonicalScene::default()
        })
    }

    pub fn into_canonical_scene_with_reference(
        self,
        reference: &CanonicalScene,
    ) -> ImplicitResult<CanonicalScene> {
        self.validate_attributes()?;
        Ok(CanonicalScene {
            vertices: self.points,
            normals: self.normals,
            mesh: false,
            point_cloud: true,
            units: reference.units,
            transform: reference.transform,
            datum: reference.datum.clone(),
            origin: GeometryOrigin::Generated,
            ..CanonicalScene::default()
        })
    }

    fn validate_attributes(&self) -> ImplicitResult<()> {
        if !self.points.iter().flatten().all(|value| value.is_finite()) {
            return Err(ImplicitError::NonFiniteFieldValue);
        }
        if let Some(normals) = &self.normals {
            if !normals.iter().flatten().all(|value| value.is_finite()) {
                return Err(ImplicitError::NonFiniteFieldValue);
            }
            if normals.len() != self.points.len() {
                return Err(ImplicitError::Backend(
                    "point-cloud normal count does not match point count".to_string(),
                ));
            }
        }
        if let Some(confidence) = &self.confidence {
            if confidence.len() != self.points.len() {
                return Err(ImplicitError::ConfidenceCountMismatch {
                    confidence: confidence.len(),
                    points: self.points.len(),
                });
            }
            if !confidence
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
            {
                return Err(ImplicitError::NonFiniteFieldValue);
            }
        }
        Ok(())
    }
}
