@echo off
setlocal

set "ROOT=%~dp0"
cd /d "%ROOT%"

if "%~1"=="" (
    set "EXE=%ROOT%target\release\agentic-cad-backend.exe"
    if not exist "%EXE%" (
        echo [!] Binary not found: %EXE%
        echo     Run cargo build --release first.
        exit /b 1
    )
    echo [*] Starting API server...
    start "3DMk Server" "%EXE%" serve --port 8181
    timeout /t 2 /nobreak >nul
    start "" "http://localhost:8181"
    goto :eof
)

set "EXE=%ROOT%target\release\agentic-cad-backend.exe"

if not exist "%EXE%" (
    echo [!] Binary not found: %EXE%
    echo     Run build_project.py or cargo build --release first.
    exit /b 1
)

"%EXE%" %*
