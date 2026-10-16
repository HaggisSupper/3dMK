[CmdletBinding()]
param(
    [string]$Version = 'master',
    [switch]$ForceSourceBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$InstallStateRoot = if ($env:LOCALAPPDATA) {
    Join-Path $env:LOCALAPPDATA 'mistralrs-source'
}
else {
    Join-Path $env:TEMP 'mistralrs-source'
}
$CudaMarkerPath = Join-Path $InstallStateRoot 'cuda-install.json'

function Write-Step {
    param([Parameter(Mandatory)][string]$Message)
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
        throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $($Arguments -join ' ')"
    }
}

function Get-MistralDoctorText {
    param([Parameter(Mandatory)][string]$MistralPath)
    return (& $MistralPath doctor 2>&1 | Out-String)
}

function Assert-CudaDoctorEvidence {
    param([Parameter(Mandatory)][string]$DoctorText)

    if (-not $DoctorText.Trim()) {
        throw 'mistralrs doctor returned no diagnostic output.'
    }

    $compiledForCuda = $DoctorText -match '(?im)^\s*(?:\[INFO\]\s*)?(?:build|compiled)\s+features\s*:[^\r\n]*\bcuda\b'
    $cudaHardwareDetected = $DoctorText -match '(?im)^\s*(?:\[INFO\]\s*)?CUDA\s*:[^\r\n]+'

    if (-not $compiledForCuda) {
        throw 'mistralrs doctor does not report the CUDA build feature.'
    }
    if (-not $cudaHardwareDetected) {
        throw 'mistralrs doctor does not report a detected CUDA toolkit and NVIDIA driver.'
    }
}

function Assert-NvidiaGpuAvailable {
    $nvidiaSmi = Get-CommandPath 'nvidia-smi'
    if (-not $nvidiaSmi) {
        throw 'nvidia-smi is required for the CUDA-only local executor.'
    }

    $gpuRows = & $nvidiaSmi --query-gpu=index,name,driver_version,memory.total --format=csv,noheader,nounits 2>&1
    if ($LASTEXITCODE -ne 0 -or -not ($gpuRows | Out-String).Trim()) {
        throw "nvidia-smi could not enumerate an NVIDIA GPU:`n$($gpuRows | Out-String)"
    }
    Write-Step "Detected NVIDIA GPU:`n$($gpuRows | Out-String)"
}

function Test-CudaMarker {
    param([Parameter(Mandatory)][string]$MistralPath)

    if (-not (Test-Path -LiteralPath $CudaMarkerPath)) { return $false }

    try {
        $marker = Get-Content -LiteralPath $CudaMarkerPath -Raw | ConvertFrom-Json
        $resolvedBinary = (Resolve-Path -LiteralPath $MistralPath).Path
        $binaryHash = (Get-FileHash -LiteralPath $resolvedBinary -Algorithm SHA256).Hash
        return (
            $marker.binary_path -eq $resolvedBinary -and
            $marker.binary_sha256 -eq $binaryHash -and
            [string]$marker.features -match '(^|\s)cuda($|\s)'
        )
    }
    catch {
        Write-Warning "Ignoring an invalid Mistral.rs CUDA marker: $($_.Exception.Message)"
        return $false
    }
}

function Test-CudaMistralRs {
    param([Parameter(Mandatory)][string]$MistralPath)

    try {
        Assert-NvidiaGpuAvailable
        $doctor = Get-MistralDoctorText -MistralPath $MistralPath
        Write-Host $doctor.TrimEnd()
        Assert-CudaDoctorEvidence -DoctorText $doctor

        if (Test-CudaMarker -MistralPath $MistralPath) {
            Write-Step "Verified the CUDA source-build marker for $MistralPath."
        }
        else {
            Write-Step 'CUDA was confirmed by mistralrs doctor; no matching local source-build marker was found.'
        }
        return $true
    }
    catch {
        Write-Warning $_.Exception.Message
        return $false
    }
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

$mistral = Get-CommandPath 'mistralrs'
if ($mistral -and -not $ForceSourceBuild) {
    Write-Step "Existing binary found at $mistral."
    if (Test-CudaMistralRs -MistralPath $mistral) {
        Write-Step 'Existing Mistral.rs installation satisfies the mandatory CUDA contract.'
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
    $instructions = @(
        'A native Windows CUDA build cannot start because these prerequisites are missing:',
        ($missing | ForEach-Object { "  - $_" }),
        '',
        'Install Rust 1.94+, Visual Studio 2022 C++ Build Tools, the NVIDIA driver, and the CUDA toolkit.',
        'The local 3DMk agent will not run without CUDA.'
    ) -join [Environment]::NewLine
    throw $instructions
}

Assert-NvidiaGpuAvailable

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
        throw "Mistral.rs failed to compile with both mandatory CUDA feature sets: $($_.Exception.Message)"
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
$doctor = Get-MistralDoctorText -MistralPath $mistral
Write-Host $doctor.TrimEnd()
Assert-CudaDoctorEvidence -DoctorText $doctor
if (-not (Test-CudaMarker -MistralPath $mistral)) {
    throw 'The CUDA installation marker could not be verified against the installed Mistral.rs binary.'
}

Write-Step "CUDA-enabled Mistral.rs is ready from source commit $sourceCommit with features '$featureSet'."