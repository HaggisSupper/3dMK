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
$api = Replace-Required 'calibrated upload map entry handling' $api @'
        if uploads_by_name.contains_key(&key) {
            errors.push(UnmatchedCalibratedPhoto {
                filename: upload.filename,
                camera_id: String::new(),
                source_path: String::new(),
                reason: "duplicate_upload_filename",
            });
        } else {
            uploads_by_name.insert(key, upload);
        }
'@ @'
        match uploads_by_name.entry(key) {
            std::collections::hash_map::Entry::Occupied(_) => {
                errors.push(UnmatchedCalibratedPhoto {
                    filename: upload.filename,
                    camera_id: String::new(),
                    source_path: String::new(),
                    reason: "duplicate_upload_filename",
                });
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(upload);
            }
        }
'@
$api = Replace-Required 'cleanup capability truth' $api @'
            reason: "Exact component masks and reviewed immutable revision semantics are not implemented.",
'@ @'
            reason: "Transient exact component masks are implemented for review, but reviewed immutable revision publication is not yet implemented.",
'@
Write-SourceFile $apiPath $api

$packagesPath = 'src/packages.rs'
$packages = Read-SourceFile $packagesPath
$packages = Replace-Required 'candidate ordering' $packages @'
    candidates.sort_by(|left, right| right.confidence.cmp(&left.confidence));
'@ @'
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.confidence));
'@
Write-SourceFile $packagesPath $packages

$pointCloudPath = 'src/point_cloud.rs'
$pointCloud = Read-SourceFile $pointCloudPath
$pointCloud = Replace-Required 'reconstruction backend default derive' $pointCloud @'
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VwmReconstructionBackend {
    Auto,
    Pdal,
    Vwm,
}

impl Default for VwmReconstructionBackend {
    fn default() -> Self {
        Self::Auto
    }
}
'@ @'
#[derive(
    Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum VwmReconstructionBackend {
    #[default]
    Auto,
    Pdal,
    Vwm,
}
'@
$pointCloud = Replace-Required 'surface extraction default derive' $pointCloud @'
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VwmSurfaceExtraction {
    Poisson,
    SurfaceNets,
}

impl Default for VwmSurfaceExtraction {
    fn default() -> Self {
        Self::Poisson
    }
}
'@ @'
#[derive(
    Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum VwmSurfaceExtraction {
    #[default]
    Poisson,
    SurfaceNets,
}
'@
$pointCloud = Replace-Required 'polyline loop form' $pointCloud @'
    loop {
        let Some(connections) = point_map.get(&cursor) else {
            break;
        };
        let Some(connection) = connections.iter().find(|connection| !used[connection.idx]) else {
            break;
        };
        used[connection.idx] = true;
        if forward {
            polyline.push(connection.point);
        } else {
            polyline.insert(0, connection.point);
        }
        cursor = connection.other;
    }
'@ @'
    while let Some(connections) = point_map.get(&cursor) {
        let Some(connection) = connections.iter().find(|connection| !used[connection.idx]) else {
            break;
        };
        used[connection.idx] = true;
        if forward {
            polyline.push(connection.point);
        } else {
            polyline.insert(0, connection.point);
        }
        cursor = connection.other;
    }
'@
Write-SourceFile $pointCloudPath $pointCloud

$scenePath = 'src/scene.rs'
$scene = Read-SourceFile $scenePath
$scene = Replace-Required 'linear reconstruction keep-mask lookup' $scene @'
    pub fn reconstruction_keep_mask(&self) -> Vec<bool> {
        self.cluster_by_point
            .iter()
            .map(|cluster_id| {
                self.clusters
                    .iter()
                    .find(|cluster| cluster.id == *cluster_id)
                    .is_none_or(|cluster| cluster.disposition != GeometryDisposition::Remove)
            })
            .collect()
    }
'@ @'
    pub fn reconstruction_keep_mask(&self) -> Vec<bool> {
        let removable_cluster_ids = self
            .clusters
            .iter()
            .filter(|cluster| cluster.disposition == GeometryDisposition::Remove)
            .map(|cluster| cluster.id)
            .collect::<HashSet<_>>();
        self.cluster_by_point
            .iter()
            .map(|cluster_id| !removable_cluster_ids.contains(cluster_id))
            .collect()
    }
'@
Write-SourceFile $scenePath $scene

$workflowPath = '.github/workflows/advanced-feature-integration.yml'
$workflow = Read-SourceFile $workflowPath
$workflow = Replace-Required 'Rust cache workspace paths' $workflow @'
          workspaces: |
            . -> target
            VWM-Repo-Implicit -> VWM-Repo-Implicit/target
            src-tauri -> src-tauri/target
'@ @'
          workspaces: |
            . -> target
            VWM-Repo-Implicit -> target
            src-tauri -> target
'@
$workflow = Replace-Required 'Tauri validation tail' $workflow @'
      - name: Compile Tauri desktop application
        shell: pwsh
        run: cargo check --locked --manifest-path src-tauri/Cargo.toml
'@ @'
      - name: Compile Tauri desktop application
        shell: pwsh
        run: cargo check --locked --manifest-path src-tauri/Cargo.toml

      - name: Validate current documentation contracts
        shell: pwsh
        run: pwsh -NoLogo -NoProfile -File scripts/test-documentation.ps1

      - name: Validate CUDA-only Mistral.rs harness contracts
        shell: pwsh
        run: pwsh -NoLogo -NoProfile -File scripts/test-mistralrs-local-agent-harness.ps1
'@
Write-SourceFile $workflowPath $workflow

Push-Location $RepositoryRoot
try {
    cargo fmt --all
    if ($LASTEXITCODE -ne 0) { throw "Root rustfmt failed with exit code $LASTEXITCODE" }
    cargo fmt --all --manifest-path VWM-Repo-Implicit/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "VWM rustfmt failed with exit code $LASTEXITCODE" }

    cargo clippy --all-targets --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Strict root clippy failed with exit code $LASTEXITCODE" }
    cargo test --all-targets --locked -- --nocapture
    if ($LASTEXITCODE -ne 0) { throw "Root regression tests failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

Write-Host '3DMK_SPATIAL_VWM_QUALITY_REPAIR_OK'
