use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LengthUnit {
    Unknown,
    Meters,
    Millimeters,
    Centimeters,
    Inches,
    Feet,
}

impl Default for LengthUnit {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DatumSource {
    Unknown,
    ScanFrameFallback,
    UserDefined,
    Metadata,
    EstimatedFromStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDatum {
    pub origin: [f64; 3],
    pub rotation_xyzw: [f64; 4],
    pub units: LengthUnit,
    pub gravity_axis: [f64; 3],
    pub source: DatumSource,
    pub confidence: f64,
}

impl Default for SceneDatum {
    fn default() -> Self {
        Self {
            origin: [0.0, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            units: LengthUnit::Unknown,
            gravity_axis: [0.0, 1.0, 0.0],
            source: DatumSource::ScanFrameFallback,
            confidence: 0.1,
        }
    }
}
