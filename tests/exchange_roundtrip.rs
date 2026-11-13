use agentic_cad_backend::{
    packages::{self, ArchiveLimits},
    projects::*,
};
use std::{fs, io::Cursor};
use uuid::Uuid;

#[test]
fn saved_package_restores_project_revision_and_asset_identity() {
    let root = std::env::temp_dir().join(format!("3dmk-roundtrip-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let store = ProjectStore::open(root.join("first")).unwrap();
    let fixture = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/models/BoxTextured.glb"
    ))
    .unwrap();
    let (project, asset, revision) = store
        .import_root_scene(
            RootSceneImport {
                project_name: "Textured source".into(),
                original_name: "BoxTextured.glb".into(),
                media_type: "model/gltf-binary".into(),
                scene_kind: SceneKind::Mesh,
                attributes: AttributeContract {
                    uvs: true,
                    ..Default::default()
                },
                original_units: Some("m".into()),
                warnings: vec![],
            },
            Cursor::new(fixture.clone()),
        )
        .unwrap();
    let zip = root.join("project.zip");
    store.export_package(project.project_id, &zip).unwrap();
    let prepared =
        packages::prepare_archive(&zip, &root.join("staging"), None, ArchiveLimits::default())
            .unwrap();
    let manifest_asset = prepared
        .assets
        .iter()
        .find(|a| a.package_paths.iter().any(|p| p == "3dmk-package.json"))
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_asset.source_path).unwrap()).unwrap();
    let reopened = ProjectStore::open(root.join("second")).unwrap();
    let result = reopened
        .restore_package_scene(&manifest, &prepared.assets)
        .unwrap();
    assert_eq!(result.project, project);
    assert_eq!(result.root_revision, revision);
    assert_eq!(result.assets.as_slice(), std::slice::from_ref(&asset));
    let (_, payload) = reopened
        .asset_payload(project.project_id, asset.asset_id)
        .unwrap();
    assert_eq!(fs::read(payload).unwrap(), fixture);
    // Reopening the same package is idempotent, never creates a duplicate project.
    let second = reopened
        .restore_package_scene(&manifest, &prepared.assets)
        .unwrap();
    assert_eq!(second.project.project_id, project.project_id);
    assert_eq!(reopened.list_projects().unwrap().len(), 1);
    let mut corrupted = manifest.clone();
    corrupted["assets"][0]["sha256"] = serde_json::json!("0".repeat(64));
    let third = ProjectStore::open(root.join("third")).unwrap();
    assert!(third
        .restore_package_scene(&corrupted, &prepared.assets)
        .is_err());
    assert!(third.list_projects().unwrap().is_empty());
    drop(prepared);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn cancellation_cannot_be_reported_complete() {
    use agentic_cad_backend::jobs::JobRegistry;
    let root = std::env::temp_dir().join(format!("3dmk-cancel-{}", Uuid::new_v4()));
    let store = ProjectStore::open(&root).unwrap();
    let project = store.create_project("Cancel").unwrap();
    let jobs = JobRegistry::open(store, 1).unwrap();
    let job = jobs
        .create(project.project_id, JobKind::Prepare, "result_discard")
        .unwrap();
    jobs.transition(job.job_id, JobState::Running, "working", None, None)
        .unwrap();
    jobs.request_cancel(job.job_id).unwrap();
    assert!(jobs
        .transition(job.job_id, JobState::Complete, "complete", Some(1.), None)
        .is_err());
    assert_eq!(
        jobs.get(job.job_id).unwrap().state,
        JobState::CancellationRequested
    );
    jobs.transition(job.job_id, JobState::Cancelled, "discarded", None, None)
        .unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scene_layout_history_survives_package_restore_and_conflicts_fail_closed() {
    use agentic_cad_backend::scene_document::*;
    let (root, store, project, _, _, old_prepared) = child_package();
    drop(old_prepared);
    let asset = store.list_assets(project.project_id).unwrap()[0].clone();
    let object = SceneObject::instance("Preserved instance", asset.asset_id);
    let id = object.object_id;
    let add = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::AddObject { object },
    };
    store.apply_scene_command(project.project_id, &add).unwrap();
    let transform = LocalTransform {
        translation: [123.25, -150.0, 5.0],
        ..LocalTransform::default()
    };
    let edit = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 1,
        command: SceneCommand::SetTransform {
            object_id: id,
            transform,
        },
    };
    let before = store
        .apply_scene_command(project.project_id, &edit)
        .unwrap();
    let zip = root.join("scene-layout.zip");
    store.export_package(project.project_id, &zip).unwrap();
    let prepared = packages::prepare_archive(
        &zip,
        &root.join("scene-staging"),
        None,
        ArchiveLimits::default(),
    )
    .unwrap();
    let manifest_asset = prepared
        .assets
        .iter()
        .find(|a| a.package_paths.iter().any(|p| p == "3dmk-package.json"))
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_asset.source_path).unwrap()).unwrap();
    let reopened = ProjectStore::open(root.join("scene-reopened")).unwrap();
    reopened
        .restore_package_scene(&manifest, &prepared.assets)
        .unwrap();
    assert_eq!(
        before.document,
        reopened
            .get_scene_document(project.project_id)
            .unwrap()
            .document
    );
    assert!(
        reopened
            .apply_scene_command(project.project_id, &edit)
            .unwrap()
            .duplicate
    );
    let undone = reopened
        .apply_scene_command(
            project.project_id,
            &SceneCommandRequest {
                command_id: Uuid::new_v4(),
                expected_revision: 2,
                command: SceneCommand::Undo {},
            },
        )
        .unwrap();
    assert_eq!(
        undone.document.objects[&id].transform,
        LocalTransform::default()
    );
    assert!(matches!(
        reopened.restore_package_scene(&manifest, &prepared.assets),
        Err(ProjectError::Conflict(_))
    ));
    drop(prepared);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn original_planned_actual_section_fixture_retains_roles_and_coordinates() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scene/planned-actual-section.glb");
    let scene = vwm_io::load_scene(&path).unwrap();
    assert_eq!(scene.primitives.len(), 4);
    let document = scene.source_document.as_ref().unwrap();
    let roles: Vec<_> = document["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["extras"]["kind"].as_str().unwrap())
        .collect();
    assert_eq!(roles, ["plan", "actual", "top", "section"]);
    assert_eq!(
        document["extras"]["mavisSyntheticFixture"]["expectedTopIntersections"],
        serde_json::json!([[10, -150, 0], [25, -150, 5]])
    );
    assert!(scene
        .vertices
        .iter()
        .all(|p| p.iter().all(|v| v.is_finite())));
}

