# Oxy_FL.ps1 — Liste les fichiers par emplacement (PowerShell 5+ / pwsh 7+)

param(
    [string]$ListDir = 'C:\Users\peter\kDrive\Documents\Listes'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $ListDir)) {
    New-Item -ItemType Directory -Path $ListDir -Force | Out-Null
}

$fileSources = @{
    'Animes'       = 'E:\Animes'
    'Docs'         = 'H:\Docs'
    'Docs Nat'     = 'H:\Docs Nat'
    'Films'        = 'F:\Films'
    'Films Animes' = 'E:\Films Animes'
    'Series'       = 'G:\Series'
    'Shows'        = 'H:\Shows'
    'Reste'        = 'H:\Reste'
}

foreach ($entry in $fileSources.GetEnumerator()) {
    $name = $entry.Key
    $dir  = $entry.Value
    $out  = Join-Path $ListDir "$name.txt"

    Write-Output "Traitement : $dir"

    if (Test-Path -LiteralPath $dir) {
        $files = Get-ChildItem -LiteralPath $dir -File -Recurse |
                 Select-Object -ExpandProperty FullName

        if ($files) {
            $files | Out-File -FilePath $out -Encoding UTF8
        } elseif (Test-Path -LiteralPath $out) {
            Remove-Item -LiteralPath $out -Force
        }
    } else {
        Write-Output "  -> Inaccessible"
    }
}

Write-Output 'Termine.'
exit 0