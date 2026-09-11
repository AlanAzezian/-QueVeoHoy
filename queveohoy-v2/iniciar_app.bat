@echo off
cd /d "C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2"
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
npm run tauri dev
