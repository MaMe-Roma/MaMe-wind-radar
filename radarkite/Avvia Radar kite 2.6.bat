@echo off
rem Apre subito l'ultimo report (si aggiorna da solo quando arrivano i dati nuovi)
rem e avvia il programma senza finestra. Quello che scrive va in dati\ultimo-avvio.txt.
set "GIA="
if exist "%~dp0report.html" (
  start "" "%~dp0avvio.html"
  set "GIA=-GiaAperta"
)
start "" "%SystemRoot%\System32\conhost.exe" --headless "%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "%~dp0radar-kite.ps1" -Nascosto %GIA%
