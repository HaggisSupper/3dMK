#[test]
fn frontend_has_a_usable_entry_point() {
    let html = include_str!("../public/index.html");

    assert!(html.contains("<aside id=\"sidebar\""));
    assert!(html.contains("<main id=\"canvas-container\""));
    assert!(!html.contains("id=\"emptyState\""));
    assert!(html.contains("/api/health"));
    assert!(html.contains("id=\"pointCloudToMeshBtn\""));
    assert!(html.contains("id=\"meshToPointCloudBtn\""));
    assert!(html.contains("function ensurePlanarUvs"));
    assert!(html.contains("new ResizeObserver(onWindowResize).observe(viewport)"));
    assert!(html.contains("JSZip.loadAsync(file)"));
    assert!(html.contains("accept=\".3ds,.3mf,.dae,.fbx,.glb,.gltf,.off,.obj,.ply,.stl,.u3d,.x3d,.zip\""));
    assert!(html.contains("manager.setURLModifier"));
    assert!(html.contains("new MTLLoader(manager)"));
    assert!(html.contains("currentTexture || geom.userData.texture"));
    assert!(html.contains("function classifyGeometry"));
    assert!(html.contains("/api/point-cloud-analyze"));
    assert!(html.contains("/api/flat-surface-correct"));
    assert!(html.contains("/api/mesh-smooth"));
    assert!(html.contains("/api/model-floorplan-recognize"));
    assert!(html.contains("function geometryPlyBlob"));
    assert!(html.contains("function renderFloorplanLayout"));
    assert!(html.contains("function applyRustTransform"));
    assert!(html.contains("function transferSmoothedPositions"));
    assert!(html.contains("id=\"correctFlatSurfaceBtn\""));
    assert!(html.contains("id=\"recognizeModelFloorplanBtn\""));
    assert!(html.contains("function taubinSmoothGeometry"));
    assert!(html.contains("function centerAndGroundGeometry"));
    assert!(html.contains("function alignLongestAxis"));
    assert!(html.contains("function getLoadedGeometryMode"));
    assert!(html.contains(
        "Loaded data mixes mesh and point-cloud layers. Isolate one mode before converting."
    ));
    assert!(html.contains("function getActiveTexture"));
    assert!(html.contains("function ensureTexturePixels"));
    assert!(html.contains("function sceneMetadataPayload"));
    assert!(html.contains("function rememberSceneAnalysis"));
    assert!(html.contains("function renderSceneAnalysisOverlay"));
    assert!(html.contains("function clearSceneAnalysisOverlay"));
    assert!(html.contains("id=\"floatingMeshReviewList\""));
    assert!(html.contains("id=\"selectAllFloatingBtn\""));
    assert!(html.contains("id=\"hideSelectedFloatingBtn\""));
    assert!(html.contains("id=\"deleteSelectedFloatingBtn\""));
    assert!(html.contains("id=\"saveFloatingEditsBtn\""));
    assert!(html.contains("id=\"discardFloatingEditsBtn\""));
    assert!(html.contains("id=\"floatingMeshContextMenu\""));
    assert!(html.contains("function renderFloatingMeshReviewList"));
    assert!(html.contains("function setFloatingClustersHidden"));
    assert!(html.contains("function stageFloatingClusterDeletion"));
    assert!(html.contains("function discardFloatingMeshEdits"));
    assert!(html.contains("function saveFloatingMeshEdits"));
    assert!(html.contains("function handleFloatingMeshContextMenu"));
    assert!(html.contains("hidden_cluster_ids"));
    assert!(html.contains("pending_deletion_cluster_ids"));
    assert!(html.contains("Detached mesh staging mutated model geometry"));
    assert!(html.contains("Save edits did not squash staged geometry"));
    assert!(html.contains("Save edits did not retain hidden state"));
    assert!(html.contains("Detached mesh undo did not restore staged state"));
    assert!(html.contains("function updateContextHelp"));
    assert!(html.contains("async function buildModelPackage"));
    assert!(html.contains("reference_images: {"));
    assert!(html.contains("references/photos/${index + 1}_"));
    assert!(html.contains("id=\"refinePackageImagesBtn\""));
    assert!(html.contains("async function refinePackageTexture"));
    assert!(html.contains("/api/image-assisted-refinement"));
    assert!(html.contains("id=\"calibratedPhotoProjectionBtn\""));
    assert!(html.contains("id=\"projectionMaxImages\""));
    assert!(html.contains("id=\"projectionOcclusionTolerance\""));
    assert!(html.contains("id=\"projectionMinObservations\""));
    assert!(html.contains("id=\"projectionOriginalColorWeight\""));
    assert!(html.contains("async function projectCalibratedPhotos"));
    assert!(html.contains("/api/calibrated-photo-project"));
    assert!(html.contains("class=\"nested-accordion\""));
    assert!(html.contains("Import geometry or package"));
    assert!(html.contains("id=\"packageImportReview\""));
    assert!(html.contains("id=\"packageModelSelect\""));
    assert!(html.contains("async function inspectZipPackage"));
    assert!(html.contains("/api/v1/projects/import-package"));
    assert!(html.contains("async function loadCommittedRustPackage"));
    assert!(html.contains("Recognition"));
    assert!(html.contains("Room plan"));
    assert!(html.contains("Working package"));
    assert!(html.contains("zip.file('manifest.json'"));
    assert!(html.contains("package_format_version: '3dmk-package-v1'"));
    assert!(html.contains("analysis/scene-analysis.json"));
    assert!(html.contains("transforms/history.json"));
    assert!(html.contains("review/room-plan.json"));
    assert!(html.contains("review/measurement-objects.json"));
    assert!(html.contains("const rememberTextureAsset ="));
    assert!(html.contains("pointGeometry.userData.texture = sourceTexture || null"));
    assert!(html.contains("meshGeometry.userData.texture = sourceTexture || null"));
    assert!(html.contains("Saved ${packaged.filename} from current PLY geometry"));
    assert!(html.contains("Exported ${asset.filename} as a raw 3D model."));
    assert!(html.contains("scene_analysis_context"));
    assert!(html.contains("mesh_smoothing"));
    assert!(html.contains("model_floorplan_recognition"));
    assert!(html.contains("scene_metadata"));
    assert!(html.contains("id=\"helpDrawer\""));
    assert!(html.contains("id=\"helpToggleBtn\""));
    assert!(html.contains("id=\"helpPinBtn\""));
    assert!(html.contains("id=\"measureUnitsSelect\""));
    assert!(html.contains("Measurement display units"));
    assert!(html.contains("saved as its own measurement object"));
    assert!(html.contains("localStorage.getItem(HELP_DRAWER_PIN_KEY)"));
    assert!(html.contains("raw_outer_boundary"));
    assert!(html.contains("outer_walls"));
    assert!(html.contains("outer_corners"));
    assert!(html.contains("Load a mesh to build a room plan."));
    assert!(html.contains("id=\"roomPlanShowWalls\""));
    assert!(html.contains("id=\"roomPlanShowCorners\""));
    assert!(html.contains("id=\"roomPlanShowRooms\""));
    assert!(html.contains("id=\"roomPlanShowRaw\""));
    assert!(html.contains("id=\"clearRoomPlanBtn\""));
    assert!(html.contains("function updateRoomPlanLayerVisibility"));
    assert!(html.contains("function clearFloorplanLayoutOverlay"));
    assert!(html.contains("renderFloorplanLayout(data.layout, { replaceScene: false, cutHeight: Number(data.cut_height) || 0 })"));
    assert!(!html.contains("Floorplan image → STEP"));
    assert!(!html.contains("Load a model, point cloud, or floorplan image"));
    assert!(html.contains("function countLayoutWalls"));
    assert!(html.contains("function countLayoutCorners"));
    assert!(html.contains("sceneAnalysisOverlayGroup"));
    assert!(html.contains("const getAccordionHeader = item =>"));
    assert!(html.contains("function openAccordionPath"));
    assert!(html.contains("function runActionTarget(button)"));
    assert!(html.contains(".accordion-item.active > .accordion-body { display: block; }"));
    assert!(html.contains(".accordion-item.active > .accordion-header::after { content: '−'; }"));
    assert!(html.contains("body.sidebar-open.help-open #historyControls"));
    assert!(html.contains("body.sidebar-open.view-open #historyControls"));
    assert!(!html.contains("handlePrecisionWheel"));
    assert!(!html.contains("function handleTrackpadPan"));
    assert!(!html.contains("id=\"navigationProfile\""));
    assert!(!html.contains("localStorage.getItem('3dmk-navigation-profile'"));
    assert!(!html.contains("localStorage.setItem('3dmk-navigation-profile'"));
    assert!(html.contains("localStorage.removeItem('3dmk-navigation-profile')"));
    assert!(html.contains("controls.mouseButtons.LEFT = THREE.MOUSE.ROTATE"));
    assert!(html.contains("controls.mouseButtons.RIGHT = THREE.MOUSE.PAN"));
    assert!(html.contains("controls.touches.TWO = THREE.TOUCH.DOLLY_PAN"));
    assert!(html.contains("OrbitControls mouse mapping contract failed"));
    assert!(html.contains("OrbitControls touch mapping contract failed"));
    assert!(!html.contains("Native pinch zoom contract failed"));
    assert!(!html.contains(".usdz"), "USDZ is not supported by a loader");
    assert!(!html.to_ascii_lowercase().contains("splat"));
    assert!(!html.to_ascii_lowercase().contains("voxelize"));
    assert!(!html.to_ascii_lowercase().contains("convex mesh"));
}

