use std::path::Path;

use anyhow::Result;

use crate::convert::convert_to_glb;
use crate::format::{detect_format, SceneFormat};
use crate::loaders::{load_gltf, load_las, load_obj, load_ply, load_stl};
use vwm_core::CanonicalScene;

pub fn load_scene(path: &Path) -> Result<CanonicalScene> {
    match detect_format(path) {
        SceneFormat::Gltf => load_gltf(path),
        SceneFormat::Obj => load_obj(path),
        SceneFormat::Ply => load_ply(path),
        SceneFormat::Stl => load_stl(path),
        SceneFormat::Las => load_las(path),
        SceneFormat::Convertible | SceneFormat::Unknown => {
            let converted = convert_to_glb(path)?;
            load_gltf(&converted)
        }
    }
}
