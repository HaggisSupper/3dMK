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

fn triangle_document() -> serde_json::Value {
    serde_json::json!({
        "asset":{"version":"2.0"},
        "buffers":[{"byteLength":42,"uri":"data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAABAAIA"}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36,"target":34962},{"buffer":0,"byteOffset":36,"byteLength":6,"target":34963}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]},{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1,"material":0}]}],
        "materials":[{"name":"Well steel","pbrMetallicRoughness":{"baseColorFactor":[0.2,0.3,0.4,1.0],"metallicFactor":0.7}}],
        "nodes":[{"translation":[10,0,0],"children":[1,2],"extras":{"id":"well-A","kind":"well"}},{"mesh":0,"translation":[0,2,0]},{"mesh":0,"translation":[0,5,0]}],
        "scenes":[{"nodes":[0]}],"scene":0,"extras":{"mavisExchange":{"schema":"mavis.glb-exchange/1"}}
    })
}
fn load_document(document: serde_json::Value) -> vwm_core::CanonicalScene {
    let path = temporary_file("gltf", &document.to_string());
    let result = load_scene(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    result
}
#[test]
fn hierarchy_instances_materials_and_extras_survive() {
    let scene = load_document(triangle_document());
    assert_eq!(scene.vertices.len(), 6);
    assert_eq!(scene.vertices[0], [10., 2., 0.]);
    assert_eq!(scene.vertices[3], [10., 5., 0.]);
    assert_eq!(scene.indices.as_ref().unwrap(), &[[0, 1, 2], [3, 4, 5]]);
    assert_eq!(scene.material_ids.as_ref().unwrap(), &[0, 0]);
    assert_eq!(scene.primitives.len(), 2);
    let json = scene.source_document.as_ref().unwrap();
    assert_eq!(json["nodes"][0]["extras"]["id"], "well-A");
    assert_eq!(json["materials"][0]["name"], "Well steel");
    assert_eq!(
        json["extras"]["mavisExchange"]["schema"],
        "mavis.glb-exchange/1"
    );
    let encoded = serde_json::to_vec(&scene).unwrap();
    let reopened: vwm_core::CanonicalScene = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(reopened.source_document, scene.source_document);
}
#[test]
fn nonindexed_triangles_and_mirrored_winding_are_explicit() {
    let mut doc = triangle_document();
    doc["meshes"][0]["primitives"][0]
        .as_object_mut()
        .unwrap()
        .remove("indices");
    doc["nodes"][0]["scale"] = serde_json::json!([-1, 1, 1]);
    let scene = load_document(doc);
    assert_eq!(scene.indices.unwrap(), [[0, 2, 1], [3, 5, 4]]);
}
#[test]
fn line_primitives_are_not_silently_regrouped_as_triangles() {
    let mut doc = triangle_document();
    doc["meshes"][0]["primitives"][0]["mode"] = serde_json::json!(3);
    let path = temporary_file("gltf", &doc.to_string());
    assert!(load_scene(&path)
        .unwrap_err()
        .to_string()
        .contains("line primitives"));
    std::fs::remove_file(path).unwrap();
}
#[test]
fn large_translation_is_rebased_before_float_storage() {
    let mut doc = triangle_document();
    doc["nodes"][0]["translation"] = serde_json::json!([6000000, 5000000, 0]);
    doc["nodes"][1]["translation"] = serde_json::json!([0.01, 0, 0]);
    doc["nodes"][2]["translation"] = serde_json::json!([0.02, 0, 0]);
    let scene = load_document(doc);
    assert!((scene.vertices[3][0] - 0.01).abs() < 1e-6);
    assert!((scene.datum.origin[0] - 6000000.01).abs() < 1e-6);
}
#[test]
fn las_centimetres_at_map_coordinates_survive() {
    use las::{Point, Write, Writer};
    let path = temporary_file("las", "");
    let mut builder = las::Builder::default();
    builder.transforms.x.offset = 6_000_000.;
    let mut writer = Writer::from_path(&path, builder.into_header().unwrap()).unwrap();
    writer
        .write(Point {
            x: 6_000_000.0,
            y: 500_000.0,
            z: 100.,
            ..Default::default()
        })
        .unwrap();
    writer
        .write(Point {
            x: 6_000_000.01,
            y: 500_000.0,
            z: 100.,
            ..Default::default()
        })
        .unwrap();
    writer.close().unwrap();
    let scene = load_scene(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert!((scene.vertices[1][0] - 0.01).abs() < 1e-6);
    assert_eq!(scene.datum.origin, [6_000_000., 500_000., 100.]);
}

#[test]
fn single_large_json_translation_preserves_centimetres() {
    let mut doc = triangle_document();
    doc["nodes"][0]["translation"] = serde_json::json!([6000000.01, 0, 0]);
    let scene = load_document(doc);
    assert!((scene.datum.origin[0] - 6000000.01).abs() < 1e-8);
    assert!(
        (scene.source_document.unwrap()["nodes"][0]["translation"][0]
            .as_f64()
            .unwrap()
            - 6000000.01)
            .abs()
            < 1e-8
    );
}
