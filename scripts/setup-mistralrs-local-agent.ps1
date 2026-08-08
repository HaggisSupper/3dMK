[CmdletBinding()]
param(
    [string]$Version = 'master',
    [switch]$ForceSourceBuild,
    [switch]$AllowCpuFallback
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$InstallStateRoot = Join-Path $env:LOCALAPPDATA 'mistralrs-source'
$CudaMarkerPath = Join-Path $InstallStateRoot 'cuda-install.json'

function Write-Step {
    param([string]$Message)
    Write-Host "[mistral.rs setup] $Message"
}

function Get-CommandPath {
    param([Parameter(Mandatory)][string]$Name)
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    return $null
}

function Invoke-Native {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter(ValueFromRemainingArguments)][string[]]$Arguments
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code $LASTEXITCODE: $FilePath $($Arguments -join ' ')"
    }
}

function Get-MistralDoctorText {
    $mistral = Get-CommandPath 'mistralrs'
    if (-not $mistral) { return '' }
    return (& $mistral doctor 2>&1 | Out-String)
}

function Test-CudaMistralRs {
    $mistral = Get-CommandPath 'mistralrs'
    if (-not $mistral) { return $false }

    if (Test-Path -LiteralPath $CudaMarkerPath) {
        try {
            $marker = Get-Content -LiteralPath $CudaMarkerPath -Raw | ConvertFrom-Json
            $resolvedBinary = (Resolve-Path -LiteralPath $mistral).Path
            $binaryHash = (Get-FileHash -LiteralPath $resolvedBinary -Algorithm SHA256).Hash
            if ($marker.binary_path -eq $resolvedBinary -and $marker.binary_sha256 -eq $binaryHash) {
                Write-Step "Verified the CUDA source-build marker for $resolvedBinary."
                return $true
            }
        }
        catch {
            Write-Warning "Ignoring an invalid Mistral.rs CUDA marker: $($_.Exception.Message)"
        }
    }

    $doctor = Get-MistralDoctorText
    if ($doctor) { Write-Host $doctor.TrimEnd() }
    return $doctor -match '(?im)(build features|accelerator|backend).{0,120}\bcuda\b'
}

function Write-CudaInstallMarker {
    param(
        [Parameter(Mandatory)][string]$BinaryPath,
        [Parameter(Mandatory)][string]$SourceCommit,
        [Parameter(Mandatory)][string]$FeatureSet
    )
    New-Item -ItemType Directory -Force -Path $InstallStateRoot | Out-Null
    $resolvedBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
    [ordered]@{
        schema_version = 1
        binary_path = $resolvedBinary
        binary_sha256 = (Get-FileHash -LiteralPath $resolvedBinary -Algorithm SHA256).Hash
        source_commit = $SourceCommit
        features = $FeatureSet
        installed_at_utc = [DateTime]::UtcNow.ToString('o')
    } | ConvertTo-Json | Set-Content -LiteralPath $CudaMarkerPath -Encoding utf8
}

function Get-RustVersion {
    $rustc = Get-CommandPath 'rustc'
    if (-not $rustc) { return $null }
    $text = (& $rustc --version 2>&1 | Out-String).Trim()
    if ($text -match 'rustc\s+(\d+)\.(\d+)\.(\d+)') {
        return [version]::new([int]$Matches[1], [int]$Matches[2], [int]$Matches[3])
    }
    return $null
}

function Test-VisualStudioBuildTools {
    $roots = @(${env:ProgramFiles(x86)}, $env:ProgramFiles) | Where-Object { $_ }
    foreach ($root in $roots) {
        $vswhere = Join-Path $root 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (-not (Test-Path -LiteralPath $vswhere)) { continue }
        $installation = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
        if ($installation) { return $true }
    }
    return $false
}

function Install-CpuFallback {
    Write-Step 'Installing the official Windows CPU binary as an explicitly allowed degraded fallback.'
    if ($Version -and $Version -ne 'master') {
        $env:MISTRALRS_INSTALL_TAG = $Version
    }
    try {
        $installer = Invoke-RestMethod -Uri 'https://raw.githubusercontent.com/EricLBuehler/mistral.rs/master/install.ps1'
        Invoke-Expression $installer
    }
    finally {
        Remove-Item Env:MISTRALRS_INSTALL_TAG -ErrorAction SilentlyContinue
    }
    $managedBin = Join-Path $env:USERPROFILE '.local\bin'
    if (Test-Path -LiteralPath $managedBin) {
        $env:PATH = "$managedBin;$env:PATH"
    }
    if (-not (Get-CommandPath 'mistralrs')) {
        throw 'The official Mistral.rs installer completed without placing mistralrs on PATH.'
    }
    Remove-Item -LiteralPath $CudaMarkerPath -Force -ErrorAction SilentlyContinue
    $doctor = Get-MistralDoctorText
    if ($doctor) { Write-Host $doctor.TrimEnd() }
    Write-Warning 'Mistral.rs is installed without CUDA. Local agent execution will be materially slower.'
}

$mistral = Get-CommandPath 'mistralrs'
if ($mistral -and -not $ForceSourceBuild) {
    Write-Step "Existing binary found at $mistral."
    if (Test-CudaMistralRs) {
        Write-Step 'Existing installation is confirmed as the CUDA source build; no rebuild is required.'
        exit 0
    }
    if ($AllowCpuFallback) {
        Write-Warning 'Existing Mistral.rs installation is not CUDA-confirmed; retaining it because -AllowCpuFallback was supplied.'
        exit 0
    }
    Write-Step 'Existing installation is not CUDA-confirmed; preparing a CUDA source build.'
}

