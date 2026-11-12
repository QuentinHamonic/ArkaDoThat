@echo off
setlocal

echo [1/2] Verification frontend...
call npm.cmd run check
if errorlevel 1 (
  echo Echec de la verification frontend.
  exit /b 1
)

echo [2/2] Generation Tauri...
call npm.cmd run build:desktop
if errorlevel 1 (
  echo Echec de la generation desktop.
  exit /b 1
)

echo.
echo Generation terminee.
echo Executable brut : target\release\taskhud.exe
echo Installateurs   : target\release\bundle\
exit /b 0
