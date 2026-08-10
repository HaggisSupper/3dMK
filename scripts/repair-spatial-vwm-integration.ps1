[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path

function Read-SourceFile {
    param([Parameter(Mandatory)][string]$RelativePath)
    Get-Content -LiteralPath (Join-Path $RepositoryRoot $RelativePath) -Raw
}

function Write-SourceFile {
    param(
        [Parameter(Mandatory)][string]$RelativePath,
        [Parameter(Mandatory)][string]$Content
    )
    Set-Content -LiteralPath (Join-Path $RepositoryRoot $RelativePath) -Value $Content -Encoding utf8 -NoNewline
}

function Replace-Required {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Content,
        [Parameter(Mandatory)][string]$Old,
        [Parameter(Mandatory)][string]$New
    )
    if (-not $Content.Contains($Old, [StringComparison]::Ordinal)) {
        throw "$Name is missing the expected repair anchor."
    }
    $Content.Replace($Old, $New)
}

$apiPath = 'src/api.rs'
$api = Read-SourceFile $apiPath
$api = Replace-Required 'direct import extension condition' $api @'
                if !matches!(extension.as_str(), "glb" | "ply" | "obj") {
'@ @'
                if !direct_model_extension_supported(&extension) {
'@
$api = Replace-Required 'direct import support message' $api @'
                        "Phase 1 import accepts GLB, PLY, or OBJ. Use a package for dependent files.",
'@ @'
                        "Phase 1 import accepts GLB, GLTF, PLY, OBJ, STL, FBX, DAE, 3DS, 3MF, OFF, U3D, and X3D. Use a package for dependent files.",
'@
$api = Replace-Required 'missing model message' $api @'
            "Choose a GLB, PLY, or OBJ model to import.",
'@ @'
            "Choose a GLB, GLTF, PLY, OBJ, STL, FBX, DAE, 3DS, 3MF, OFF, U3D, or X3D model to import.",
'@
$api = Replace-Required 'model media type contract' $api @'
fn model_media_type(extension: &str) -> &'static str {
    match extension {
        "glb" => "model/gltf-binary",
        "ply" => "application/ply",
        "obj" => "model/obj",
        _ => "application/octet-stream",
    }
}
'@ @'
const DIRECT_MODEL_EXTENSIONS: &[&str] = &[
    "3ds", "3mf", "dae", "fbx", "glb", "gltf", "off", "obj", "ply", "stl", "u3d",
    "x3d",
];

fn direct_model_extension_supported(extension: &str) -> bool {
    DIRECT_MODEL_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
}

fn model_media_type(extension: &str) -> &'static str {
    match extension {
        "3ds" => "application/x-3ds",
        "3mf" => "model/3mf",
        "dae" => "model/vnd.collada+xml",
        "fbx" => "model/fbx",
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        "off" => "model/off",
        "obj" => "model/obj",
        "ply" => "application/ply",
        "stl" => "model/stl",
        "u3d" => "model/u3d",
        "x3d" => "model/x3d+xml",
        _ => "application/octet-stream",
    }
}
'@
$api = Replace-Required 'explicit import units message' $api @'
            "PLY, OBJ, STL, and LAS imports require explicit source units.",
'@ @'
            "GLTF, PLY, OBJ, STL, FBX, DAE, 3DS, 3MF, OFF, U3D, X3D, and LAS imports require explicit source units.",
