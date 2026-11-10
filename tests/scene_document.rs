use agentic_cad_backend::scene_document::*;
use std::{collections::HashSet, fs};
use uuid::Uuid;

fn setup() -> (SceneState, HashSet<Uuid>, SceneObject) {
    let asset_id = Uuid::new_v4();
    let object = SceneObject::instance("Synthetic model (metres)", asset_id);
    (
        SceneState::new(Uuid::new_v4()),
        HashSet::from([asset_id]),
        object,
    )
}

fn run(
    state: &mut SceneState,
    assets: &HashSet<Uuid>,
    command: SceneCommand,
) -> SceneCommandResult {
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: state.document.revision,
        command,
    };
    state.apply(&request, assets).unwrap()
}

#[test]
fn numeric_edit_is_one_atomic_undo_step_and_redo_survives_serialization() {
    let (mut state, assets, object) = setup();
    let id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let transform = LocalTransform {
        translation: [123.25, -87.5, 2.0],
        ..LocalTransform::default()
    };
    run(
        &mut state,
        &assets,
        SceneCommand::SetTransform {
            object_id: id,
            transform,
        },
    );
    run(&mut state, &assets, SceneCommand::Undo {});
    assert_eq!(
        state.document.objects[&id].transform,
        LocalTransform::default()
    );
    let bytes = serde_json::to_vec(&state).unwrap();
    let mut reopened: SceneState = serde_json::from_slice(&bytes).unwrap();
    reopened.validate(&assets).unwrap();
    run(&mut reopened, &assets, SceneCommand::Redo {});
    assert_eq!(
        reopened.document.objects[&id].transform.translation,
        [123.25, -87.5, 2.0]
    );
    assert_eq!(reopened.document.revision, 4);
}

#[test]
fn stale_commands_and_changed_duplicate_ids_leave_state_unchanged() {
    let (mut state, assets, object) = setup();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::AddObject { object },
    };
    state.apply(&request, &assets).unwrap();
    let before = serde_json::to_vec(&state).unwrap();
    assert!(state.apply(&request, &assets).unwrap().duplicate);
    assert_eq!(serde_json::to_vec(&state).unwrap(), before);
    let mut changed = request.clone();
    changed.command = SceneCommand::Undo {};
    assert!(matches!(
        state.apply(&changed, &assets),
        Err(SceneError::CommandConflict(_))
    ));
    changed.command_id = Uuid::new_v4();
    assert!(matches!(
        state.apply(&changed, &assets),
        Err(SceneError::StaleRevision {
            expected: 0,
            actual: 1
        })
    ));
    assert_eq!(serde_json::to_vec(&state).unwrap(), before);
}

#[test]
fn invalid_transforms_and_missing_assets_are_transactional() {
    let (mut state, assets, object) = setup();
    let id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    for transform in [
        LocalTransform {
            translation: [f64::NAN, 0.0, 0.0],
            ..Default::default()
        },
        LocalTransform {
            rotation: [0.0; 4],
            ..Default::default()
        },
        LocalTransform {
            rotation: [0.0, 0.0, 0.0, 2.0],
            ..Default::default()
        },
        LocalTransform {
            scale: [1.0, 0.0, 1.0],
            ..Default::default()
        },
    ] {
        let before = state.clone();
        let request = SceneCommandRequest {
            command_id: Uuid::new_v4(),
            expected_revision: state.document.revision,
            command: SceneCommand::SetTransform {
                object_id: id,
                transform,
            },
        };
        assert!(state.apply(&request, &assets).is_err());
        assert_eq!(state, before);
    }
    let object = SceneObject::instance("Missing source", Uuid::new_v4());
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: state.document.revision,
        command: SceneCommand::AddObject { object },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::MissingAsset(_))
    ));
}

#[test]
fn parent_cycles_and_missing_parents_are_rejected() {
    let (mut state, assets, mut object) = setup();
    let root = SceneObject::group("Group");
    let root_id = root.object_id;
    object.parent_id = Some(root_id);
    let child_id = object.object_id;
    run(
        &mut state,
        &assets,
        SceneCommand::AddObject { object: root },
    );
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let before = state.clone();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 2,
        command: SceneCommand::SetParent {
            object_id: root_id,
            parent_id: Some(child_id),
        },
    };
    assert!(state.apply(&request, &assets).is_err());
    assert_eq!(state, before);
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 2,
        command: SceneCommand::SetParent {
            object_id: child_id,
            parent_id: Some(Uuid::new_v4()),
        },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::MissingParent { .. })
    ));
    assert_eq!(state, before);
}

#[test]
fn locked_descendants_and_locked_ancestors_cannot_move_indirectly() {
    let (mut state, assets, mut object) = setup();
    let root = SceneObject::group("Group");
    let root_id = root.object_id;
    let child_id = object.object_id;
    object.parent_id = Some(root_id);
    run(
        &mut state,
        &assets,
        SceneCommand::AddObject { object: root },
    );
    run(&mut state, &assets, SceneCommand::AddObject { object });
    run(
        &mut state,
        &assets,
        SceneCommand::SetLocked {
            object_id: child_id,
            locked: true,
        },
    );
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 3,
        command: SceneCommand::SetTransform {
            object_id: root_id,
            transform: LocalTransform {
                translation: [5.0, 0.0, 0.0],
                ..Default::default()
            },
        },
    };
    assert!(
        matches!(state.apply(&request, &assets), Err(SceneError::LockedObject(id)) if id == child_id)
    );
    run(
        &mut state,
        &assets,
        SceneCommand::SetLocked {
            object_id: child_id,
            locked: false,
        },
    );
    run(
        &mut state,
        &assets,
        SceneCommand::SetLocked {
            object_id: root_id,
            locked: true,
        },
    );
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 5,
        command: SceneCommand::SetParent {
            object_id: child_id,
            parent_id: None,
        },
    };
    assert!(
        matches!(state.apply(&request, &assets), Err(SceneError::LockedObject(id)) if id == root_id)
    );
}

