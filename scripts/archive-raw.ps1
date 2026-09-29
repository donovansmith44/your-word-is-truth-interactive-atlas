# RAW-INTEGRITY Task 6: the copy of data/raw that survives a worktree removal
# (PRINCIPLES 20 -- 2026-09-28 lost 374 MB of data/raw when a junctioned worktree was force-
# removed). Archives the trees that have no pinned upstream source (geo, concord, kretzmann)
# plus the three raw zips still on disk, to a path outside every worktree, refusing to run over
# a tree that does not verify and refusing to silently overwrite an existing backup of the same
# root.
[CmdletBinding()]
param(
    [string]$RawDir = (Join-Path (Split-Path -Parent $PSScriptRoot) 'data\raw'),
    [string]$Destination = (Join-Path $env:USERPROFILE 'Documents\bible-atlas-backups'),
    [string]$BibexPath,
    [string]$SevenZipExe = '7z'
)

$ErrorActionPreference = 'Stop'

# The trees a fetch cannot reproduce byte-for-byte (no pin: a live GitHub repo listing, or a
# scraped website) plus the three zips fetch-raw.ps1 leaves on disk. Order is the archive's own
# member order, so a diff of two archives' listings is a diff of this list.
$ArchivedEntries = @(
    [pscustomobject]@{ Name = 'geo'; Kind = 'dir' }
    [pscustomobject]@{ Name = 'concord'; Kind = 'dir' }
    [pscustomobject]@{ Name = 'kretzmann'; Kind = 'dir' }
    [pscustomobject]@{ Name = 'theographic.zip'; Kind = 'file' }
    [pscustomobject]@{ Name = 'cross-references.zip'; Kind = 'file' }
    [pscustomobject]@{ Name = 'catechism-mapping.zip'; Kind = 'file' }
)

function Resolve-BibexPath {
    param([string]$RepoRoot)
    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $RepoRoot 'server\target' }
    $release = Join-Path $targetRoot 'release\bibex.exe'
    if (Test-Path -LiteralPath $release) { return $release }
    return (Join-Path $targetRoot 'debug\bibex.exe')
}

