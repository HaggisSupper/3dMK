param(
    [string]$Model = $(if ($env:THREEDMK_VLM_MODEL) { $env:THREEDMK_VLM_MODEL } else { 'mistralai/Pixtral-12B-2409' }),
    [ValidateRange(1, 65535)][int]$Port = 8080,
    [ValidateSet(2, 3, 4, 5, 6, 8)][int]$Quant = 4,
    [ValidateRange(2048, 32768)][int]$ContextLength = 8192,
    [switch]$SkipCudaSetup
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Resolve-CommandPath {
    param([Parameter(Mandatory)][string]$Name)
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $command) { throw "$Name is not available on PATH." }
    return $command.Source
}

if (-not $SkipCudaSetup) {
    $setup = Join-Path $PSScriptRoot 'setup-mistralrs-local-agent.ps1'
    & (Resolve-CommandPath 'pwsh') -NoLogo -NoProfile -File $setup
    if ($LASTEXITCODE -ne 0) { throw "CUDA Mistral.rs setup failed with exit code $LASTEXITCODE." }
}

$mistral = Resolve-CommandPath 'mistralrs'
$doctor = (& $mistral doctor 2>&1 | Out-String)
if ($doctor -notmatch '(?im)cuda') {
    throw 'Mistral.rs doctor did not report CUDA support; refusing to start a CPU VLM.'
}

$env:THREEDMK_VLM_ENDPOINT = "http://127.0.0.1:$Port"
$env:THREEDMK_VLM_MODEL = $Model
Write-Host "Starting CUDA-backed local VLM '$Model' on $($env:THREEDMK_VLM_ENDPOINT)."
Write-Host 'Keep this process running while 3DMk uses VWM perception.'

& $mistral serve `
    --host 127.0.0.1 `
    --port $Port `
    --no-ui `
    --quant $Quant `
    --max-seq-len $ContextLength `
    --paged-attn auto `
    multimodal `
    --model-id $Model

if ($LASTEXITCODE -ne 0) { throw "Mistral.rs VLM exited with code $LASTEXITCODE." }