'@
$api = Replace-Required 'direct import regression test insertion' $api @'
    #[test]
    fn uploaded_filename_cannot_escape_output_directory() {
'@ @'
    #[test]
    fn direct_model_import_contract_preserves_supported_formats() {
        for extension in DIRECT_MODEL_EXTENSIONS {
            assert!(
                direct_model_extension_supported(extension),
                "missing direct import support for {extension}"
            );
            assert_ne!(model_media_type(extension), "application/octet-stream");
        }
        for extension in ["exe", "html", "js", "zip"] {
            assert!(!direct_model_extension_supported(extension));
        }
        assert!(direct_model_extension_supported("GLB"));
    }

    #[test]
    fn uploaded_filename_cannot_escape_output_directory() {
'@
Write-SourceFile $apiPath $api

$visionPath = 'src/ai_vision.rs'
$vision = Read-SourceFile $visionPath
$vision = Replace-Required 'VLM server-owned configuration helpers' $vision @'
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VlmObjectDetectionOptions {
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
'@ @'
fn configured_vlm_endpoint() -> String {
    std::env::var("THREEDMK_VLM_ENDPOINT").unwrap_or_else(|_| mistral_endpoint())
}

fn configured_vlm_model() -> String {
    std::env::var("THREEDMK_VLM_MODEL").unwrap_or_else(|_| "pixtral".to_string())
}

fn configured_vlm_api_key() -> String {
    std::env::var("THREEDMK_VLM_API_KEY").unwrap_or_default()
}

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VlmObjectDetectionOptions {
    #[serde(skip_deserializing, default = "configured_vlm_endpoint")]
    pub endpoint: String,
    #[serde(skip_deserializing, default = "configured_vlm_model")]
    pub model: String,
    #[serde(skip_deserializing, default = "configured_vlm_api_key")]
    pub api_key: String,
'@
$vision = Replace-Required 'VLM default configuration' $vision @'
            endpoint: std::env::var("THREEDMK_VLM_ENDPOINT").unwrap_or_else(|_| mistral_endpoint()),
            model: std::env::var("THREEDMK_VLM_MODEL").unwrap_or_else(|_| "pixtral".to_string()),
            api_key: std::env::var("THREEDMK_VLM_API_KEY").unwrap_or_default(),
'@ @'
            endpoint: configured_vlm_endpoint(),
            model: configured_vlm_model(),
            api_key: configured_vlm_api_key(),
'@
$vision = Replace-Required 'VLM loopback endpoint validator insertion' $vision @'
fn default_detection_confidence() -> f64 {
    0.5
}

pub async fn detect_objects_vlm(
'@ @'
fn default_detection_confidence() -> f64 {
    0.5
}

fn local_vlm_completion_url(endpoint: &str) -> Result<reqwest::Url> {
    let mut url = reqwest::Url::parse(endpoint.trim().trim_end_matches('/'))
        .context("VLM endpoint is not a valid URL")?;
    if url.scheme() != "http" {
        anyhow::bail!("VLM endpoint must use loopback HTTP");
    }
    let host = url.host_str().unwrap_or_default();
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        anyhow::bail!("VLM endpoint must use a loopback host");
    }
    if !url.username().is_empty() || url.password().is_some() {
        anyhow::bail!("VLM endpoint must not contain URL credentials");
    }
    if url.query().is_some() || url.fragment().is_some() {
        anyhow::bail!("VLM endpoint must not contain a query or fragment");
    }
    match url.path().trim_end_matches('/') {
        "" | "/" => url.set_path("/v1/chat/completions"),
        "/v1/chat/completions" | "/chat/completions" => {}
        _ => anyhow::bail!(
            "VLM endpoint must be a loopback server origin or chat-completions endpoint"
        ),
    }
    Ok(url)
}

pub async fn detect_objects_vlm(
'@
$vision = Replace-Required 'VLM server-owned endpoint selection' $vision @'
    options.endpoint = options.endpoint.trim().trim_end_matches('/').to_string();
    options.model = options.model.trim().to_string();
    if !options.endpoint.starts_with("http://") && !options.endpoint.starts_with("https://") {
        anyhow::bail!("VLM endpoint must use http:// or https://");
    }
'@ @'
    options.endpoint = configured_vlm_endpoint().trim().trim_end_matches('/').to_string();
    options.model = configured_vlm_model().trim().to_string();
    options.api_key = configured_vlm_api_key();
    let completion_url = local_vlm_completion_url(&options.endpoint)?;
'@
$vision = Replace-Required 'duplicate completion URL construction' $vision @'
    let completion_url = if options.endpoint.ends_with("/chat/completions") {
        options.endpoint.clone()
    } else {
        format!("{}/v1/chat/completions", options.endpoint)
    };
'@ ''
$vision = Replace-Required 'VLM connection security tests' $vision @'
    #[test]
    fn base64_encode_known() {
'@ @'
    #[test]
    fn vlm_endpoint_is_loopback_only() {
        for endpoint in [
            "http://127.0.0.1:8080",
            "http://localhost:8080/v1/chat/completions",
            "http://[::1]:8080/chat/completions",
        ] {
            assert!(local_vlm_completion_url(endpoint).is_ok(), "{endpoint}");
        }
        for endpoint in [
            "https://127.0.0.1:8080",
            "http://example.com:8080",
            "http://user:password@127.0.0.1:8080",
            "http://127.0.0.1:8080/proxy",
            "file:///tmp/model",
        ] {
            assert!(local_vlm_completion_url(endpoint).is_err(), "{endpoint}");
        }
    }

    #[test]
    fn request_payload_cannot_override_vlm_connection_or_credentials() {
        let options: VlmObjectDetectionOptions = serde_json::from_value(serde_json::json!({
            "endpoint": "http://example.com:9000",
            "model": "remote-model",
            "api_key": "request-secret",
            "candidate_labels": ["door"],
            "confidence_threshold": 0.4
        }))
        .unwrap();
        assert_eq!(options.endpoint, configured_vlm_endpoint());
        assert_eq!(options.model, configured_vlm_model());
        assert_eq!(options.api_key, configured_vlm_api_key());
        assert_ne!(options.api_key, "request-secret");
    }

    #[test]
    fn base64_encode_known() {
'@
Write-SourceFile $visionPath $vision

Push-Location $RepositoryRoot
try {
    cargo fmt --all
    if ($LASTEXITCODE -ne 0) { throw "Root rustfmt failed with exit code $LASTEXITCODE" }
    cargo fmt --all --manifest-path VWM-Repo-Implicit/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "VWM rustfmt failed with exit code $LASTEXITCODE" }

    cargo test --locked direct_model_import_contract_preserves_supported_formats -- --nocapture
    if ($LASTEXITCODE -ne 0) { throw "Direct import regression test failed with exit code $LASTEXITCODE" }
    cargo test --locked vlm_endpoint_is_loopback_only -- --nocapture
    if ($LASTEXITCODE -ne 0) { throw "VLM loopback test failed with exit code $LASTEXITCODE" }
    cargo test --locked request_payload_cannot_override_vlm_connection_or_credentials -- --nocapture
    if ($LASTEXITCODE -ne 0) { throw "VLM request-boundary test failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

Write-Host '3DMK_SPATIAL_VWM_INTEGRATION_REPAIR_OK'
