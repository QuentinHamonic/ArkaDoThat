@echo off
setlocal

echo [1/3] Frontend check...
call npm.cmd run check
if errorlevel 1 (
  echo Frontend check failed.
  exit /b 1
)

echo [2/3] Tauri build...
call npm.cmd run build:desktop
if errorlevel 1 (
  echo Desktop build failed.
  exit /b 1
)

echo [3/3] Copying portable executable to the project root...
copy /Y "target\release\taskhud.exe" "TaskHUD.exe" >nul
if errorlevel 1 (
  echo Could not copy TaskHUD.exe to the project root.
  exit /b 1
)

echo.
echo Build finished.
echo Portable executable : TaskHUD.exe
echo Installers     : target\release\bundle\
exit /b 0
