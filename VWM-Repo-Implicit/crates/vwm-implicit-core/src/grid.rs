use serde::{Deserialize, Serialize};

use crate::{Aabb3, ImplicitError, ImplicitField, ImplicitResult, Point3, Scalar, Vector3};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DenseScalarGrid {
    origin: Point3,
    spacing: Vector3,
    dimensions: [u32; 3],
    values: Vec<f32>,
    iso_value: Scalar,
}

impl DenseScalarGrid {
    pub fn new(
        origin: Point3,
        spacing: Vector3,
        dimensions: [u32; 3],
        values: Vec<f32>,
        iso_value: Scalar,
    ) -> ImplicitResult<Self> {
        if dimensions.iter().any(|value| *value < 2) {
            return Err(ImplicitError::InvalidGridDimensions);
        }
        if !spacing
            .iter()
            .all(|value| value.is_finite() && *value > 0.0)
        {
            return Err(ImplicitError::InvalidGridSpacing);
        }
        if !origin.iter().all(|value| value.is_finite()) || !iso_value.is_finite() {
            return Err(ImplicitError::NonFiniteFieldValue);
        }
        let expected = checked_grid_len(dimensions)?;
        if values.len() != expected {
            return Err(ImplicitError::GridValueCountMismatch {
                actual: values.len(),
                expected,
            });
        }
        if !values.iter().all(|value| value.is_finite()) {
            return Err(ImplicitError::NonFiniteFieldValue);
        }
        Ok(Self {
            origin,
            spacing,
            dimensions,
            values,
            iso_value,
        })
    }

    pub fn origin(&self) -> Point3 {
        self.origin
    }

    pub fn spacing(&self) -> Vector3 {
        self.spacing
    }

    pub fn dimensions(&self) -> [u32; 3] {
        self.dimensions
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub fn world_position(&self, coordinate: [u32; 3]) -> ImplicitResult<Point3> {
        if (0..3).any(|axis| coordinate[axis] >= self.dimensions[axis]) {
            return Err(ImplicitError::OutsideDomain);
        }
        Ok([
            self.origin[0] + self.spacing[0] * coordinate[0] as f64,
            self.origin[1] + self.spacing[1] * coordinate[1] as f64,
            self.origin[2] + self.spacing[2] * coordinate[2] as f64,
        ])
    }

    pub fn value_at(&self, coordinate: [u32; 3]) -> ImplicitResult<f32> {
        let index = self.index(coordinate)?;
        Ok(self.values[index])
    }

    pub fn index(&self, coordinate: [u32; 3]) -> ImplicitResult<usize> {
        if (0..3).any(|axis| coordinate[axis] >= self.dimensions[axis]) {
            return Err(ImplicitError::OutsideDomain);
        }
        let x = coordinate[0] as usize;
        let y = coordinate[1] as usize;
        let z = coordinate[2] as usize;
        let width = self.dimensions[0] as usize;
        let height = self.dimensions[1] as usize;
        Ok(x + width * (y + height * z))
    }
}

impl ImplicitField for DenseScalarGrid {
    fn bounds(&self) -> Aabb3 {
        let max = [
            self.origin[0] + self.spacing[0] * (self.dimensions[0] - 1) as f64,
            self.origin[1] + self.spacing[1] * (self.dimensions[1] - 1) as f64,
            self.origin[2] + self.spacing[2] * (self.dimensions[2] - 1) as f64,
        ];
        Aabb3 {
            min: self.origin,
            max,
        }
    }

    fn iso_value(&self) -> Scalar {
        self.iso_value
    }

    fn value(&self, point: Point3) -> ImplicitResult<Scalar> {
        if !self.bounds().contains(point) {
            return Err(ImplicitError::OutsideDomain);
        }
        let mut base = [0_u32; 3];
        let mut fraction = [0.0; 3];
        for axis in 0..3 {
            let coordinate = (point[axis] - self.origin[axis]) / self.spacing[axis];
            let max_base = self.dimensions[axis] - 2;
            let floored = coordinate.floor().max(0.0) as u32;
            base[axis] = floored.min(max_base);
            fraction[axis] = (coordinate - base[axis] as f64).clamp(0.0, 1.0);
        }

        let mut result = 0.0;
        for dz in 0..=1_u32 {
            for dy in 0..=1_u32 {
                for dx in 0..=1_u32 {
                    let weight_x = if dx == 0 {
                        1.0 - fraction[0]
                    } else {
                        fraction[0]
                    };
                    let weight_y = if dy == 0 {
                        1.0 - fraction[1]
                    } else {
                        fraction[1]
                    };
                    let weight_z = if dz == 0 {
                        1.0 - fraction[2]
                    } else {
                        fraction[2]
                    };
                    let value = self.value_at([base[0] + dx, base[1] + dy, base[2] + dz])?;
                    result += value as f64 * weight_x * weight_y * weight_z;
                }
            }
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GridSamplingConfig {
    pub bounds: Aabb3,
    pub dimensions: [u32; 3],
}

impl GridSamplingConfig {
    pub fn validate(&self) -> ImplicitResult<()> {
        self.bounds.validate()?;
        if self.dimensions.iter().any(|value| *value < 2) {
            return Err(ImplicitError::InvalidGridDimensions);
        }
        checked_grid_len(self.dimensions)?;
        Ok(())
    }
}

pub fn sample_field_to_grid(
    field: &dyn ImplicitField,
    config: GridSamplingConfig,
) -> ImplicitResult<DenseScalarGrid> {
    config.validate()?;
    let extent = config.bounds.extent();
    let spacing = [
        extent[0] / (config.dimensions[0] - 1) as f64,
        extent[1] / (config.dimensions[1] - 1) as f64,
        extent[2] / (config.dimensions[2] - 1) as f64,
    ];
    if !spacing
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
    {
        return Err(ImplicitError::InvalidGridSpacing);
    }
    let mut values = Vec::with_capacity(checked_grid_len(config.dimensions)?);
    for z in 0..config.dimensions[2] {
        for y in 0..config.dimensions[1] {
            for x in 0..config.dimensions[0] {
                let point = [
                    config.bounds.min[0] + spacing[0] * x as f64,
                    config.bounds.min[1] + spacing[1] * y as f64,
                    config.bounds.min[2] + spacing[2] * z as f64,
                ];
                let value = field.value(point)?;
                if !value.is_finite() {
                    return Err(ImplicitError::NonFiniteFieldValue);
                }
                values.push(value as f32);
            }
        }
    }
    DenseScalarGrid::new(
        config.bounds.min,
        spacing,
        config.dimensions,
        values,
        field.iso_value(),
    )
}

fn checked_grid_len(dimensions: [u32; 3]) -> ImplicitResult<usize> {
    dimensions
        .into_iter()
        .try_fold(1_usize, |acc, value| acc.checked_mul(value as usize))
        .ok_or(ImplicitError::GridSizeOverflow)
}
