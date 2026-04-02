param(
    [Parameter(Mandatory=$true)]
    [string]$Path
)

# 1. Remplacement du texte à l'intérieur des fichiers
Write-Host "--- Remplacement du contenu des fichiers ---" -ForegroundColor Cyan
$files = Get-ChildItem -Path $Path -Recurse -File
foreach ($file in $files) {
    $content = Get-Content -Path $file.FullName -Raw
    if ($content -cmatch "Oxytools") {
        $content = $content -creplace "Oxytools", "Oxytools"
        Set-Content -Path $file.FullName -Value $content
        Write-Host "Modifié (contenu) : $($file.FullName)" -ForegroundColor Green
    }
}

# 2. Renommage des fichiers et dossiers (du plus profond au plus superficiel)
Write-Host "`n--- Renommage des fichiers et dossiers ---" -ForegroundColor Cyan
$items = Get-ChildItem -Path $Path -Recurse | 
         Sort-Object FullName -Descending # On trie pour renommer les enfants avant les parents

foreach ($item in $items) {
    if ($item.Name -cmatch "Oxytools") {
        $newName = $item.Name -creplace "Oxytools", "Oxytools"
        Rename-Item -Path $item.FullName -NewName $newName
        Write-Host "Renommé : $($item.Name) -> $newName" -ForegroundColor Yellow
    }
}

Write-Host "`nTerminé !" -ForegroundColor Cyan