#[test]
fn calibrated_projection_posts_complete_inputs_and_commits_once() {
    let html = include_str!("../public/index.html");

    for id in [
        "calibratedPhotoProjectionBtn",
        "projectionMaxImages",
        "projectionOcclusionTolerance",
        "projectionMinObservations",
        "projectionOriginalColorWeight",
        "calibratedPhotoProjectionStatus",
    ] {
        assert!(html.contains(&format!("id=\"{id}\"")), "missing {id}");
    }
    assert!(html.contains("safeBind('calibratedPhotoProjectionBtn', 'click', projectCalibratedPhotos)"));
    assert!(html.contains("async function projectCalibratedPhotos"));
    assert!(html.contains("form.append('cloud', geometryPlyBlob(2000000, true"));
    assert!(html.contains("form.append('camera_set'"));
    assert!(html.contains("form.append('photo_manifest'"));
    assert!(html.contains("form.append('geometry_provenance'"));
    assert!(html.contains("form.append('projection_options'"));
    assert!(html.contains("form.append('reference_photo'"));
    assert!(html.contains("fetchJson('/api/calibrated-photo-project'"));
    assert!(html.contains("loadPlyGeometryFromUrl(data.projected_model)"));
    assert!(html.contains("commitGeometryEdit([projectedGeometry]"));
    assert!(html.contains("source_geometry_fingerprint: activeGeometryFingerprint"));
    assert!(html.contains("photoEvidence.length"));
    assert!(html.contains("photoProjection: currentModelPackage.photoProjection || null"));
}

