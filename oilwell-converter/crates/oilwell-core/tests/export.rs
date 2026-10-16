use oilwell_core::{export_mesh, Mesh, OutputFormat};

fn triangle() -> Mesh {
    Mesh::new(
        "Triangle",
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0.0, 0.0, 1.0]; 3],
        vec![0, 1, 2],
    )
}

#[test]
fn writes_valid_geometry_signatures_for_all_formats() {
    assert_eq!(
        &export_mesh(&[triangle()], OutputFormat::Glb).unwrap()[..4],
        b"glTF"
    );
    assert!(
        String::from_utf8(export_mesh(&[triangle()], OutputFormat::Obj).unwrap())
            .unwrap()
            .contains("f 1//1 2//2 3//3")
    );
    assert!(
        String::from_utf8(export_mesh(&[triangle()], OutputFormat::Ply).unwrap())
            .unwrap()
            .starts_with("ply\nformat ascii 1.0")
    );
    let stl = export_mesh(&[triangle()], OutputFormat::Stl).unwrap();
    assert_eq!(stl.len(), 134);
    assert_eq!(u32::from_le_bytes(stl[80..84].try_into().unwrap()), 1);
}