fn child_package() -> (
    std::path::PathBuf,
    ProjectStore,
    Project,
    Revision,
    serde_json::Value,
    packages::PreparedPackage,
) {
    let root = std::env::temp_dir().join(format!("3dmk-child-{}", Uuid::new_v4()));
    let store = ProjectStore::open(root.join("original")).unwrap();
    let fixture = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/models/BoxTextured.glb"
    ))
    .unwrap();
    let (project, _, revision) = store
        .import_root_scene(
            RootSceneImport {
                project_name: "Active child".into(),
                original_name: "root.glb".into(),
                media_type: "model/gltf-binary".into(),
                scene_kind: SceneKind::Mesh,
                attributes: AttributeContract::default(),
                original_units: Some("m".into()),
                warnings: vec![],
            },
            Cursor::new(fixture),
        )
        .unwrap();
    let child_bytes = b"ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n100 0 0\n101 0 0\n100 1 0\n3 0 1 2\n";
    let asset = store
        .store_derived_asset(
            project.project_id,
            "child.ply",
            "application/octet-stream",
            AssetRole::DerivedScene,
            Cursor::new(child_bytes),
        )
        .unwrap();
    let child = store
        .create_child_revision(
            project.project_id,
            revision.revision_id,
            asset.asset_id,
            SceneKind::Mesh,
            TopologyRelation::Reconstructed,
            AttributeContract::default(),
            None,
        )
        .unwrap();
    let project = store
        .set_active_revision(project.project_id, child.revision_id)
        .unwrap();
    let zip = root.join("child.zip");
    store.export_package(project.project_id, &zip).unwrap();
    let prepared =
        packages::prepare_archive(&zip, &root.join("staging"), None, ArchiveLimits::default())
            .unwrap();
    let asset = prepared
        .assets
        .iter()
        .find(|a| a.package_paths.iter().any(|p| p == "3dmk-package.json"))
        .unwrap();
    let manifest = serde_json::from_slice(&fs::read(&asset.source_path).unwrap()).unwrap();
    (root, store, project, child, manifest, prepared)
}