#[test]
fn instances_share_asset_identity_without_geometry_or_rebinding() {
    let (mut state, assets, object) = setup();
    let mut second = object.clone();
    second.object_id = Uuid::new_v4();
    let first_id = object.object_id;
    run(
        &mut state,
        &assets,
        SceneCommand::AddObject {
            object: object.clone(),
        },
    );
    run(
        &mut state,
        &assets,
        SceneCommand::AddObject {
            object: second.clone(),
        },
    );
    assert_eq!(
        state.document.objects[&first_id].asset_id,
        state.document.objects[&second.object_id].asset_id
    );
    run(
        &mut state,
        &assets,
        SceneCommand::RemoveObject {
            object_id: first_id,
        },
    );
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 3,
        command: SceneCommand::AddObject { object },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::ObjectIdentityConflict(_))
    ));
}

fn view_for(id: Uuid) -> NamedView {
    NamedView {
        view_id: Uuid::new_v4(),
        name: "Synthetic plan/actual section (metres)".into(),
        camera: ViewCamera {
            projection: CameraProjection::Orthographic {
                height_metres: 100.0,
                near: 0.1,
                far: 10000.0,
            },
            position: [0.0, 0.0, 100.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            target: [0.0; 3],
        },
        visibility: [(id, false)].into(),
        clipping: ClippingState {
            enabled: true,
            planes: vec![ClipPlane {
                normal: [0.0, 0.0, 1.0],
                offset_metres: 0.0,
            }],
        },
        section: Some(SectionState {
            name: "Synthetic vertical section".into(),
            origin: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            up: [0.0, 1.0, 0.0],
            thickness_metres: 2.0,
        }),
    }
}

#[test]
fn view_restores_projection_visibility_clipping_and_section() {
    let (mut state, assets, object) = setup();
    let id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let view = view_for(id);
    let view_id = view.view_id;
    run(
        &mut state,
        &assets,
        SceneCommand::SaveView { view: view.clone() },
    );
    run(
        &mut state,
        &assets,
        SceneCommand::ActivateView {
            view_id,
            previous_view: None,
        },
    );
    assert_eq!(state.document.active_view_id, Some(view_id));
    assert!(!state.document.objects[&id].visible);
    assert_eq!(state.document.views[&view_id], view);
    run(&mut state, &assets, SceneCommand::Undo {});
    assert_eq!(state.document.active_view_id, None);
    assert!(state.document.objects[&id].visible);
}

#[test]
fn missing_view_references_and_referenced_removals_are_rejected_cleanly() {
    let (mut state, assets, object) = setup();
    let id = object.object_id;
    let view = view_for(id);
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::SaveView { view: view.clone() },
    };
    assert!(
        matches!(state.apply(&request, &assets), Err(SceneError::MissingViewObject { object_id, .. }) if object_id == id)
    );
    run(&mut state, &assets, SceneCommand::AddObject { object });
    run(&mut state, &assets, SceneCommand::SaveView { view });
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 2,
        command: SceneCommand::RemoveObject { object_id: id },
    };
    assert!(
        matches!(state.apply(&request, &assets), Err(SceneError::MissingViewObject { object_id, .. }) if object_id == id)
    );
    assert!(state.document.objects.contains_key(&id));
}

#[test]
fn history_is_bounded_and_new_edits_clear_redo() {
    let (mut state, assets, object) = setup();
    let id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    for n in 0..MAX_HISTORY_STEPS + 4 {
        run(
            &mut state,
            &assets,
            SceneCommand::SetTransform {
                object_id: id,
                transform: LocalTransform {
                    translation: [n as f64, 0.0, 0.0],
                    ..Default::default()
                },
            },
        );
    }
    assert_eq!(state.undo_depth(), MAX_HISTORY_STEPS);
    run(&mut state, &assets, SceneCommand::Undo {});
    assert!(state.can_redo());
    run(
        &mut state,
        &assets,
        SceneCommand::RenameObject {
            object_id: id,
            name: "Changed".into(),
        },
    );
    assert!(!state.can_redo());
}

#[test]
fn persistence_roundtrip_keeps_history_identity_and_receipts() {
    let (mut state, assets, object) = setup();
    let dir = std::env::temp_dir().join(format!("3dmk-scene-{}", Uuid::new_v4()));
    fs::create_dir(&dir).unwrap();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::AddObject { object },
    };
    state.apply(&request, &assets).unwrap();
    save_scene_state(&dir, &state, &assets).unwrap();
    let mut reopened = load_scene_state(&dir, state.document.project_id, &assets).unwrap();
    assert_eq!(reopened, state);
    assert!(reopened.apply(&request, &assets).unwrap().duplicate);
    assert!(matches!(
        load_scene_state(&dir, Uuid::new_v4(), &assets),
        Err(SceneError::ProjectMismatch)
    ));
    assert!(matches!(
        load_scene_state(&dir, state.document.project_id, &HashSet::new()),
        Err(SceneError::MissingAsset(_))
    ));
    run(&mut reopened, &assets, SceneCommand::Undo {});
    save_scene_state(&dir, &reopened, &assets).unwrap();
    assert!(load_scene_state(&dir, state.document.project_id, &assets)
        .unwrap()
        .can_redo());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn frame_is_explicit_metres_y_up_and_unknown_fields_reject() {
    let state = SceneState::new(Uuid::new_v4());
    let json = serde_json::to_value(&state.document).unwrap();
    assert_eq!(json["units"], "metres");
    assert_eq!(json["frame"]["up_axis"], "y");
    assert_eq!(json["frame"]["kind"], "project_local");
    let bad = serde_json::json!({"command_id":Uuid::new_v4(),"expected_revision":0,"command":{"type":"undo","asset_id":Uuid::new_v4()}});
    assert!(serde_json::from_value::<SceneCommandRequest>(bad).is_err());
}

