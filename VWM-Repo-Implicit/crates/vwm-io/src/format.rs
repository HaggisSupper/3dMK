use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneFormat {
    Gltf,
    Obj,
    Ply,
    Stl,
    Las,
    Unknown,
}

pub fn detect_format(path: &Path) -> SceneFormat {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
    {
        Some(ext) if ext == "glb" || ext == "gltf" => SceneFormat::Gltf,
        Some(ext) if ext == "obj" => SceneFormat::Obj,
        Some(ext) if ext == "ply" => SceneFormat::Ply,
        Some(ext) if ext == "stl" => SceneFormat::Stl,
        Some(ext) if ext == "las" || ext == "laz" => SceneFormat::Las,
        _ => SceneFormat::Unknown,
    }
}
