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
        [Parameter(Mandatory)][AllowEmptyString()][string]$Old,
        [Parameter(Mandatory)][AllowEmptyString()][string]$New
    )
    if (-not $Content.Contains($Old, [StringComparison]::Ordinal)) {
        throw "$Name is missing the expected hardening anchor."
    }
    $Content.Replace($Old, $New)
}

$htmlPath = 'public/index.html'
$html = Read-SourceFile $htmlPath
$controlPattern = '(?m)^\s*<input id="vlmEndpoint"[^>]*>\r?\n\s*<input id="vlmModel"[^>]*>\r?\n\s*<input id="vlmApiKey"[^>]*>'
$replacement = '                                        <div id="vlmRuntimeStatus" class="inline-note">Local VLM connection, model, and credentials are managed by the 3DMk runtime.</div>'
$patchedHtml = [regex]::Replace($html, $controlPattern, $replacement, 1)
if ($patchedHtml -eq $html -and -not $html.Contains('id="vlmRuntimeStatus"', [StringComparison]::Ordinal)) {
    throw 'VLM connection controls were not replaced by the managed-runtime status.'
}
$html = $patchedHtml
$html = Replace-Required 'VLM control enablement' $html @'
            ['vlmEndpoint', 'vlmModel', 'vlmApiKey', 'vlmCandidateLabels', 'vlmConfidence'].forEach(id => { $(id).disabled = !vlmEnabled; });
'@ @'
            ['vlmCandidateLabels', 'vlmConfidence'].forEach(id => { $(id).disabled = !vlmEnabled; });
            $('vlmRuntimeStatus').textContent = vlmEnabled
                ? 'Local VLM connection, model, and credentials are managed by the 3DMk runtime.'
                : 'Deterministic perception selected; the managed local VLM will not be invoked.';
'@
$html = Replace-Required 'VLM request options' $html @'
            return {
                endpoint: $('vlmEndpoint').value.trim(),
                model: $('vlmModel').value.trim(),
                api_key: $('vlmApiKey').value,
                candidate_labels: $('vlmCandidateLabels').value.split(',').map(label => label.trim()).filter(Boolean),
                confidence_threshold: Number($('vlmConfidence').value)
            };
'@ @'
            return {
                candidate_labels: $('vlmCandidateLabels').value.split(',').map(label => label.trim()).filter(Boolean),
                confidence_threshold: Number($('vlmConfidence').value)
            };
'@
Write-SourceFile $htmlPath $html

$uiTestPath = 'tests/vwm_advanced_ui.rs'
$uiTest = Read-SourceFile $uiTestPath
$uiTest = Replace-Required 'VLM UI control inventory' $uiTest @'
        "vwmPerceptionMode",
        "vlmEndpoint",
        "vlmModel",
        "vlmConfidence",
'@ @'
        "vwmPerceptionMode",
        "vlmRuntimeStatus",
        "vlmConfidence",
'@
$uiTest = Replace-Required 'VLM UI security assertions' $uiTest @'
    assert!(html.contains("form.append('vlm_options'"));
    assert!(html.contains("vlm-object-detection"));
'@ @'
    assert!(html.contains("form.append('vlm_options'"));
    assert!(html.contains("Local VLM connection, model, and credentials are managed by the 3DMk runtime."));
    assert!(!html.contains("id=\"vlmEndpoint\""));
    assert!(!html.contains("id=\"vlmModel\""));
    assert!(!html.contains("id=\"vlmApiKey\""));
    assert!(!html.contains("api_key: $('vlmApiKey').value"));
    assert!(html.contains("vlm-object-detection"));
'@
Write-SourceFile $uiTestPath $uiTest

