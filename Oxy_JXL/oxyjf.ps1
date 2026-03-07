$SRC = "C:\Users\peter\Downloads\Compressed"
$CJXL = "C:\Users\peter\kDrive\Soft_WIN\jxl\bin\cjxl.exe"

$files = Get-ChildItem -Path $SRC -File -Recurse | Where-Object { 
    $_.Extension -match '\.(jpg|jpeg|png)$' -and $_.DirectoryName -notmatch ' jxl$' 
}

foreach ($file in $files) {
    $newDirName = $file.DirectoryName + " jxl"
    if (-not (Test-Path $newDirName)) { New-Item -ItemType Directory -Path $newDirName | Out-Null }

    $out = Join-Path $newDirName ($file.BaseName + ".jxl")
    if (Test-Path $out) { continue }

    Write-Host "Traitement : $($file.Name)... " -NoNewline

    # Tentative 1 : Transcodage direct (le plus léger)
    & $CJXL $file.FullName $out --lossless_jpeg=1 -e 1 2>$null

    if ($LASTEXITCODE -ne 0) {
        # Tentative 2 : Force le mode Pixels (Règle tes erreurs "Tail Data" et "Decoding")
        # --allow_jpeg_reconstruction 0 est la clé ici pour stopper le mode transcodage
        & $CJXL $file.FullName $out -d 0 -e 1 --allow_jpeg_reconstruction 0 2>$null
    }

    if ($LASTEXITCODE -eq 0) { Write-Host "OK" -ForegroundColor Green } else { Write-Host "FAIL" -ForegroundColor Red }
}

Write-Host "`nTerminé."
Pause