#[test]
fn bootstrap_and_restored_history_reject_source_identity_tampering() {
    let (mut state, mut assets, object) = setup();
    let id = object.object_id;
    let source = object.asset_id.unwrap();
    let another_asset = Uuid::new_v4();
    assets.insert(another_asset);
    run(&mut state, &assets, SceneCommand::AddObject { object });
    run(
        &mut state,
        &assets,
        SceneCommand::RenameObject {
            object_id: id,
            name: "Edited name".into(),
        },
    );
    let original = serde_json::to_value(&state).unwrap();
    let mut tampered = original.clone();
    tampered["document"]["objects"][id.to_string()]["asset_id"] = serde_json::json!(another_asset);
    let altered: SceneState = serde_json::from_value(tampered).unwrap();
    assert!(
        matches!(altered.validate(&assets), Err(SceneError::ObjectIdentityConflict(object_id)) if object_id == id)
    );
    let mut tampered = original;
    tampered["undo_history"][1]["objects"][id.to_string()]["asset_id"] =
        serde_json::json!(another_asset);
    let altered: SceneState = serde_json::from_value(tampered).unwrap();
    assert!(
        matches!(altered.validate(&assets), Err(SceneError::ObjectIdentityConflict(object_id)) if object_id == id)
    );
    run(
        &mut state,
        &assets,
        SceneCommand::RemoveObject { object_id: id },
    );
    assets.remove(&source);
    assert!(
        matches!(state.validate(&assets), Err(SceneError::MissingAsset(asset_id)) if asset_id == source)
    );
}

#[test]
fn duplicate_old_request_after_undo_does_not_reapply_its_edit() {
    let (mut state, assets, object) = setup();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::AddObject { object },
    };
    state.apply(&request, &assets).unwrap();
    run(&mut state, &assets, SceneCommand::Undo {});
    let before = state.clone();
    let response = state.apply(&request, &assets).unwrap();
    assert!(response.duplicate);
    assert!(response.document.objects.is_empty());
    assert_eq!(response.document.revision, 2);
    assert_eq!(state, before);
}

#[test]
fn cycle_between_groups_is_detected_without_mutation() {
    let (mut state, assets, _) = setup();
    let first = SceneObject::group("First");
    let first_id = first.object_id;
    let mut second = SceneObject::group("Second");
    let second_id = second.object_id;
    second.parent_id = Some(first_id);
    run(
        &mut state,
        &assets,
        SceneCommand::AddObject { object: first },
    );
    run(
        &mut state,
        &assets,
        SceneCommand::AddObject { object: second },
    );
    let before = state.clone();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 2,
        command: SceneCommand::SetParent {
            object_id: first_id,
            parent_id: Some(second_id),
        },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::GraphCycle(_))
    ));
    assert_eq!(state, before);
}

#[test]
fn invalid_view_camera_and_section_are_rejected() {
    let (mut state, assets, object) = setup();
    let id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let mut views = vec![view_for(id); 5];
    views[0].camera.projection = CameraProjection::Perspective {
        vertical_fov_degrees: 180.0,
        near: 0.1,
        far: 100.0,
    };
    views[1].camera.position = views[1].camera.target;
    views[2].clipping.planes[0].normal = [0.0, 0.0, 2.0];
    views[3].section.as_mut().unwrap().up = [0.0, 0.0, 1.0];
    views[4].camera.projection = CameraProjection::Orthographic {
        height_metres: 0.0,
        near: 0.1,
        far: 100.0,
    };
    for view in views {
        let request = SceneCommandRequest {
            command_id: Uuid::new_v4(),
            expected_revision: 1,
            command: SceneCommand::SaveView { view },
        };
        assert!(matches!(
            state.apply(&request, &assets),
            Err(SceneError::Invalid(_))
        ));
        assert_eq!(state.document.revision, 1);
    }
}

