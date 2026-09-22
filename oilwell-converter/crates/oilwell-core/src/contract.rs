use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Glb,
    Obj,
    Ply,
    Stl,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InterpolationMethod {
    MinimumCurvature,
    LinearDirectionBlend,
}

pub type ColourPalette = BTreeMap<String, [u8; 3]>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionRequest {
    pub survey_path: String,
    pub sections_path: String,
    pub bha_path: Option<String>,
    pub formation_path: Option<String>,
    pub output_path: String,
    pub output_format: OutputFormat,
    pub diameter_scale: f64,
    pub smooth_step_md: f64,
    pub interpolation_method: InterpolationMethod,
    pub colour_palette: ColourPalette,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub output_path: String,
    pub wells: usize,
    pub formations: usize,
    pub meshes: usize,
    pub warnings: Vec<String>,
}
