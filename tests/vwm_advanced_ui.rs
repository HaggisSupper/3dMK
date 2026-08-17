use agentic_cad_backend::point_cloud::{VwmReconstructionOptions, VwmSurfaceExtraction};

#[test]
fn slide_outs_follow_the_unzoomed_live_viewport_height() {
    let html = include_str!("../public/index.html");

    assert!(html.contains("--app-viewport-width"));
    assert!(html.contains("--app-viewport-height"));
    assert!(html.contains("function syncSlideOutViewport()"));
    assert!(html.contains("width: var(--app-viewport-width, 100vw)"));
    assert!(html.contains("position: fixed; top: 0; left: 0"));
    assert!(html.contains("viewportWidth / Math.max(0.01, Number(uiScale) || 1)"));
    assert!(html.contains("viewportHeight / Math.max(0.01, Number(uiScale) || 1)"));
    assert!(html.contains("window.visualViewport?.addEventListener('resize', syncSlideOutViewport"));
    assert!(html.contains("#helpDrawer, #viewDrawer { position: fixed; top: 0; bottom: 0;"));
    assert!(html.contains("#sidebar { position: fixed; top: 0; bottom: 0;"));
    assert!(!html.contains("body.sidebar-open #canvas-container { width:"));
    assert!(!html.contains("body.help-open:not(.sidebar-open) #canvas-container"));
    assert!(!html.contains("body.sidebar-open.help-open #canvas-container"));
}

#[test]
fn all_operational_vwm_controls_are_exposed_and_wired() {
    let html = include_str!("../public/index.html");
    for id in [
        "vwmPatchAngle",
        "vwmMinPatchFaces",
        "vwmPlaneMinVertices",
        "vwmParallelAngle",
        "vwmPerpendicularAngle",
        "vwmAdjacencyDistance",
        "vwmCoplanarOffset",
        "vwmReconstructionBackend",
        "vwmSurfaceExtraction",
        "vwmSampleLimit",
        "vwmScreening",
        "vwmDensityDepth",
        "vwmMaxDepth",
        "vwmRelaxIterations",
        "vwmSurfaceNetsResolution",
        "vwmBackgroundColor",
        "vwmChannelTolerance",
        "vwmMinimumPixels",
        "vwmEightConnected",
        "vwmCropPadding",
        "floatingConnectionScale",
        "floatingSceneRatio",
        "floatingPrimaryRatio",
        "floatingExtentRatio",
        "floatingSeparationRatio",
        "vwmPerceptionMode",
        "vlmRuntimeStatus",
        "vlmRuntimeCard",
        "vlmRuntimeDetails",
        "vlmCheckBtn",
        "vlmConfidence",
    ] {
        assert!(html.contains(&format!("id=\"{id}\"")), "missing {id}");
    }
    assert!(html.contains("fetchJson('/api/vwm-geometry-analyze'"));
    assert!(html.contains("form.append('vwm_options'"));
    assert!(html.contains("value=\"surface_nets\""));
    assert!(html.contains("form.append('options'"));
    assert!(html.contains("ONNX requires a packaged model manifest"));
    assert!(html.contains("structured-ray extraction require corresponding capture data"));
    assert!(html.contains("form.append('floating_mesh_settings'"));
    assert!(html.contains("form.append('vlm_options'"));
    assert!(html.contains("/api/v1/vlm/status"));
    assert!(html.contains("function checkVlmRuntime()"));
    assert!(html.contains("showLoader(true, 'Preparing VWM geometry analysis…'"));
    assert!(html.contains("setProcessingProgress(42, '42% · Running selected VWM extractor')"));
    assert!(html.contains("data-state=\"checking\""));
    assert!(html
        .contains("Local VLM connection, model, and credentials are managed by the 3DMk runtime."));
    assert!(!html.contains("id=\"vlmEndpoint\""));
    assert!(!html.contains("id=\"vlmModel\""));
    assert!(!html.contains("id=\"vlmApiKey\""));
    assert!(!html.contains("api_key: $('vlmApiKey').value"));
    assert!(html.contains("vlm-object-detection"));
    assert!(html.contains("uniform float uDensity"));
    assert!(html.contains("id=\"strideInput\" min=\"1\" max=\"100\" step=\"1\" value=\"100\""));
    assert!(html.contains("id=\"pointSize\" min=\"0.002\" max=\"0.15\""));
}

#[test]
fn vwm_reconstruction_controls_validate_real_engine_ranges() {
    let defaults = VwmReconstructionOptions::default();
    assert!(defaults.validate().is_ok());

    let surface_nets = VwmReconstructionOptions {
        extraction: VwmSurfaceExtraction::SurfaceNets,
        surface_nets_resolution: 64,
        ..defaults
    };
    assert!(surface_nets.validate().is_ok());

    assert!(VwmReconstructionOptions {
        max_depth: 1,
        ..defaults
    }
    .validate()
    .is_err());
    assert!(VwmReconstructionOptions {
        surface_nets_resolution: 512,
        ..defaults
    }
    .validate()
    .is_err());
}