#[test]
fn rejected_save_preserves_previous_json_and_leaves_no_temporary_file() {
    let (mut state, assets, object) = setup();
    let id = object.object_id;
    let dir = std::env::temp_dir().join(format!("3dmk-scene-{}", Uuid::new_v4()));
    fs::create_dir(&dir).unwrap();
    run(&mut state, &assets, SceneCommand::AddObject { object });
    save_scene_state(&dir, &state, &assets).unwrap();
    let previous = fs::read(dir.join(SCENE_FILE_NAME)).unwrap();
    state.document.objects.get_mut(&id).unwrap().opacity = f64::INFINITY;
    assert!(save_scene_state(&dir, &state, &assets).is_err());
    assert_eq!(fs::read(dir.join(SCENE_FILE_NAME)).unwrap(), previous);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn finite_but_oversized_state_and_unsupported_schema_fail_closed() {
    let (state, assets, object) = setup();
    let mut document = state.document.clone();
    document.schema_version += 1;
    assert!(matches!(
        SceneState::from_document(document, &assets),
        Err(SceneError::UnsupportedSchema(_))
    ));
    let mut document = state.document;
    for _ in 0..=MAX_SCENE_OBJECTS {
        let object = SceneObject {
            object_id: Uuid::new_v4(),
            ..object.clone()
        };
        document.objects.insert(object.object_id, object);
    }
    assert!(matches!(
        SceneState::from_document(document, &assets),
        Err(SceneError::Limit(_))
    ));
}

#[test]
fn project_store_zip_roundtrip_preserves_scene_commands_views_and_conflicts() {
    use agentic_cad_backend::{
        packages::{self, ArchiveLimits},
        projects::{AttributeContract, ProjectError, ProjectStore, RootSceneImport, SceneKind},
    };
    use std::io::Cursor;

    let root = std::env::temp_dir().join(format!("3dmk-scene-package-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let store = ProjectStore::open(root.join("original")).unwrap();
    let fixture = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/models/BoxTextured.glb"
    ))
    .unwrap();
    let (project, asset, _) = store
        .import_root_scene(
            RootSceneImport {
                project_name: "Synthetic placement review (metres)".into(),
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
    let object = SceneObject::instance("Synthetic reference instance", asset.asset_id);
    let object_id = object.object_id;
    let add = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::AddObject { object },
    };
    store.apply_scene_command(project.project_id, &add).unwrap();
    let transform = LocalTransform {
        translation: [125.125, -40.25, -7.75],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 2.0, 1.0],
    };
    let numeric_edit = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 1,
        command: SceneCommand::SetTransform {
            object_id,
            transform: transform.clone(),
        },
    };
    store
        .apply_scene_command(project.project_id, &numeric_edit)
        .unwrap();
    let view = view_for(object_id);
    let save_view = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 2,
        command: SceneCommand::SaveView { view: view.clone() },
    };
    store
        .apply_scene_command(project.project_id, &save_view)
        .unwrap();
    let activate = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 3,
        command: SceneCommand::ActivateView {
            view_id: view.view_id,
            previous_view: None,
        },
    };
    let expected = store
        .apply_scene_command(project.project_id, &activate)
        .unwrap()
        .document;
    assert_eq!(expected.objects[&object_id].transform, transform);
    assert!(!expected.objects[&object_id].visible);

    // Reopen the filesystem store before export, and retain accepted request IDs.
    drop(store);
    let reopened = ProjectStore::open(root.join("original")).unwrap();
    let repeated = reopened
        .apply_scene_command(project.project_id, &numeric_edit)
        .unwrap();
    assert!(repeated.duplicate);
    assert_eq!(repeated.document, expected);
    let stale = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 1,
        command: SceneCommand::RenameObject {
            object_id,
            name: "Must not publish".into(),
        },
    };
    assert!(matches!(
        reopened.apply_scene_command(project.project_id, &stale),
        Err(ProjectError::Scene(SceneError::StaleRevision {
            expected: 1,
            actual: 4
        }))
    ));
    let mut changed_replay = numeric_edit.clone();
    changed_replay.command = SceneCommand::Undo {};
    assert!(matches!(
        reopened.apply_scene_command(project.project_id, &changed_replay),
        Err(ProjectError::Scene(SceneError::CommandConflict(_)))
    ));
    assert_eq!(
        reopened
            .get_scene_document(project.project_id)
            .unwrap()
            .document,
        expected
    );

    let package_path = root.join("scene.zip");
    reopened
        .export_package(project.project_id, &package_path)
        .unwrap();
    let prepared = packages::prepare_archive(
        &package_path,
        &root.join("staging"),
        None,
        ArchiveLimits::default(),
    )
    .unwrap();
    let manifest_file = prepared
        .assets
        .iter()
        .find(|candidate| {
            candidate
                .package_paths
                .iter()
                .any(|path| path == "3dmk-package.json")
        })
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_file.source_path).unwrap()).unwrap();
    assert_eq!(
        manifest["scene_state"]["document"],
        serde_json::to_value(&expected).unwrap()
    );
    assert_eq!(
        manifest["scene_state"]["command_receipts"]
            .as_object()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        manifest["scene_state"]["undo_history"]
            .as_array()
            .unwrap()
            .len(),
        4
    );

    let restored = ProjectStore::open(root.join("restored")).unwrap();
    restored
        .restore_package_scene(&manifest, &prepared.assets)
        .unwrap();
    assert_eq!(
        restored
            .get_scene_document(project.project_id)
            .unwrap()
            .document,
        expected
    );
    let (_, source_path) = restored
        .asset_payload(project.project_id, asset.asset_id)
        .unwrap();
    assert_eq!(fs::read(source_path).unwrap(), fixture);
    assert!(
        restored
            .apply_scene_command(project.project_id, &add)
            .unwrap()
            .duplicate
    );
    assert!(matches!(
        restored.apply_scene_command(project.project_id, &stale),
        Err(ProjectError::Scene(SceneError::StaleRevision {
            actual: 4,
            ..
        }))
    ));
    // Re-importing an identical envelope is idempotent; a later local edit is protected.
    restored
        .restore_package_scene(&manifest, &prepared.assets)
        .unwrap();
    assert_eq!(restored.list_projects().unwrap().len(), 1);
    let undo = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 4,
        command: SceneCommand::Undo {},
    };
    let after_undo = restored
        .apply_scene_command(project.project_id, &undo)
        .unwrap();
    assert_eq!(after_undo.document.active_view_id, None);
    assert!(after_undo.document.objects[&object_id].visible);
    assert_eq!(after_undo.document.views[&view.view_id], view);
    assert!(after_undo.can_redo);
    assert!(matches!(
        restored.restore_package_scene(&manifest, &prepared.assets),
        Err(ProjectError::Conflict(_))
    ));
    assert_eq!(
        restored
            .get_scene_document(project.project_id)
            .unwrap()
            .document,
        after_undo.document
    );

    // An invalid archived view reference fails before publishing a new project.
    let mut damaged = manifest.clone();
    damaged["scene_state"]["document"]["views"][view.view_id.to_string()]["visibility"]
        [Uuid::new_v4().to_string()] = serde_json::json!(true);
    let rejected = ProjectStore::open(root.join("rejected")).unwrap();
    assert!(matches!(
        rejected.restore_package_scene(&damaged, &prepared.assets),
        Err(ProjectError::Scene(SceneError::MissingViewObject { .. }))
    ));
    assert!(rejected.list_projects().unwrap().is_empty());
    drop(prepared);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cloned_project_store_serializes_competing_scene_revisions() {
    use agentic_cad_backend::projects::{ProjectError, ProjectStore};
    use std::sync::{Arc, Barrier};
    let root = std::env::temp_dir().join(format!("3dmk-scene-concurrent-{}", Uuid::new_v4()));
    let store = ProjectStore::open(&root).unwrap();
    let project = store
        .create_project("Concurrent placement commands")
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let threads = (0..2)
        .map(|index| {
            let store = store.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let request = SceneCommandRequest {
                    command_id: Uuid::new_v4(),
                    expected_revision: 0,
                    command: SceneCommand::AddObject {
                        object: SceneObject::group(format!("Group {index}")),
                    },
                };
                barrier.wait();
                store.apply_scene_command(project.project_id, &request)
            })
        })
        .collect::<Vec<_>>();
    let results = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(ProjectError::Scene(SceneError::StaleRevision {
                    expected: 0,
                    actual: 1
                }))
            ))
            .count(),
        1
    );
    let state = store.get_scene_document(project.project_id).unwrap();
    assert_eq!(state.document.revision, 1);
    assert_eq!(state.document.objects.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn orthographic_signed_clip_range_is_preserved_while_perspective_requires_positive_near() {
    let (mut state, assets, object) = setup();
    let object_id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let mut view = view_for(object_id);
    for near in [-1000.0, 0.0] {
        view.camera.projection = CameraProjection::Orthographic {
            height_metres: 100.0,
            near,
            far: 1000.0,
        };
        run(
            &mut state,
            &assets,
            SceneCommand::SaveView { view: view.clone() },
        );
        assert_eq!(
            state.document.views[&view.view_id].camera.projection,
            view.camera.projection
        );
    }
    for near in [-1000.0, 0.0] {
        view.camera.projection = CameraProjection::Perspective {
            vertical_fov_degrees: 45.0,
            near,
            far: 1000.0,
        };
        let request = SceneCommandRequest {
            command_id: Uuid::new_v4(),
            expected_revision: state.document.revision,
            command: SceneCommand::SaveView { view: view.clone() },
        };
        assert!(matches!(
            state.apply(&request, &assets),
            Err(SceneError::Invalid(_))
        ));
    }
}

#[test]
fn first_view_activation_preserves_previous_camera_for_undo_across_reopen() {
    let (mut state, assets, object) = setup();
    let object_id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let target = view_for(object_id);
    run(
        &mut state,
        &assets,
        SceneCommand::SaveView {
            view: target.clone(),
        },
    );
    let mut previous = view_for(object_id);
    previous.name = "Unsaved camera before activation".into();
    previous.camera.position = [50.0, 25.0, 75.0];
    previous.camera.projection = CameraProjection::Perspective {
        vertical_fov_degrees: 60.0,
        near: 0.01,
        far: 2000.0,
    };
    previous.visibility.insert(object_id, true);
    previous.clipping = ClippingState::default();
    previous.section = None;
    run(
        &mut state,
        &assets,
        SceneCommand::ActivateView {
            view_id: target.view_id,
            previous_view: Some(previous.clone()),
        },
    );
    assert_eq!(state.document.camera_view, Some(target.clone()));
    assert!(!state.document.views.contains_key(&previous.view_id));
    let dir = std::env::temp_dir().join(format!("3dmk-scene-camera-{}", Uuid::new_v4()));
    fs::create_dir(&dir).unwrap();
    save_scene_state(&dir, &state, &assets).unwrap();
    let mut reopened = load_scene_state(&dir, state.document.project_id, &assets).unwrap();
    run(&mut reopened, &assets, SceneCommand::Undo {});
    assert_eq!(reopened.document.active_view_id, None);
    assert_eq!(reopened.document.camera_view, Some(previous));
    assert!(reopened.document.objects[&object_id].visible);
    run(&mut reopened, &assets, SceneCommand::Redo {});
    assert_eq!(reopened.document.active_view_id, Some(target.view_id));
    assert_eq!(reopened.document.camera_view, Some(target));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn invalid_previous_camera_is_rejected_without_mutating_history() {
    let (mut state, assets, object) = setup();
    let object_id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let target = view_for(object_id);
    run(
        &mut state,
        &assets,
        SceneCommand::SaveView {
            view: target.clone(),
        },
    );
    let mut previous = view_for(Uuid::new_v4());
    previous.name = "Invalid snapshot".into();
    let before = state.clone();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: state.document.revision,
        command: SceneCommand::ActivateView {
            view_id: target.view_id,
            previous_view: Some(previous),
        },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::MissingViewObject { .. })
    ));
    assert_eq!(state, before);
}

