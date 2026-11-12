@echo off
setlocal

echo [1/2] Frontend check...
call npm.cmd run check
if errorlevel 1 (
  echo Frontend check failed.
  exit /b 1
)

echo [2/2] Tauri build...
call npm.cmd run build:desktop
if errorlevel 1 (
  echo Desktop build failed.
  exit /b 1
)

echo.
echo Build finished.
echo Raw executable : target\release\taskhud.exe
echo Installers     : target\release\bundle\
exit /b 0
