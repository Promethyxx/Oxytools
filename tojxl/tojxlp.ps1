Add-Type -AssemblyName System.Drawing
$SRC = ""
$CJXL = ""

$files = Get-ChildItem -Path $SRC -File -Recurse | Where-Object { $_.Extension -match '\.(jpg|jpeg|png)$' }

foreach ($file in $files) {
    $out = Join-Path $file.DirectoryName ($file.BaseName + "_2026.jxl")
    if (Test-Path $out) { continue }

    Write-Host "Forçage système : $($file.Name)... " -NoNewline

    try {
        # On force le chargement via Windows
        $bmp = [System.Drawing.Bitmap]::FromFile($file.FullName)
        $tempPng = [System.IO.Path]::GetTempFileName() + ".png"
        
        # On sauvegarde en PNG (format pivot propre)
        $bmp.Save($tempPng, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()

        # On convertit le PNG en JXL
        & $CJXL $tempPng $out -d 0 -e 1 --allow_jpeg_reconstruction 0 2>$null
        Remove-Item $tempPng
        Write-Host "OK" -ForegroundColor Green
    } catch {
        Write-Host "ÉCHEC CRITIQUE" -ForegroundColor Red
    }
}