#[test]
fn old_activate_view_json_keeps_its_canonical_fingerprint_and_no_invented_camera() {
    let (mut state, assets, object) = setup();
    let id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let view = view_for(id);
    run(
        &mut state,
        &assets,
        SceneCommand::SaveView { view: view.clone() },
    );
    let json = serde_json::json!({"command_id":Uuid::new_v4(),"expected_revision":2,"command":{"type":"activate_view","view_id":view.view_id}});
    let request: SceneCommandRequest = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&request).unwrap(), json);
    state.apply(&request, &assets).unwrap();
    run(&mut state, &assets, SceneCommand::Undo {});
    assert!(state.document.camera_view.is_none());
    assert!(state.document.active_view_id.is_none());
}

#[test]
fn removing_last_named_view_does_not_make_current_camera_references_undeletable() {
    let (mut state, assets, object) = setup();
    let object_id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let view = view_for(object_id);
    run(
        &mut state,
        &assets,
        SceneCommand::SaveView { view: view.clone() },
    );
    run(
        &mut state,
        &assets,
        SceneCommand::ActivateView {
            view_id: view.view_id,
            previous_view: None,
        },
    );
    run(
        &mut state,
        &assets,
        SceneCommand::RemoveView {
            view_id: view.view_id,
        },
    );
    run(
        &mut state,
        &assets,
        SceneCommand::RemoveObject { object_id },
    );
    assert!(state
        .document
        .camera_view
        .as_ref()
        .unwrap()
        .visibility
        .is_empty());
    run(&mut state, &assets, SceneCommand::Undo {});
    assert!(state
        .document
        .camera_view
        .as_ref()
        .unwrap()
        .visibility
        .contains_key(&object_id));
}

