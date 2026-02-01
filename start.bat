@echo off
title mITyFactory
echo.
echo  ╔══════════════════════════════════════╗
echo  ║         mITyFactory Launcher         ║
echo  ╚══════════════════════════════════════╝
echo.

cd /d "%~dp0"

:: Check if cargo is available
where cargo >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Cargo not found. Please install Rust from https://rustup.rs
    pause
    exit /b 1
)

:: Kill any running instance first
taskkill /f /im mity_ui.exe >nul 2>&1

:: Always rebuild to get latest changes
echo [INFO] Building mITyFactory (this may take a few minutes)...
cargo build --release -p mity_ui
if %ERRORLEVEL% neq 0 (
    echo.
    echo [ERROR] Build failed
    pause
    exit /b 1
)

echo [INFO] Launching mITyFactory...
start "" "target\release\mity_ui.exe"

echo [INFO] mITyFactory started!
timeout /t 2 >nul
