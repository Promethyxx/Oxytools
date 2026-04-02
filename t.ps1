param(
    [Parameter(Mandatory=$true)]
    [string]$Path
)

# 1. Remplacement du texte à l'intérieur des fichiers (minuscules uniquement)
Write-Host "--- Remplacement du contenu des fichiers ---" -ForegroundColor Cyan
$files = Get-ChildItem -Path $Path -Recurse -File
foreach ($file in $files) {
    $content = Get-Content -Path $file.FullName -Raw
    if ($content -cmatch "oxytools") {
        $content = $content -creplace "oxytools", "oxytools"
        Set-Content -Path $file.FullName -Value $content
        Write-Host "Modifié (contenu) : $($file.FullName)" -ForegroundColor Green
    }
}

# 2. Renommage des fichiers et dossiers (minuscules uniquement)
Write-Host "`n--- Renommage des fichiers et dossiers ---" -ForegroundColor Cyan
$items = Get-ChildItem -Path $Path -Recurse | 
         Sort-Object FullName -Descending # On trie du plus profond au plus superficiel

foreach ($item in $items) {
    if ($item.Name -cmatch "oxytools") {
        $newName = $item.Name -creplace "oxytools", "oxytools"
        Rename-Item -Path $item.FullName -NewName $newName
        Write-Host "Renommé : $($item.Name) -> $newName" -ForegroundColor Yellow
    }
}

Write-Host "`nTerminé !" -ForegroundColor Cyan
