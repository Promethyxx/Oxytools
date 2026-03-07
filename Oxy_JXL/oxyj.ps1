$SRC = "C:\Users\peter\Downloads\Compressed"
$CJXL = "C:\Users\peter\kDrive\Soft_WIN\jxl\bin\cjxl.exe"

# On récupère uniquement les sources, on ignore les .jxl
$files = Get-ChildItem -Path $SRC -File -Recurse | Where-Object { 
    $_.Extension -match '\.(jpg|jpeg|png|webp)$' 
}

foreach ($file in $files) {
    # Sortie dans le même répertoire que le fichier source
    $out = Join-Path $file.DirectoryName ($file.BaseName + ".jxl")
    
    # Si le .jxl existe déjà, on passe au suivant
    if (Test-Path $out) { continue }

    Write-Host "Traitement : $($file.Name)... " -NoNewline

    # Mode 100% Lossless (Transcodage bit-identique pour JPEG)
    & $CJXL $file.FullName $out --lossless_jpeg=1 -d 0 -e 1 2>$null

    if ($LASTEXITCODE -eq 0) { 
        Write-Host "OK" -ForegroundColor Green 
    } else { 
        Write-Host "FAIL" -ForegroundColor Red 
    }
}

Write-Host "`nTerminé. Les fichiers sont côte à côte."
Pause