# Runs a native command with stdout, stderr and the exit code all captured cleanly -- PowerShell
# 5.1's `2>&1` on a native exe wraps every stderr line in a NativeCommandError instead of plain
# text, which would corrupt the "name the files" text bibex itself already produces.
function Invoke-ProcessCapture {
    param([string]$FilePath, [string[]]$ArgumentList)
    $outFile = [System.IO.Path]::GetTempFileName()
    $errFile = [System.IO.Path]::GetTempFileName()
    try {
        $p = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -NoNewWindow -PassThru -Wait `
            -RedirectStandardOutput $outFile -RedirectStandardError $errFile
        [pscustomobject]@{
            ExitCode = $p.ExitCode
            StdOut   = (Get-Content -LiteralPath $outFile -Raw -ErrorAction SilentlyContinue)
            StdErr   = (Get-Content -LiteralPath $errFile -Raw -ErrorAction SilentlyContinue)
        }
    } finally {
        Remove-Item -LiteralPath $outFile, $errFile -Force -ErrorAction SilentlyContinue
    }
}

function Test-SevenZipAvailable {
    param([string]$Exe)
    [bool](Get-Command $Exe -ErrorAction SilentlyContinue)
}

function New-ArchiveFileName {
    param([string]$Root, [string]$Extension)
    "bible-atlas-data-raw-{0}-raw-{1}{2}" -f (Get-Date -Format 'yyyy-MM-dd'), $Root, $Extension
}

function Get-ExistingArchiveForRoot {
    param([string]$Destination, [string]$Root)
    if (-not (Test-Path -LiteralPath $Destination)) { return $null }
    Get-ChildItem -LiteralPath $Destination -Filter "*-raw-$Root.*" -File | Select-Object -First 1
}

function Invoke-Archive {
    param([string[]]$Names, [string]$RawDir, [string]$ArchivePath, [bool]$SevenZipAvailable, [string]$SevenZipExe)
    Push-Location -LiteralPath $RawDir
    try {
        if ($SevenZipAvailable) {
            & $SevenZipExe a '-t7z' '-mx5' '-mmt=4' $ArchivePath @Names | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "7z failed (exit $LASTEXITCODE) archiving $($Names -join ', ') into $ArchivePath" }
        } else {
            $fullPaths = $Names | ForEach-Object { Join-Path $RawDir $_ }
            Compress-Archive -Path $fullPaths -DestinationPath $ArchivePath -Force
        }
    } finally {
        Pop-Location
    }
}

function Expand-RawArchive {
    param([string]$ArchivePath, [string]$DestinationDir, [bool]$SevenZipAvailable, [string]$SevenZipExe)
    if ($SevenZipAvailable) {
        & $SevenZipExe x $ArchivePath "-o$DestinationDir" '-y' | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "7z failed (exit $LASTEXITCODE) extracting $ArchivePath for round-trip verification" }
    } else {
        Expand-Archive -Path $ArchivePath -DestinationPath $DestinationDir -Force
    }
}

function Invoke-ArchiveRaw {
    [CmdletBinding()]
    param(
        [string]$RawDir,
        [string]$Destination,
        [string]$BibexPath,
        [string]$SevenZipExe,
        [pscustomobject[]]$ArchivedEntries
    )

    $compiledDir = Join-Path (Split-Path -Parent $RawDir) 'compiled'
    $manifestPath = Join-Path $RawDir 'MANIFEST.toml'

    if (-not (Test-Path -LiteralPath $BibexPath)) {
        throw "bibex not found at $BibexPath -- build it first (cargo build --release -p atlas-cli, from server/)"
    }
    $verify = Invoke-ProcessCapture -FilePath $BibexPath -ArgumentList @('--json', 'verify', '--data-dir', $compiledDir)
    if ($verify.ExitCode -ne 0) {
        $failure = ($verify.StdErr | ConvertFrom-Json).error
        throw "data/raw does not verify -- refusing to archive it -- $($failure.code): $($failure.message)"
    }
    $raw = ($verify.StdOut | ConvertFrom-Json).raw
    if ($raw.status -ne 'ok') {
        throw "data/raw is not recorded -- refusing to archive it -- $($raw.why)"
    }
    $root = $raw.root

    $existing = Get-ExistingArchiveForRoot -Destination $Destination -Root $root
    if ($existing) {
        throw "an archive for root $root already exists at $($existing.FullName) -- refusing to overwrite it"
    }

    if (-not (Test-Path -LiteralPath $Destination)) { New-Item -ItemType Directory -Force -Path $Destination | Out-Null }

    $sevenZipAvailable = Test-SevenZipAvailable -Exe $SevenZipExe
    if (-not $sevenZipAvailable) {
        Write-Warning '7z not found on PATH -- falling back to Compress-Archive (.zip, slower and larger than 7z -mx5)'
    }
    $extension = if ($sevenZipAvailable) { '.7z' } else { '.zip' }
    $archiveName = New-ArchiveFileName -Root $root -Extension $extension
    $archivePath = Join-Path $Destination $archiveName

    $names = $ArchivedEntries | ForEach-Object { $_.Name }
    Invoke-Archive -Names $names -RawDir $RawDir -ArchivePath $archivePath -SevenZipAvailable $sevenZipAvailable -SevenZipExe $SevenZipExe

    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "archive-raw-roundtrip-$([guid]::NewGuid())"
    $tempRawDir = Join-Path $tempRoot 'raw'
    $tempCompiledDir = Join-Path $tempRoot 'compiled'
    New-Item -ItemType Directory -Force -Path $tempRawDir | Out-Null
    try {
        Expand-RawArchive -ArchivePath $archivePath -DestinationDir $tempRawDir -SevenZipAvailable $sevenZipAvailable -SevenZipExe $SevenZipExe

        # An entry's hash never depends on its siblings, so the recorded manifest answers for each one alone.
        Copy-Item -LiteralPath $manifestPath -Destination $tempRawDir
        foreach ($entry in $ArchivedEntries) {
            $check = Invoke-ProcessCapture -FilePath $BibexPath -ArgumentList @('raw', 'check', $entry.Name, '--data-dir', $tempCompiledDir)
            if ($check.ExitCode -ne 0) {
                throw "round-trip verification failed for $($entry.Name) -- $($check.StdErr)$($check.StdOut)"
            }
        }
    } finally {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }

    $size = (Get-Item -LiteralPath $archivePath).Length
    $restoreCommand = if ($sevenZipAvailable) {
        "7z x `"$archivePath`" -o`"$RawDir`" -y"
    } else {
        "Expand-Archive -Path `"$archivePath`" -DestinationPath `"$RawDir`" -Force"
    }

    [pscustomobject]@{
        Path            = $archivePath
        SizeBytes       = $size
        Root            = $root
        RestoreCommand  = $restoreCommand
    }
}

if ($MyInvocation.InvocationName -ne '.') {
    if (-not $BibexPath) { $BibexPath = Resolve-BibexPath -RepoRoot (Split-Path -Parent $PSScriptRoot) }
    $result = Invoke-ArchiveRaw -RawDir $RawDir -Destination $Destination -BibexPath $BibexPath -SevenZipExe $SevenZipExe -ArchivedEntries $ArchivedEntries
    $sizeMiB = [math]::Round($result.SizeBytes / 1MB, 1)
    Write-Output "archive $($result.Path)"
    Write-Output "size $sizeMiB MiB ($($result.SizeBytes) bytes)"
    Write-Output "root $($result.Root)"
    Write-Output "restore: $($result.RestoreCommand)"
}
