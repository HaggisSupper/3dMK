use agentic_cad_backend::{api::create_router, scene_document::*};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

async fn upload(app: &Router, url: &str, bytes: &[u8]) -> Value {
    let mut body=b"--fixture\r\nContent-Disposition: form-data; name=\"file\"; filename=\"same.glb\"\r\nContent-Type: model/gltf-binary\r\n\r\n".to_vec();
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n--fixture--\r\n");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(url)
                .header("content-type", "multipart/form-data; boundary=fixture")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let data = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    assert!(
        status.is_success(),
        "{status}: {}",
        String::from_utf8_lossy(&data)
    );
    serde_json::from_slice(&data).unwrap()
}
async fn command(app: &Router, url: &str, request: &SceneCommandRequest) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(url)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let data = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&data).unwrap())
}

#[tokio::test]
async fn two_glbs_compose_with_independent_roots_hide_reopen_and_invalid_node_rejection() {
    let root = std::env::temp_dir().join(format!("3dmk-multi-glb-{}", Uuid::new_v4()));
    std::fs::create_dir_all(root.join("output")).unwrap();
    let app = create_router(
        root.join("output"),
        root.join("public"),
        root.join("data"),
        0,
    )
    .unwrap();
    let bytes = include_bytes!("fixtures/scene/planned-actual-section.glb");
    let imported = upload(&app, "/api/v1/projects/import", bytes).await;
    let project = imported["project"]["project_id"].as_str().unwrap();
    let source_one: Uuid =
        serde_json::from_value(imported["source_asset"]["asset_id"].clone()).unwrap();
    let second = upload(&app, &format!("/api/v1/projects/{project}/assets"), bytes).await;
    let source_two: Uuid = serde_json::from_value(second["asset_id"].clone()).unwrap();
    let url = format!("/api/v1/projects/{project}/scene");
    let one = SceneObject::group("same.glb");
    let two = SceneObject::group("same.glb (2)");
    let mut first_node = SceneObject::instance("Planned", source_one);
    first_node.parent_id = Some(one.object_id);
    first_node.asset_node_index = Some(0);
    let mut second_node = SceneObject::instance("Actual", source_two);
    second_node.parent_id = Some(two.object_id);
    second_node.asset_node_index = Some(1);
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::AddObjects {
            objects: vec![
                first_node.clone(),
                two.clone(),
                second_node.clone(),
                one.clone(),
            ],
        },
    };
    let (status, result) = command(&app, &url, &request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["document"]["objects"].as_object().unwrap().len(), 4);
    assert_eq!(command(&app, &url, &request).await.1["duplicate"], true);
    let hide = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 1,
        command: SceneCommand::SetDisplay {
            object_id: one.object_id,
            visible: false,
            opacity: 1.0,
        },
    };
    assert_eq!(command(&app, &url, &hide).await.0, StatusCode::OK);
    let store = agentic_cad_backend::projects::ProjectStore::open(root.join("data")).unwrap();
    let document = store
        .get_scene_document(Uuid::parse_str(project).unwrap())
        .unwrap()
        .document;
    assert!(!document.objects[&one.object_id].visible);
    assert!(document.objects[&two.object_id].visible);
    assert!(
        document.objects[&first_node.object_id].visible,
        "hidden parent does not erase child visibility"
    );
    // The user-facing Save project path must retain both immutable sources,
    // source-node selectors and independent root visibility on another store.
    let project_id = Uuid::parse_str(project).unwrap();
    let package_path = root.join("two-files.zip");
    store.export_package(project_id, &package_path).unwrap();
    let prepared = agentic_cad_backend::packages::prepare_archive(
        &package_path,
        &root.join("staging"),
        None,
        agentic_cad_backend::packages::ArchiveLimits::default(),
    )
    .unwrap();
    let manifest_asset = prepared
        .assets
        .iter()
        .find(|asset| {
            asset
                .package_paths
                .iter()
                .any(|path| path == "3dmk-package.json")
        })
        .unwrap();
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(&manifest_asset.source_path).unwrap()).unwrap();
    let restored =
        agentic_cad_backend::projects::ProjectStore::open(root.join("restored")).unwrap();
    restored
        .restore_package_scene(&manifest, &prepared.assets)
        .unwrap();
    assert_eq!(
        restored.get_scene_document(project_id).unwrap().document,
        document
    );
    assert_eq!(
        restored
            .get_scene_document(project_id)
            .unwrap()
            .document
            .objects[&first_node.object_id]
            .asset_node_index,
        Some(0)
    );
    assert_eq!(
        restored
            .get_scene_document(project_id)
            .unwrap()
            .document
            .objects[&second_node.object_id]
            .asset_node_index,
        Some(1)
    );
    let mut invalid = SceneObject::instance("Missing node", source_one);
    invalid.asset_node_index = Some(9999);
    let bad = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 2,
        command: SceneCommand::AddObject { object: invalid },
    };
    assert_eq!(
        command(&app, &url, &bad).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        store
            .get_scene_document(Uuid::parse_str(project).unwrap())
            .unwrap()
            .document,
        document
    );
    let undo = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 2,
        command: SceneCommand::Undo {},
    };
    assert_eq!(command(&app, &url, &undo).await.0, StatusCode::OK);
    assert!(
        store
            .get_scene_document(Uuid::parse_str(project).unwrap())
            .unwrap()
            .document
            .objects[&one.object_id]
            .visible
    );
    drop(app);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
