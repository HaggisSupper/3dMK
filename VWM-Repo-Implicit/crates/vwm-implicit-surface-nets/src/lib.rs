use fast_surface_nets::ndshape::RuntimeShape;
use fast_surface_nets::{surface_nets, SurfaceNetsBuffer};
use serde::{Deserialize, Serialize};
use vwm_implicit_core::{DenseScalarGrid, FieldMesh, ImplicitError, ImplicitField, ImplicitResult};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SurfaceNetsConfig {
    pub min: [u32; 3],
    pub max: Option<[u32; 3]>,
}

impl Default for SurfaceNetsConfig {
    fn default() -> Self {
        Self {
            min: [0, 0, 0],
            max: None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SurfaceNetsExtractor;

impl SurfaceNetsExtractor {
    pub fn extract(
        &self,
        grid: &DenseScalarGrid,
        config: SurfaceNetsConfig,
    ) -> ImplicitResult<FieldMesh> {
        let dimensions = grid.dimensions();
        let max = config
            .max
            .unwrap_or([dimensions[0] - 1, dimensions[1] - 1, dimensions[2] - 1]);
        for axis in 0..3 {
            if config.min[axis] >= max[axis] || max[axis] >= dimensions[axis] {
                return Err(ImplicitError::Backend(
                    "surface-nets extraction bounds are outside the sampled grid".to_string(),
                ));
            }
        }

        let shape = RuntimeShape::<u32, 3>::new(dimensions);
        let iso = grid.iso_value();
        let sdf = grid
            .values()
            .iter()
            .map(|value| (*value as f64 - iso) as f32)
            .collect::<Vec<_>>();
        let mut output = SurfaceNetsBuffer::default();
        surface_nets(&sdf, &shape, config.min, max, &mut output);

        let origin = grid.origin();
        let spacing = grid.spacing();
        let positions = output
            .positions
            .into_iter()
            .map(|position| {
                [
                    (origin[0] + position[0] as f64 * spacing[0]) as f32,
                    (origin[1] + position[1] as f64 * spacing[1]) as f32,
                    (origin[2] + position[2] as f64 * spacing[2]) as f32,
                ]
            })
            .collect::<Vec<_>>();

        let normals = output
            .normals
            .into_iter()
            .map(|normal| transform_normal(normal, spacing))
            .collect::<Vec<_>>();

        let triangles = output
            .indices
            .chunks_exact(3)
            .map(|triangle| [triangle[0], triangle[1], triangle[2]])
            .collect::<Vec<_>>();

        let mesh = FieldMesh {
            positions,
            normals,
            triangles,
            source_backend: "fast_surface_nets-0.2.1".to_string(),
        };
        mesh.validate()?;
        Ok(mesh)
    }
}

fn transform_normal(normal: [f32; 3], spacing: [f64; 3]) -> [f32; 3] {
    let transformed = [
        normal[0] as f64 / spacing[0],
        normal[1] as f64 / spacing[1],
        normal[2] as f64 / spacing[2],
    ];
    let length = (transformed[0] * transformed[0]
        + transformed[1] * transformed[1]
        + transformed[2] * transformed[2])
        .sqrt();
    if length <= f64::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        [
            (transformed[0] / length) as f32,
            (transformed[1] / length) as f32,
            (transformed[2] / length) as f32,
        ]
    }
}