#[test]
fn batch_hierarchy_is_order_independent_and_one_persisted_undo_redo_step() {
    let (mut state, mut assets, mut first) = setup();
    let second_asset = Uuid::new_v4();
    assets.insert(second_asset);
    let mut second = SceneObject::instance("Second GLB source node", second_asset);
    let root = SceneObject::group("Composited GLBs");
    let mut first_node = SceneObject::group("First GLB node");
    let mut second_node = SceneObject::group("Second GLB node");
    first_node.parent_id = Some(root.object_id);
    second_node.parent_id = Some(root.object_id);
    first.parent_id = Some(first_node.object_id);
    first.asset_node_index = Some(0);
    second.parent_id = Some(second_node.object_id);
    second.asset_node_index = Some(17);
    let first_id = first.object_id;
    let second_id = second.object_id;
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::AddObjects {
            objects: vec![first, second, second_node, first_node, root],
        },
    };
    let response = state.apply(&request, &assets).unwrap();
    assert!(!response.duplicate);
    assert_eq!(state.document.revision, 1);
    assert_eq!(state.undo_depth(), 1);
    assert_eq!(state.document.objects.len(), 5);
    assert_eq!(state.document.objects[&first_id].asset_node_index, Some(0));
    assert_eq!(
        state.document.objects[&second_id].asset_node_index,
        Some(17)
    );
    let accepted = state.document.objects.clone();
    assert!(state.apply(&request, &assets).unwrap().duplicate);
    assert_eq!(state.undo_depth(), 1);

    let dir = std::env::temp_dir().join(format!("3dmk-scene-batch-{}", Uuid::new_v4()));
    fs::create_dir(&dir).unwrap();
    save_scene_state(&dir, &state, &assets).unwrap();
    let mut reopened = load_scene_state(&dir, state.document.project_id, &assets).unwrap();
    run(&mut reopened, &assets, SceneCommand::Undo {});
    assert!(reopened.document.objects.is_empty());
    assert_eq!(reopened.undo_depth(), 0);
    assert_eq!(reopened.redo_depth(), 1);
    let after_undo = reopened.clone();
    assert!(reopened.apply(&request, &assets).unwrap().duplicate);
    assert_eq!(reopened, after_undo);
    save_scene_state(&dir, &reopened, &assets).unwrap();
    let mut reopened = load_scene_state(&dir, state.document.project_id, &assets).unwrap();
    run(&mut reopened, &assets, SceneCommand::Redo {});
    assert_eq!(reopened.document.objects, accepted);
    assert_eq!(reopened.document.revision, 3);
    assert_eq!(reopened.undo_depth(), 1);
    assert!(!reopened.can_redo());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn invalid_batch_rolls_back_objects_identities_receipts_and_history() {
    let (mut state, assets, object) = setup();
    run(&mut state, &assets, SceneCommand::AddObject { object });
    run(&mut state, &assets, SceneCommand::Undo {});
    let before = state.clone();
    let valid = SceneObject::group("Must roll back");
    let mut invalid = SceneObject::instance("Missing source", Uuid::new_v4());
    invalid.asset_node_index = Some(0);
    let mut request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: state.document.revision,
        command: SceneCommand::AddObjects {
            objects: vec![valid.clone(), invalid.clone()],
        },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::MissingAsset(_))
    ));
    assert_eq!(state, before);
    invalid.asset_id = assets.iter().next().copied();
    request.command = SceneCommand::AddObjects {
        objects: vec![valid, invalid],
    };
    // Reusing the failed command and object IDs is safe: no receipt or identity leaked.
    state.apply(&request, &assets).unwrap();
    assert_eq!(state.document.objects.len(), 2);
    assert_eq!(state.undo_depth(), 1);
    assert!(!state.can_redo());
}

#[test]
fn batch_rejects_duplicate_live_and_previously_used_object_ids() {
    let (mut state, assets, object) = setup();
    let mut changed = object.clone();
    changed.asset_node_index = Some(9);
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::AddObjects {
            objects: vec![object.clone(), changed.clone()],
        },
    };
    let before = state.clone();
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::ObjectIdentityConflict(id)) if id == object.object_id
    ));
    assert_eq!(state, before);
    run(
        &mut state,
        &assets,
        SceneCommand::AddObject {
            object: object.clone(),
        },
    );
    for removed in [false, true] {
        if removed {
            run(
                &mut state,
                &assets,
                SceneCommand::RemoveObject {
                    object_id: object.object_id,
                },
            );
        }
        let before = state.clone();
        let request = SceneCommandRequest {
            command_id: Uuid::new_v4(),
            expected_revision: state.document.revision,
            command: SceneCommand::AddObjects {
                objects: vec![SceneObject::group("Must roll back"), changed.clone()],
            },
        };
        assert!(matches!(
            state.apply(&request, &assets),
            Err(SceneError::ObjectIdentityConflict(id)) if id == object.object_id
        ));
        assert_eq!(state, before);
    }
}

#[test]
fn batch_rejects_empty_invalid_parent_cycles_and_locked_hierarchies() {
    let (state, assets, instance) = setup();
    let mut missing_parent = SceneObject::group("Missing parent");
    missing_parent.parent_id = Some(Uuid::new_v4());
    let mut child_of_instance = SceneObject::group("Invalid parent kind");
    child_of_instance.parent_id = Some(instance.object_id);
    let mut cyclic_a = SceneObject::group("Cycle A");
    let mut cyclic_b = SceneObject::group("Cycle B");
    cyclic_a.parent_id = Some(cyclic_b.object_id);
    cyclic_b.parent_id = Some(cyclic_a.object_id);
    let mut self_parent = SceneObject::group("Self parent");
    self_parent.parent_id = Some(self_parent.object_id);
    let mut locked = SceneObject::group("Locked parent");
    locked.locked = true;
    let mut child_of_locked = SceneObject::group("Child of locked parent");
    child_of_locked.parent_id = Some(locked.object_id);
    for objects in [
        vec![],
        vec![SceneObject::group("Valid prefix"), missing_parent],
        vec![child_of_instance, instance],
        vec![cyclic_a, cyclic_b],
        vec![self_parent],
        vec![child_of_locked, locked],
    ] {
        let mut candidate = state.clone();
        let request = SceneCommandRequest {
            command_id: Uuid::new_v4(),
            expected_revision: 0,
            command: SceneCommand::AddObjects { objects },
        };
        assert!(candidate.apply(&request, &assets).is_err());
        assert_eq!(candidate, state);
    }
}