$visionPath = 'src/ai_vision.rs'
$vision = Read-SourceFile $visionPath
$vision = Replace-Required 'Mistral endpoint helper' $vision @'
fn mistral_endpoint() -> String {
    std::env::var("MISTRAL_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
        .trim_end_matches('/')
        .to_string()
}
'@ @'
fn local_mistral_origin(endpoint: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(endpoint.trim().trim_end_matches('/')).ok()?;
    if url.scheme() != "http" {
        return None;
    }
    if !matches!(url.host_str().unwrap_or_default(), "127.0.0.1" | "localhost" | "::1") {
        return None;
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return None;
    }
    url.set_path("");
    Some(url.to_string().trim_end_matches('/').to_string())
}

fn mistral_endpoint() -> String {
    std::env::var("MISTRAL_ENDPOINT")
        .ok()
        .and_then(|endpoint| local_mistral_origin(&endpoint))
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string())
}
'@
$vision = Replace-Required 'Mistral endpoint tests' $vision @'
    #[test]
    fn vlm_endpoint_is_loopback_only() {
'@ @'
    #[test]
    fn mistral_endpoint_origin_is_loopback_only() {
        for endpoint in [
            "http://127.0.0.1:8080",
            "http://localhost:8080",
            "http://[::1]:8080/",
        ] {
            assert!(local_mistral_origin(endpoint).is_some(), "{endpoint}");
        }
        for endpoint in [
            "https://127.0.0.1:8080",
            "http://example.com:8080",
            "http://user:password@127.0.0.1:8080",
            "http://127.0.0.1:8080/proxy",
            "http://127.0.0.1:8080?target=remote",
            "file:///tmp/model",
        ] {
            assert!(local_mistral_origin(endpoint).is_none(), "{endpoint}");
        }
    }

    #[test]
    fn vlm_endpoint_is_loopback_only() {
'@
Write-SourceFile $visionPath $vision

$perceptionPath = 'src/perception.rs'
$perception = Read-SourceFile $perceptionPath
$perception = Replace-Required 'perception resource constants' $perception @'
use vwm_perception::{
    ColorRegionConfig, ColorRegionSegmenter, ImageFrame, PerceptionBatch, PerceptionInput,
    PerceptionPipeline, PerceptionPipelineConfig, ShapeClassifier,
};
'@ @'
use vwm_perception::{
    ColorRegionConfig, ColorRegionSegmenter, ImageFrame, PerceptionBatch, PerceptionInput,
    PerceptionPipeline, PerceptionPipelineConfig, ShapeClassifier,
};

pub const MAX_VWM_PERCEPTION_IMAGE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_VWM_PERCEPTION_PIXELS: u64 = 40_000_000;
'@
$perception = Replace-Required 'perception decode budget' $perception @'
    let options = options.validate()?;
    let rgba = image::load_from_memory(bytes)
        .context("VWM perception could not decode the supplied image")?
        .to_rgba8();
    let (width, height) = rgba.dimensions();
'@ @'
    let options = options.validate()?;
    if bytes.len() > MAX_VWM_PERCEPTION_IMAGE_BYTES {
        anyhow::bail!("VWM perception image exceeds the compressed-byte budget");
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .context("VWM perception could not determine the supplied image format")?;
    let (width, height) = reader
        .into_dimensions()
        .context("VWM perception could not read the supplied image dimensions")?;
    validate_image_dimensions(width, height)?;
    let rgba = image::load_from_memory(bytes)
        .context("VWM perception could not decode the supplied image")?
        .to_rgba8();
'@
$perception = Replace-Required 'perception validation helper insertion' $perception @'
pub fn recognize_image(source_id: &str, bytes: &[u8]) -> Result<PerceptionBatch> {
    recognize_image_with_options(source_id, bytes, VwmPerceptionOptions::default())
}
'@ @'
fn validate_image_dimensions(width: u32, height: u32) -> Result<()> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .context("VWM perception image dimensions overflow")?;
    if width == 0 || height == 0 || pixels > MAX_VWM_PERCEPTION_PIXELS {
        anyhow::bail!("VWM perception image exceeds the decoded-pixel budget");
    }
    Ok(())
}

pub fn recognize_image(source_id: &str, bytes: &[u8]) -> Result<PerceptionBatch> {
    recognize_image_with_options(source_id, bytes, VwmPerceptionOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compressed_and_decoded_perception_budgets_fail_closed() {
        let oversized = vec![0_u8; MAX_VWM_PERCEPTION_IMAGE_BYTES + 1];
        let error = recognize_image("oversized", &oversized).unwrap_err().to_string();
        assert!(error.contains("compressed-byte budget"));
        assert!(validate_image_dimensions(8_000, 5_000).is_ok());
        assert!(validate_image_dimensions(8_001, 5_000).is_err());
        assert!(validate_image_dimensions(0, 1).is_err());
    }
}
'@
Write-SourceFile $perceptionPath $perception

$apiPath = 'src/api.rs'
$api = Read-SourceFile $apiPath
$api = Replace-Required 'perception route body constant' $api @'
const MAX_CALIBRATED_COMPRESSED_IMAGE_BYTES: usize = 192 * 1024 * 1024;
'@ @'
const MAX_CALIBRATED_COMPRESSED_IMAGE_BYTES: usize = 192 * 1024 * 1024;
const MAX_VWM_PERCEPTION_ROUTE_BODY_BYTES: usize =
    perception::MAX_VWM_PERCEPTION_IMAGE_BYTES + 512 * 1024;
'@
$api = Replace-Required 'perception route body gate' $api @'
        .route("/api/vwm-perception", post(handle_vwm_perception))
'@ @'
        .route(
            "/api/vwm-perception",
            post(handle_vwm_perception)
                .layer(DefaultBodyLimit::max(MAX_VWM_PERCEPTION_ROUTE_BODY_BYTES)),
        )
'@
Write-SourceFile $apiPath $api

Push-Location $RepositoryRoot
try {
    cargo fmt --all
    if ($LASTEXITCODE -ne 0) { throw "Root rustfmt failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

Write-Host '3DMK_SPATIAL_VWM_FINAL_HARDENING_OK'
