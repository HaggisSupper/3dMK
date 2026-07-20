use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use vwm_io::load_scene;

fn temporary_file(extension: &str, contents: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "vwm-io-{}-{unique}.{extension}",
        std::process::id()
    ));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn standard_ply_int_polygon_faces_are_triangulated() {
    let path = temporary_file(
        "ply",
        "ply\n\
         format ascii 1.0\n\
         element vertex 4\n\
         property float x\n\
         property float y\n\
         property float z\n\
         element face 1\n\
         property list uchar int vertex_indices\n\
         end_header\n\
         0 0 0\n\
         1 0 0\n\
         1 1 0\n\
         0 1 0\n\
         4 0 1 2 3\n",
    );

    let scene = load_scene(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert!(scene.mesh);
    assert_eq!(scene.indices.unwrap(), vec![[0, 1, 2], [0, 2, 3]]);
}

#[test]
fn ply_missing_coordinate_is_rejected_instead_of_invented() {
    let path = temporary_file(
        "ply",
        "ply\n\
         format ascii 1.0\n\
         element vertex 1\n\
         property float x\n\
         property float y\n\
         end_header\n\
         1 2\n",
    );

    let error = load_scene(&path).unwrap_err();
    std::fs::remove_file(path).unwrap();

    assert!(error.to_string().contains("missing numeric z"));
}

#[test]
fn ply_float_colors_remain_normalized() {
    let path = temporary_file(
        "ply",
        "ply\n\
         format ascii 1.0\n\
         element vertex 1\n\
         property float x\n\
         property float y\n\
         property float z\n\
         property float red\n\
         property float green\n\
         property float blue\n\
         end_header\n\
         0 0 0 1 0.5 0\n",
    );

    let scene = load_scene(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(scene.colors.unwrap(), vec![[1.0, 0.5, 0.0]]);
}

#[test]
fn gltf_node_transform_is_applied_to_geometry() {
    let path = temporary_file(
        "gltf",
        r#"{
          "asset": {"version": "2.0"},
          "buffers": [{"byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAABAAIA"}],
          "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": 36, "target": 34962},
            {"buffer": 0, "byteOffset": 36, "byteLength": 6, "target": 34963}
          ],
          "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0]},
            {"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}
          ],
          "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
          "nodes": [{"mesh": 0, "translation": [10, 0, 0]}],
          "scenes": [{"nodes": [0]}],
          "scene": 0
        }"#,
    );

    let scene = load_scene(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(
        scene.vertices,
        vec![[10.0, 0.0, 0.0], [11.0, 0.0, 0.0], [10.0, 1.0, 0.0]]
    );
    assert_eq!(scene.indices.unwrap(), vec![[0, 1, 2]]);
}
