@echo off
title Radar kite
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0radar-kite.ps1"
if errorlevel 1 (
  echo.
  echo Qualcosa e' andato storto: copia il testo qui sopra e mandalo a Claude.
  pause
) else (
  timeout /t 8
)
