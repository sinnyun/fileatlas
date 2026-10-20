@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0"
title FileAtlas development launcher

rem Usage: start-dev.bat [plugin-list]
rem Double-click loads every plugin (profile "all", including dev-test so the
rem database, settings, session and thumbnails stay isolated under target/dev-test).
rem plugin-list is comma-separated, e.g. start-dev.bat core
if not "%~1"=="" set "FILEATLAS_DEV_PLUGINS=%~1"
if not defined FILEATLAS_DEV_PLUGINS set "FILEATLAS_DEV_PLUGINS=all"
echo Usage: start-dev.bat [plugin-list]
echo Plugin profile: %FILEATLAS_DEV_PLUGINS%

where cargo >nul 2>nul
if errorlevel 1 (
    echo Cargo was not found. Install Rust and add Cargo to PATH.
    goto :failed
)

set "USE_RUSTUP_197=0"
where rustup >nul 2>nul
if not errorlevel 1 (
    rustup run 1.97.1 rustc --version >nul 2>nul
    if not errorlevel 1 set "USE_RUSTUP_197=1"
)

echo Checking fileatlas-shell compilation...
if "%USE_RUSTUP_197%"=="1" (
    rustup run 1.97.1 cargo check -p fileatlas-shell --locked -j 2
) else (
    cargo check -p fileatlas-shell --locked -j 2
)
if errorlevel 1 (
    echo Compilation check failed with exit code !ERRORLEVEL!. The desktop app was not started.
    goto :failed
)

echo Starting the FileAtlas desktop development UI...
if "%USE_RUSTUP_197%"=="1" (
    rustup run 1.97.1 cargo run -p fileatlas-shell --locked -j 2
) else (
    cargo run -p fileatlas-shell --locked -j 2
)
if errorlevel 1 (
    echo Application run failed with exit code !ERRORLEVEL!.
    goto :failed
)
exit /b 0

:failed
echo.
pause
exit /b 1
