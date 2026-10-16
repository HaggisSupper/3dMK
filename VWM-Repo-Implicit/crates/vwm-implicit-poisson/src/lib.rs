use nalgebra::{Point3 as NaPoint3, Vector3 as NaVector3};
use poisson_reconstruction::PoissonReconstruction;
use serde::{Deserialize, Serialize};
use vwm_implicit_core::{
    compute_vertex_normals, Aabb3, FieldMesh, ImplicitError, ImplicitField, ImplicitResult,
    OrientedPointSet, Point3, Scalar, Vector3,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PoissonConfig {
    pub screening: Scalar,
    pub density_estimation_depth: usize,
    pub max_depth: usize,
    pub max_relaxation_iterations: usize,
}

impl PoissonConfig {
    pub fn validate(&self) -> ImplicitResult<()> {
        if !self.screening.is_finite()
            || self.screening < 0.0
            || self.max_depth < 2
            || self.density_estimation_depth > self.max_depth
            || self.max_relaxation_iterations == 0
        {
            return Err(ImplicitError::Backend(
                "invalid Screened Poisson configuration".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for PoissonConfig {
    fn default() -> Self {
        Self {
            screening: 1.0,
            density_estimation_depth: 5,
            max_depth: 8,
            max_relaxation_iterations: 10,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PoissonReconstructor;

pub struct PoissonField {
    reconstruction: PoissonReconstruction,
    bounds: Aabb3,
    config: PoissonConfig,
}

impl PoissonReconstructor {
    pub fn reconstruct(
        &self,
        input: &OrientedPointSet,
        config: PoissonConfig,
    ) -> ImplicitResult<PoissonField> {
        input.validate()?;
        config.validate()?;
        if input.points.len() < 4 {
            return Err(ImplicitError::Backend(
                "Screened Poisson reconstruction requires at least four oriented points"
                    .to_string(),
            ));
        }
        let normals = input.normalized_normals()?;
        let points = input
            .points
            .iter()
            .map(|point| NaPoint3::new(point[0], point[1], point[2]))
            .collect::<Vec<_>>();
        let normals = normals
            .iter()
            .map(|normal| NaVector3::new(normal[0], normal[1], normal[2]))
            .collect::<Vec<_>>();

        let reconstruction = PoissonReconstruction::from_points_and_normals(
            &points,
            &normals,
            config.screening,
            config.density_estimation_depth,
            config.max_depth,
            config.max_relaxation_iterations,
        );
        let source_bounds = reconstruction.aabb();
        let bounds = Aabb3::new(
            [
                source_bounds.mins.x,
                source_bounds.mins.y,
                source_bounds.mins.z,
            ],
            [
                source_bounds.maxs.x,
                source_bounds.maxs.y,
                source_bounds.maxs.z,
            ],
        )?;
        Ok(PoissonField {
            reconstruction,
            bounds,
            config,
        })
    }
}

impl PoissonField {
    pub fn config(&self) -> PoissonConfig {
        self.config
    }

    pub fn reconstruct_mesh(&self) -> ImplicitResult<FieldMesh> {
        let buffers = self.reconstruction.reconstruct_mesh_buffers();
        let positions = buffers
            .vertices()
            .iter()
            .map(|point| [point.x as f32, point.y as f32, point.z as f32])
            .collect::<Vec<_>>();
        let triangles = buffers
            .indices()
            .chunks_exact(3)
            .map(|triangle| [triangle[0], triangle[1], triangle[2]])
            .collect::<Vec<_>>();
        let normals = compute_vertex_normals(&positions, &triangles)?;
        let mesh = FieldMesh {
            positions,
            normals,
            triangles,
            source_backend: "poisson_reconstruction-0.4.0".to_string(),
        };
        mesh.validate()?;
        Ok(mesh)
    }
}

impl ImplicitField for PoissonField {
    fn bounds(&self) -> Aabb3 {
        self.bounds
    }

    fn value(&self, point: Point3) -> ImplicitResult<Scalar> {
        if !self.bounds.contains(point) {
            return Err(ImplicitError::OutsideDomain);
        }
        let value = self
            .reconstruction
            .eval(&NaPoint3::new(point[0], point[1], point[2]));
        if value.is_finite() {
            Ok(value)
        } else {
            Err(ImplicitError::NonFiniteFieldValue)
        }
    }

    fn gradient(&self, point: Point3) -> ImplicitResult<Vector3> {
        if !self.bounds.contains(point) {
            return Err(ImplicitError::OutsideDomain);
        }
        let gradient = self
            .reconstruction
            .eval_gradient(&NaPoint3::new(point[0], point[1], point[2]));
        if gradient.x.is_finite() && gradient.y.is_finite() && gradient.z.is_finite() {
            Ok([gradient.x, gradient.y, gradient.z])
        } else {
            Err(ImplicitError::NonFiniteFieldValue)
        }
    }
}
