<#
.SYNOPSIS
    Bootstraps the Agentic CAD Backend environment.
.DESCRIPTION
    Validates pre-requisites, writes payload files via Python, initializes Git, and compiles the Rust binary.
#>

$ErrorActionPreference = "Stop"

Write-Host "Initiating Agentic CAD Backend Bootstrap Sequence..." -ForegroundColor Cyan

# Pre-flight Tool Verification
$requiredTools = @("cargo", "python", "git")
foreach ($tool in $requiredTools) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        Write-Host "[-] Missing Dependency: $tool" -ForegroundColor Red
        Write-Host "Please install $tool before running this bootstrap sequence."
        Pause
        exit 1
    }
}
Write-Host "[+] All pre-flight tool checks passed." -ForegroundColor Green

# Materialize Payload
Write-Progress -Activity "Bootstrapping Project" -Status "Materializing repository files via build_project.py..."
python build_project.py

# Mandatory Git Initialization
Write-Progress -Activity "Bootstrapping Project" -Status "Initializing Git Repository..."
if (-not (Test-Path ".git")) {
    git init
    git add .
    git commit -m "Initial commit: Omnissiah's blessing (Agentic CAD Backend)"
    Write-Host "[+] Git repository initialized and baseline committed." -ForegroundColor Green
} else {
    Write-Host "[*] Git repository already exists. Skipping init." -ForegroundColor Yellow
}

# Compile Rust Backend
Write-Progress -Activity "Bootstrapping Project" -Status "Compiling Local Rust Backend (Release Mode)..."
Write-Host "Compiling Rust binary... (this may take a moment)" -ForegroundColor Cyan
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n[+++] Bootstrap Complete. Backend Ready." -ForegroundColor Green
    Write-Host "Executables located in target/release/agentic-cad-backend.exe"
} else {
    Write-Host "`n[!!!] Compilation Failed. Review cargo output above." -ForegroundColor Red
}

Pause