$missing = [System.Collections.Generic.List[string]]::new()
foreach ($commandName in @('git', 'cargo', 'rustc', 'nvidia-smi', 'nvcc')) {
    if (-not (Get-CommandPath $commandName)) { $missing.Add($commandName) }
}
if (-not (Test-VisualStudioBuildTools)) {
    $missing.Add('Visual Studio 2022 C++ Build Tools')
}

$rustVersion = Get-RustVersion
if (-not $rustVersion) {
    if (-not $missing.Contains('rustc')) { $missing.Add('Rust 1.94+') }
}
elseif ($rustVersion -lt [version]'1.94.0') {
    $missing.Add("Rust 1.94+ (found $rustVersion)")
}

if ($missing.Count -gt 0) {
    if ($AllowCpuFallback) {
        Write-Warning ("CUDA source build prerequisites are missing: {0}" -f ($missing -join ', '))
        Install-CpuFallback
        exit 0
    }
    $instructions = @(
        'A native Windows CUDA build cannot start because these prerequisites are missing:',
        ($missing | ForEach-Object { "  - $_" }),
        '',
        'Install Rust 1.94+, Visual Studio 2022 C++ Build Tools, the NVIDIA driver, and the CUDA toolkit.',
        'Then rerun this script. Add -AllowCpuFallback only when CPU inference is deliberately acceptable.'
    ) -join [Environment]::NewLine
    throw $instructions
}

$sourceRoot = Join-Path $InstallStateRoot ($Version -replace '[^A-Za-z0-9_.-]', '_')
New-Item -ItemType Directory -Force -Path $InstallStateRoot | Out-Null

if (-not (Test-Path -LiteralPath (Join-Path $sourceRoot '.git'))) {
    if (Test-Path -LiteralPath $sourceRoot) {
        throw "Refusing to replace non-git directory: $sourceRoot"
    }
    Write-Step "Cloning Mistral.rs ref $Version into $sourceRoot."
    try {
        Invoke-Native (Get-CommandPath 'git') clone --depth 1 --branch $Version https://github.com/EricLBuehler/mistral.rs.git $sourceRoot
    }
    catch {
        if ($Version -eq 'master') { throw }
        Write-Warning "Ref '$Version' was unavailable. Falling back to current master for this experiment."
        Invoke-Native (Get-CommandPath 'git') clone --depth 1 --branch master https://github.com/EricLBuehler/mistral.rs.git $sourceRoot
    }
}
else {
    Write-Step "Refreshing existing Mistral.rs source at $sourceRoot."
    Invoke-Native (Get-CommandPath 'git') -C $sourceRoot fetch --tags --prune origin
    if ($Version -eq 'master') {
        Invoke-Native (Get-CommandPath 'git') -C $sourceRoot checkout master
        Invoke-Native (Get-CommandPath 'git') -C $sourceRoot pull --ff-only origin master
    }
    else {
        try {
            Invoke-Native (Get-CommandPath 'git') -C $sourceRoot checkout --detach $Version
        }
        catch {
            Write-Warning "Ref '$Version' was unavailable in the existing checkout. Using origin/master."
            Invoke-Native (Get-CommandPath 'git') -C $sourceRoot checkout --detach origin/master
        }
    }
}

$sourceCommit = (& (Get-CommandPath 'git') -C $sourceRoot rev-parse HEAD 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or -not $sourceCommit) {
    throw 'Could not resolve the Mistral.rs source commit.'
}
Write-Step "Building source commit $sourceCommit."

$env:CARGO_NET_GIT_FETCH_WITH_CLI = 'true'
$env:RUST_BACKTRACE = '1'
$cargo = Get-CommandPath 'cargo'
$featureSet = 'cuda flash-attn cudnn'

Write-Step 'Building Mistral.rs with CUDA, FlashAttention, and cuDNN.'
$fullFeatureBuildPassed = $true
try {
    Invoke-Native $cargo install --path (Join-Path $sourceRoot 'mistralrs-cli') --locked --features $featureSet --force
}
catch {
    $fullFeatureBuildPassed = $false
    Write-Warning "The full CUDA feature build failed: $($_.Exception.Message)"
}

if (-not $fullFeatureBuildPassed) {
    $featureSet = 'cuda'
    Write-Step 'Retrying the CUDA build without optional FlashAttention/cuDNN integrations.'
    try {
        Invoke-Native $cargo install --path (Join-Path $sourceRoot 'mistralrs-cli') --locked --features $featureSet --force
    }
    catch {
        if ($AllowCpuFallback) {
            Write-Warning "The CUDA-only build also failed: $($_.Exception.Message)"
            Install-CpuFallback
            exit 0
        }
        throw 'Mistral.rs failed to compile with both the full CUDA feature set and the CUDA-only fallback.'
    }
}

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path -LiteralPath $cargoBin) {
    $env:PATH = "$cargoBin;$env:PATH"
}
$mistral = Get-CommandPath 'mistralrs'
if (-not $mistral) {
    throw 'Cargo completed but mistralrs is not available on PATH.'
}
Write-CudaInstallMarker -BinaryPath $mistral -SourceCommit $sourceCommit -FeatureSet $featureSet
if (-not (Test-CudaMistralRs)) {
    throw 'The CUDA installation marker could not be verified against the installed Mistral.rs binary.'
}

Write-Step "CUDA-enabled Mistral.rs is ready from source commit $sourceCommit with features '$featureSet'."
