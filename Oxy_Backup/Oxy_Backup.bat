@echo off
setlocal

:: Source
set "SOURCE=C:\Users\peter\Documents\Test"

:: Date du jour YYYYMMDD via PowerShell
for /f %%i in ('powershell -NoProfile -Command "Get-Date -Format yyyyMMdd"') do set TODAY=%%i

:: Destination
set "DEST=E:\Backup\Backup_%TODAY%.zip"

:: Chemin de 7-Zip
set "SEVENZIP=C:\Program Files\7-Zip\7z.exe"
if not exist "%SEVENZIP%" (
    echo 7-Zip introuvable
    pause
    exit /b 1
)

:: Compression en excluant .git et target
"%SEVENZIP%" a -tzip "%DEST%" "%SOURCE%\*" -xr!.git -xr!.github -xr!target -y

echo Sauvegarde terminee : %DEST%
exit /b 0
