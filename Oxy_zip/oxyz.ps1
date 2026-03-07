# Oxy_Backup.ps1 — Sauvegarde ZIP multiplateforme (PowerShell 5+ / pwsh 7+)
#
# Windows silencieux : utiliser Oxy_Backup_Run.cmd
# Linux/macOS        : pwsh Oxy_Backup.ps1 &

param(
    [string]$Source  = 'C:\Users\peter\Documents\Test',
    [string]$DestDir = 'E:\Backup'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ── Date du jour ──
$today   = Get-Date -Format 'yyyyMMdd'
$destZip = Join-Path $DestDir "Backup_${today}.zip"

# ── Vérifications ──
if (-not (Test-Path -LiteralPath $Source)) {
    Write-Error "Source introuvable : $Source"
    exit 1
}
if (-not (Test-Path -LiteralPath $DestDir)) {
    New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
}

# ── Détection 7-Zip (compatible PS5 et pwsh 7) ──
$sevenZip = $null

# PS5 n'a pas $IsWindows, on teste l'OS autrement
$onWindows = ($env:OS -eq 'Windows_NT') -or ($PSVersionTable.Platform -eq 'Win32NT') -or
             (($null -ne $IsWindows) -and $IsWindows)

if ($onWindows) {
    $candidate = 'C:\Program Files\7-Zip\7z.exe'
    if (Test-Path -LiteralPath $candidate) { $sevenZip = $candidate }
}
if (-not $sevenZip) {
    $found = Get-Command 7z -ErrorAction SilentlyContinue
    if ($found) { $sevenZip = $found.Source }
}

# ── Compression ──
if ($sevenZip) {
    # Via 7-Zip
    $argList = @(
        'a', '-tzip', $destZip,
        (Join-Path $Source '*'),
        '-xr!.git', '-xr!.github', '-xr!target', '-y'
    )
    & $sevenZip @argList
    if ($LASTEXITCODE -ne 0) {
        Write-Error "7-Zip a echoue (code $LASTEXITCODE)"
        exit 1
    }
}
else {
    # Via .NET natif (System.IO.Compression)
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem

    if (Test-Path -LiteralPath $destZip) { Remove-Item -LiteralPath $destZip -Force }

    # Normalise le chemin source (résolu, sans trailing separator)
    $srcRoot = (Resolve-Path -LiteralPath $Source).Path.TrimEnd(
        [IO.Path]::DirectorySeparatorChar,
        [IO.Path]::AltDirectorySeparatorChar
    )

    # Collecte des fichiers en excluant .git, .github, target
    $files = Get-ChildItem -LiteralPath $srcRoot -Recurse -File | Where-Object {
        $rel = $_.FullName.Substring($srcRoot.Length + 1)
        ($rel -notmatch '(^|[\\/])\.git(hub)?([\\/]|$)') -and
        ($rel -notmatch '(^|[\\/])target([\\/]|$)')
    }

    $zip = [System.IO.Compression.ZipFile]::Open(
        $destZip,
        [System.IO.Compression.ZipArchiveMode]::Create
    )
    try {
        foreach ($f in $files) {
            $entryName = $f.FullName.Substring($srcRoot.Length + 1) -replace '\\', '/'
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                $zip, $f.FullName, $entryName,
                [System.IO.Compression.CompressionLevel]::Optimal
            ) | Out-Null
        }
    }
    finally {
        $zip.Dispose()
    }
}

Write-Output "Sauvegarde terminee : $destZip"
exit 0