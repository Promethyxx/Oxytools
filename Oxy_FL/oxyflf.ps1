# Oxy_FLF.ps1 — Liste les dossiers multimedia (PowerShell 5+ / pwsh 7+)

param(
    [string]$ListDir = 'C:\Users\peter\kDrive\Documents\Listes'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $ListDir)) {
    New-Item -ItemType Directory -Path $ListDir -Force | Out-Null
}

$outputFolders = Join-Path $ListDir 'multimedia.txt'
if (Test-Path -LiteralPath $outputFolders) { Remove-Item -LiteralPath $outputFolders -Force }

$folderSources = @(
    'E:\Animes',
    'G:\Series',
    'H:\Reste\Ebooks'
)

$folderLines = foreach ($dir in $folderSources) {
    if (Test-Path -LiteralPath $dir) {
        Get-ChildItem -LiteralPath $dir -Directory -Recurse |
            Select-Object -ExpandProperty FullName
    } else {
        Write-Output "# Inaccessible : $dir"
    }
}

if ($folderLines) {
    $folderLines | Out-File -FilePath $outputFolders -Encoding UTF8
}

Write-Output 'Termine.'
exit 0