#[test]
fn batch_preserves_existing_parent_locks_and_rejects_excessive_depth() {
    let (mut state, assets, _) = setup();
    let mut locked = SceneObject::group("Existing locked root");
    locked.locked = true;
    let locked_id = locked.object_id;
    run(
        &mut state,
        &assets,
        SceneCommand::AddObject { object: locked },
    );
    let mut child = SceneObject::group("Child");
    child.parent_id = Some(locked_id);
    let before = state.clone();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: state.document.revision,
        command: SceneCommand::AddObjects {
            objects: vec![child],
        },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::LockedObject(id)) if id == locked_id
    ));
    assert_eq!(state, before);
    let mut hierarchy = Vec::new();
    let mut parent_id = None;
    for index in 0..65 {
        let mut group = SceneObject::group(format!("Level {index}"));
        group.parent_id = parent_id;
        parent_id = Some(group.object_id);
        hierarchy.push(group);
    }
    hierarchy.reverse();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: state.document.revision,
        command: SceneCommand::AddObjects { objects: hierarchy },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::Limit(_))
    ));
    assert_eq!(state, before);
}

#[test]
fn batch_obeys_live_object_and_retained_identity_limits() {
    let (mut state, assets, _) = setup();
    let objects = (0..MAX_SCENE_OBJECTS)
        .map(|index| SceneObject::group(format!("Object {index}")))
        .collect::<Vec<_>>();
    run(&mut state, &assets, SceneCommand::AddObjects { objects });
    assert_eq!(state.document.objects.len(), MAX_SCENE_OBJECTS);
    let before = state.clone();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: state.document.revision,
        command: SceneCommand::AddObjects {
            objects: vec![SceneObject::group("Excess object")],
        },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::Limit(_))
    ));
    assert_eq!(state, before);

    // Valid retained identities for absent objects consume the independent identity budget.
    let mut envelope = serde_json::to_value(SceneState::new(Uuid::new_v4())).unwrap();
    let identities = envelope["object_identities"].as_object_mut().unwrap();
    for _ in 0..MAX_OBJECT_IDENTITIES - 1 {
        identities.insert(
            Uuid::new_v4().to_string(),
            serde_json::json!({"kind": "group", "asset_id": null}),
        );
    }
    let mut state: SceneState = serde_json::from_value(envelope).unwrap();
    state.validate(&assets).unwrap();
    let before = state.clone();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: 0,
        command: SceneCommand::AddObjects {
            objects: vec![SceneObject::group("One"), SceneObject::group("Two")],
        },
    };
    assert!(matches!(
        state.apply(&request, &assets),
        Err(SceneError::Limit(_))
    ));
    assert_eq!(state, before);
    run(
        &mut state,
        &assets,
        SceneCommand::AddObjects {
            objects: vec![SceneObject::group("Last identity")],
        },
    );
    assert_eq!(state.document.objects.len(), 1);
}

#[test]
fn source_node_identity_cannot_change_in_document_history_or_ledger() {
    let (mut state, assets, mut object) = setup();
    object.asset_node_index = Some(3);
    let id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    run(
        &mut state,
        &assets,
        SceneCommand::RenameObject {
            object_id: id,
            name: "Renamed source node".into(),
        },
    );
    run(
        &mut state,
        &assets,
        SceneCommand::SetDisplay {
            object_id: id,
            visible: false,
            opacity: 0.5,
        },
    );
    run(&mut state, &assets, SceneCommand::Undo {});
    let original = serde_json::to_value(&state).unwrap();
    for selector in [serde_json::json!(4), serde_json::Value::Null] {
        for location in [
            "document",
            "undo_history",
            "redo_history",
            "object_identities",
        ] {
            let mut damaged = original.clone();
            match location {
                "document" => {
                    damaged[location]["objects"][id.to_string()]["asset_node_index"] =
                        selector.clone()
                }
                "undo_history" => {
                    damaged[location][1]["objects"][id.to_string()]["asset_node_index"] =
                        selector.clone()
                }
                "redo_history" => {
                    damaged[location][0]["objects"][id.to_string()]["asset_node_index"] =
                        selector.clone()
                }
                "object_identities" => {
                    damaged[location][id.to_string()]["asset_node_index"] = selector.clone()
                }
                _ => unreachable!(),
            }
            let altered: SceneState = serde_json::from_value(damaged).unwrap();
            assert!(
                matches!(
                    altered.validate(&assets),
                    Err(SceneError::ObjectIdentityConflict(object_id)) if object_id == id
                ),
                "identity edit accepted in {location}"
            );
        }
    }
}

#[test]
fn groups_cannot_select_asset_nodes_and_instances_still_require_assets() {
    let (state, assets, mut instance) = setup();
    let mut group = SceneObject::group("Invalid node group");
    group.asset_node_index = Some(0);
    instance.asset_id = None;
    instance.asset_node_index = Some(2);
    for object in [group, instance] {
        let mut candidate = state.clone();
        let request = SceneCommandRequest {
            command_id: Uuid::new_v4(),
            expected_revision: 0,
            command: SceneCommand::AddObject { object },
        };
        assert!(matches!(
            candidate.apply(&request, &assets),
            Err(SceneError::Invalid(_))
        ));
        assert_eq!(candidate, state);
    }
    let mut envelope = serde_json::to_value(&state).unwrap();
    envelope["object_identities"][Uuid::new_v4().to_string()] =
        serde_json::json!({"kind": "group", "asset_id": null, "asset_node_index": 0});
    let altered: SceneState = serde_json::from_value(envelope).unwrap();
    assert!(matches!(
        altered.validate(&assets),
        Err(SceneError::Invalid(_))
    ));
}

