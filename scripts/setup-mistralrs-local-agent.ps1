[CmdletBinding()]
param(
    [string]$Version = 'v0.9.0',
    [switch]$ForceSourceBuild,
    [switch]$AllowCpuFallback
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

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
    $doctor = Get-MistralDoctorText
    if (-not $doctor) { return $false }
    Write-Host $doctor.TrimEnd()
    return $doctor -match '(?im)build features:.*\bcuda\b'
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
    $roots = @(
        ${env:ProgramFiles(x86)},
        $env:ProgramFiles
    ) | Where-Object { $_ }
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
    $doctor = Get-MistralDoctorText
    Write-Host $doctor.TrimEnd()
    Write-Warning 'Mistral.rs is installed without CUDA. Local agent execution will be materially slower.'
}

$mistral = Get-CommandPath 'mistralrs'
if ($mistral -and -not $ForceSourceBuild) {
    Write-Step "Existing binary found at $mistral."
    if (Test-CudaMistralRs) {
        Write-Step 'Existing installation reports CUDA support; no rebuild is required.'
        exit 0
    }
    if ($AllowCpuFallback) {
        Write-Warning 'Existing Mistral.rs installation is CPU-only; retaining it because -AllowCpuFallback was supplied.'
        exit 0
    }
    Write-Step 'Existing installation is CPU-only; preparing a CUDA source build.'
}

$missing = [System.Collections.Generic.List[string]]::new()
foreach ($commandName in @('git', 'cargo', 'rustc', 'nvidia-smi', 'nvcc')) {
    if (-not (Get-CommandPath $commandName)) { $missing.Add($commandName) }
}
if (-not (Test-VisualStudioBuildTools)) {
    $missing.Add('Visual Studio 2022 C++ Build Tools')
}

$rustVersion = Get-RustVersion
if ($rustVersion -and $rustVersion -lt [version]'1.94.0') {
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
        'Then rerun this script. Use -AllowCpuFallback only when CPU inference is acceptable.'
    ) -join [Environment]::NewLine
    throw $instructions
}

$sourceParent = Join-Path $env:LOCALAPPDATA 'mistralrs-source'
$sourceRoot = Join-Path $sourceParent ($Version -replace '[^A-Za-z0-9_.-]', '_')
New-Item -ItemType Directory -Force -Path $sourceParent | Out-Null

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

$env:CARGO_NET_GIT_FETCH_WITH_CLI = 'true'
$env:RUST_BACKTRACE = '1'
$cargo = Get-CommandPath 'cargo'

Write-Step 'Building Mistral.rs with CUDA, FlashAttention, and cuDNN.'
$fullFeatureBuildPassed = $true
try {
    Invoke-Native $cargo install --path (Join-Path $sourceRoot 'mistralrs-cli') --locked --features 'cuda flash-attn cudnn' --force
}
catch {
    $fullFeatureBuildPassed = $false
    Write-Warning "The full CUDA feature build failed: $($_.Exception.Message)"
}

if (-not $fullFeatureBuildPassed) {
    Write-Step 'Retrying the CUDA build without optional FlashAttention/cuDNN integrations.'
    try {
        Invoke-Native $cargo install --path (Join-Path $sourceRoot 'mistralrs-cli') --locked --features 'cuda' --force
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
if (-not (Get-CommandPath 'mistralrs')) {
    throw 'Cargo completed but mistralrs is not available on PATH.'
}
if (-not (Test-CudaMistralRs)) {
    throw 'The built Mistral.rs binary did not report CUDA in its compiled feature set.'
}

Write-Step 'CUDA-enabled Mistral.rs installation is ready.'