#[test]
fn ply_and_photo_pairing_contract_preserves_source_data() {
    let html = include_str!("../public/index.html");

    assert!(html.contains("const normalizePackageSourcePath ="));
    assert!(html.contains("sourcePath: packageAssetSourcePath(asset)"));
    assert!(html.contains("camera_id: camera.id"));
    assert!(html.contains("normalizePackageSourcePath(photo.sourcePath || photo.name)"));
    assert!(html.contains("source_path: referencePhotos[index].sourcePath || file"));
    assert!(html.contains("property float nx"));
    assert!(html.contains("property uchar red"));
    assert!(html.contains("property float s"));
    assert!(html.contains("property list uchar int vertex_indices"));
    assert!(html.contains("concatenateGeometryAttribute('uv'"));
}

#[test]
fn detached_review_uses_revision_bound_exact_membership() {
    let html = include_str!("../public/index.html");

    assert!(html.contains("function geometryConnectedComponents"));
    assert!(html.contains("function buildFloatingComponentMasks"));
    assert!(html.contains("function meshGeometryByFaceMask"));
    assert!(html.contains("function pointGeometryByIndexMask"));
    assert!(html.contains("faceIndices: new Set"));
    assert!(html.contains("pointIndices: new Set"));
    assert!(html.contains("const result = source.clone()"));
    assert!(html.contains("Object.entries(source.attributes)"));
    assert!(html.contains("rebuildFilteredGeometryGroups"));
    assert!(html.contains("removedFaces !== expectedRemoval.faces"));
    assert!(html.contains("removedPoints !== expectedRemoval.points"));
    assert!(!html.contains("function subsetGeometryByBounds"));
    assert!(html.contains("Exact indexed-face removal contract failed"));
    assert!(html.contains("Overlapping bounds removed unrelated component"));
    assert!(html.contains("Component mask attribute preservation failed"));
    assert!(html.contains("Component group preservation failed"));
    assert!(html.contains("Exact point-index mask contract failed"));
    assert!(html.contains("function geometryFingerprint"));
    assert!(html.contains("function requireCurrentFloatingReview"));
    assert!(html.contains("geometry_fingerprint: activeGeometryFingerprint"));
    assert!(html.contains("saveState();"), "hide/show edits must enter history");
}

#[test]
fn divergent_rust_projects_force_current_browser_package() {
    let html = include_str!("../public/index.html");

    assert!(html.contains("currentModelPackage.rustProjectId && !rustProjectHasLocalDivergence"));
    assert!(html.contains("const forcedBrowserPackage"));
    assert!(html.contains("geometryPlyBlob(2000000, true, loadedGeometryStore)"));
    assert!(html.contains("Local geometry or detached-review edits are not committed to the Rust revision"));
    assert!(html.contains("The Rust-backed revision remains unchanged."));
}

#[test]
fn vwm_perception_reports_fallback_provenance_honestly() {
    let html = include_str!("../public/index.html");

    assert!(html.contains("id=\"vwmPerceptionImageInput\""));
    assert!(html.contains("id=\"vwmPerceptionBtn\""));
    assert!(html.contains("safeBind('vwmPerceptionBtn', 'click', runVwmPerception)"));
    assert!(html.contains("async function runVwmPerception"));
    assert!(html.contains("fetchJson('/api/vwm-perception'"));
    assert!(html.contains("const mode = String(data?.mode || 'unspecified')"));
    assert!(html.contains("deterministic fallback was used and packaged ONNX inference was not used"));
    assert!(!html.contains("Packaged ONNX inference ready"));
}
