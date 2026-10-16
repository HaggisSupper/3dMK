use serde::{Deserialize, Serialize};

use crate::contracts::{checked_pixel_count, BoundingBox2D};
use crate::{PerceptionError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BinaryMask {
    pub width: u32,
    pub height: u32,
    /// Row-major values. Zero is background; non-zero is foreground.
    pub data: Vec<u8>,
}

impl BinaryMask {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self> {
        let expected = checked_pixel_count(width, height)?;
        if data.len() != expected {
            return Err(PerceptionError::InvalidMaskBuffer {
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            data: data.into_iter().map(|v| u8::from(v != 0)).collect(),
        })
    }

    pub fn filled(width: u32, height: u32, value: bool) -> Result<Self> {
        Self::new(
            width,
            height,
            vec![u8::from(value); checked_pixel_count(width, height)?],
        )
    }

    pub fn from_predicate(
        width: u32,
        height: u32,
        mut predicate: impl FnMut(u32, u32) -> bool,
    ) -> Result<Self> {
        let mut data = Vec::with_capacity(checked_pixel_count(width, height)?);
        for y in 0..height {
            for x in 0..width {
                data.push(u8::from(predicate(x, y)));
            }
        }
        Self::new(width, height, data)
    }

    pub fn validate(&self) -> Result<()> {
        let expected = checked_pixel_count(self.width, self.height)?;
        if self.data.len() != expected {
            return Err(PerceptionError::InvalidMaskBuffer {
                expected,
                actual: self.data.len(),
            });
        }
        Ok(())
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.data[(y * self.width + x) as usize] != 0
    }

    pub fn foreground_count(&self) -> usize {
        self.data.iter().filter(|&&v| v != 0).count()
    }

    pub fn bounding_box(&self) -> Option<BoundingBox2D> {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        let mut found = false;

        for y in 0..self.height {
            for x in 0..self.width {
                if self.contains(x, y) {
                    found = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        found.then_some(BoundingBox2D {
            x: min_x,
            y: min_y,
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
        })
    }

    pub fn intersection_over_union(&self, other: &Self) -> Result<f32> {
        if self.width != other.width || self.height != other.height {
            return Err(PerceptionError::ProjectionDimensionMismatch);
        }
        let mut intersection = 0usize;
        let mut union = 0usize;
        for (&a, &b) in self.data.iter().zip(&other.data) {
            let aa = a != 0;
            let bb = b != 0;
            intersection += if aa && bb { 1 } else { 0 };
            union += if aa || bb { 1 } else { 0 };
        }
        Ok(if union == 0 {
            1.0
        } else {
            intersection as f32 / union as f32
        })
    }
}
