$SRC = "."
$7Z = "7z.exe"

$dirs = Get-ChildItem -Path $SRC -Directory

foreach ($dir in $dirs) {
    $out = Join-Path $SRC ($dir.Name + ".7z")
    Write-Host "Zipping: $($dir.Name)... " -NoNewline
    
    & $7Z a -t7z $out $dir.FullName

    if ($LASTEXITCODE -eq 0) { Write-Host "OK" -ForegroundColor Green } else { Write-Host "FAIL" -ForegroundColor Red }
}
Pause