#[test]
fn old_whole_asset_request_and_persisted_receipt_keep_the_exact_fingerprint() {
    use sha2::{Digest, Sha256};
    // Canonical pre-selector request bytes, including original field ordering.
    const LEGACY_REQUEST: &str = r#"{"command_id":"11111111-1111-4111-8111-111111111111","expected_revision":0,"command":{"type":"add_object","object":{"object_id":"22222222-2222-4222-8222-222222222222","name":"Legacy whole asset","kind":"instance","role":"model","parent_id":null,"asset_id":"33333333-3333-4333-8333-333333333333","transform":{"translation":[0.0,0.0,0.0],"rotation":[0.0,0.0,0.0,1.0],"scale":[1.0,1.0,1.0]},"visible":true,"locked":false,"opacity":1.0}}}"#;
    let request: SceneCommandRequest = serde_json::from_str(LEGACY_REQUEST).unwrap();
    assert_eq!(serde_json::to_string(&request).unwrap(), LEGACY_REQUEST);
    let asset_id = Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap();
    let assets = HashSet::from([asset_id]);
    let mut state = SceneState::new(Uuid::new_v4());
    state.apply(&request, &assets).unwrap();
    let mut persisted = serde_json::to_value(&state).unwrap();
    // Use the legacy wire bytes directly, independent of the current request serializer.
    let fingerprint: [u8; 32] = Sha256::digest(LEGACY_REQUEST.as_bytes()).into();
    persisted["command_receipts"][request.command_id.to_string()]["fingerprint"] =
        serde_json::json!(fingerprint);
    let legacy_bytes = serde_json::to_vec(&persisted).unwrap();
    assert!(!String::from_utf8(legacy_bytes.clone())
        .unwrap()
        .contains("asset_node_index"));
    let mut reopened: SceneState = serde_json::from_slice(&legacy_bytes).unwrap();
    reopened.validate(&assets).unwrap();
    let before = reopened.clone();
    assert!(reopened.apply(&request, &assets).unwrap().duplicate);
    assert_eq!(reopened, before);
    let mut changed = request.clone();
    if let SceneCommand::AddObject { object } = &mut changed.command {
        assert_eq!(object.asset_node_index, None);
        object.asset_node_index = Some(0);
    }
    assert!(matches!(
        reopened.apply(&changed, &assets),
        Err(SceneError::CommandConflict(id)) if id == request.command_id
    ));
    assert_eq!(reopened, before);
}

#[test]
fn source_axis_is_undoable_and_does_not_change_source_or_placement() {
    let (mut state, assets, mut child) = setup();
    let mut root = SceneObject::group("Legacy Z-up source");
    root.source_up_axis = Some(UpAxis::Y);
    let root_id = root.object_id;
    child.parent_id = Some(root_id);
    let child_copy = child.clone();
    run(
        &mut state,
        &assets,
        SceneCommand::AddObjects {
            objects: vec![root.clone(), child],
        },
    );
    run(
        &mut state,
        &assets,
        SceneCommand::SetSourceUpAxis {
            object_id: root_id,
            up_axis: UpAxis::Z,
        },
    );
    assert_eq!(
        state.document.objects[&root_id].source_up_axis,
        Some(UpAxis::Z)
    );
    assert_eq!(state.document.objects[&root_id].transform, root.transform);
    assert_eq!(state.document.objects[&child_copy.object_id], child_copy);
    run(&mut state, &assets, SceneCommand::Undo {});
    assert_eq!(
        state.document.objects[&root_id].source_up_axis,
        Some(UpAxis::Y)
    );
    let mut reopened: SceneState =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    reopened.validate(&assets).unwrap();
    run(&mut reopened, &assets, SceneCommand::Redo {});
    assert_eq!(
        reopened.document.objects[&root_id].source_up_axis,
        Some(UpAxis::Z)
    );
    run(
        &mut reopened,
        &assets,
        SceneCommand::SetLocked {
            object_id: child_copy.object_id,
            locked: true,
        },
    );
    let before = serde_json::to_vec(&reopened).unwrap();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: reopened.document.revision,
        command: SceneCommand::SetSourceUpAxis {
            object_id: root_id,
            up_axis: UpAxis::X,
        },
    };
    assert!(reopened.apply(&request, &assets).is_err());
    assert_eq!(serde_json::to_vec(&reopened).unwrap(), before);
}

#[test]
fn source_axis_rejects_instances_and_legacy_documents_remain_compatible() {
    let (mut state, assets, object) = setup();
    let old = serde_json::to_value(&object).unwrap();
    assert!(old.get("source_up_axis").is_none());
    assert_eq!(serde_json::from_value::<SceneObject>(old).unwrap(), object);
    let id = object.object_id;
    run(&mut state, &assets, SceneCommand::AddObject { object });
    let before = serde_json::to_vec(&state).unwrap();
    let request = SceneCommandRequest {
        command_id: Uuid::new_v4(),
        expected_revision: state.document.revision,
        command: SceneCommand::SetSourceUpAxis {
            object_id: id,
            up_axis: UpAxis::Z,
        },
    };
    assert!(state.apply(&request, &assets).is_err());
    assert_eq!(serde_json::to_vec(&state).unwrap(), before);
}

#[test]
fn source_axis_wire_contract_is_lowercase() {
    use agentic_cad_backend::scene_document::UpAxis;
    for (axis, wire) in [(UpAxis::X, "x"), (UpAxis::Y, "y"), (UpAxis::Z, "z")] {
        assert_eq!(serde_json::to_value(axis).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<UpAxis>(serde_json::json!(wire)).unwrap(),
            axis
        );
    }
}