#[test]
fn active_child_package_reopens_its_distinct_render_asset() {
    let (root, _, project, child, manifest, prepared) = child_package();
    let reopened = ProjectStore::open(root.join("reopened")).unwrap();
    let restored = reopened
        .restore_package_scene(&manifest, &prepared.assets)
        .unwrap();
    assert_eq!(restored.project, project);
    assert_eq!(restored.root_revision, child);
    assert_ne!(
        restored.root_revision.revision_id,
        project.root_revision_id.unwrap()
    );
    let (_, payload) = reopened
        .asset_payload(project.project_id, child.render_asset_id.unwrap())
        .unwrap();
    assert!(String::from_utf8(fs::read(payload).unwrap())
        .unwrap()
        .contains("100 0 0"));
    let html = include_str!("../public/index.html");
    let compact_html: String = html.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(compact_html.contains(
        "imported.root_revision?.render_asset_id||imported.root_revision?.scene_asset_id"
    ));
    drop(prepared);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn dangling_grandparent_is_rejected_without_panicking() {
    let (root, _, project, child, mut manifest, prepared) = child_package();
    let revisions = manifest["revisions"].as_array_mut().unwrap();
    // Put the child first: its direct parent exists, but that parent's parent does not.
    revisions.sort_by_key(|r| r["revision_id"] != child.revision_id.to_string());
    let parent = revisions
        .iter_mut()
        .find(|r| r["revision_id"] == project.root_revision_id.unwrap().to_string())
        .unwrap();
    parent["parent_revision_ids"] = serde_json::json!([Uuid::new_v4()]);
    let reopened = ProjectStore::open(root.join("rejected")).unwrap();
    let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reopened.restore_package_scene(&manifest, &prepared.assets)
    }));
    assert!(attempt.is_ok(), "malformed package must not panic");
    assert!(attempt.unwrap().is_err());
    assert!(reopened.list_projects().unwrap().is_empty());
    drop(prepared);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn same_identity_with_changed_records_or_payload_is_a_conflict() {
    let (root, _, project, _, mut manifest, prepared) = child_package();
    let id = Uuid::new_v4();
    let key = format!("measurements/{id}.json");
    manifest["project"]["measurement_set_ids"] = serde_json::json!([id]);
    manifest["records"][&key] =
        serde_json::json!({"measurement_set_id": id, "measurements": [{"length_m": 1.0}]});
    let reopened = ProjectStore::open(root.join("reopened")).unwrap();
    let restored = reopened
        .restore_package_scene(&manifest, &prepared.assets)
        .unwrap();
    let mut changed = manifest.clone();
    changed["records"][&key]["measurements"][0]["length_m"] = serde_json::json!(2.0);
    assert!(matches!(
        reopened.restore_package_scene(&changed, &prepared.assets),
        Err(ProjectError::Conflict(_))
    ));
    let (_, path) = reopened
        .asset_payload(project.project_id, restored.root_revision.scene_asset_id)
        .unwrap();
    let mut bytes = fs::read(&path).unwrap();
    bytes[0] ^= 1;
    fs::write(path, bytes).unwrap();
    assert!(reopened
        .restore_package_scene(&manifest, &prepared.assets)
        .is_err());
    drop(prepared);
    fs::remove_dir_all(root).unwrap();
}
