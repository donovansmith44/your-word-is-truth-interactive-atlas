# Pester tests for archive-raw.ps1. Pester (bundled with Windows PowerShell 5.1, found via
# `Get-Module -ListAvailable Pester`) is used rather than a hand-rolled harness -- it is already
# on this host, so no new test framework is introduced for two PowerShell files.
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'archive-raw.ps1')

$script:BibexPath = Resolve-BibexPath -RepoRoot $RepoRoot
if (-not (Test-Path -LiteralPath $script:BibexPath)) {
    throw "bibex not found at $script:BibexPath -- build it first (cargo build --release -p atlas-cli, from server/) before running these tests"
}

# A minimal fixture: the six archived entries only, populated with distinct small contents and
# blessed for real by bibex, so the fixture's MANIFEST.toml is exactly what the tool itself
# would produce -- never hand-written TOML. The compiled side is copied once from the real
# data/compiled (read-only, never mutated) so `bibex verify` has a real, passing compiled
# manifest to check beside the fixture's raw tree.
$script:SharedCompiledDir = Join-Path $env:TEMP "archive-raw-tests-compiled-$PID"

function New-RawFixture {
    param([string]$Name)
    $root = Join-Path $env:TEMP "archive-raw-fixture-$Name-$PID"
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    $compiled = Join-Path $root 'compiled'
    $raw = Join-Path $root 'raw'
    New-Item -ItemType Directory -Force -Path $raw, (Join-Path $raw 'geo'), (Join-Path $raw 'concord'), (Join-Path $raw 'kretzmann') | Out-Null
    Copy-Item -LiteralPath $script:SharedCompiledDir -Destination $compiled -Recurse
    Set-Content -LiteralPath (Join-Path $raw 'geo\ancient.jsonl') -Value 'ancient places' -NoNewline
    Set-Content -LiteralPath (Join-Path $raw 'geo\modern.jsonl') -Value 'modern places' -NoNewline
    Set-Content -LiteralPath (Join-Path $raw 'concord\augsburg-confession.html') -Value 'augsburg text' -NoNewline
    Set-Content -LiteralPath (Join-Path $raw 'kretzmann\genesis-1.html') -Value 'kretzmann genesis 1' -NoNewline
    Set-Content -LiteralPath (Join-Path $raw 'theographic.zip') -Value 'theographic bytes' -NoNewline
    Set-Content -LiteralPath (Join-Path $raw 'cross-references.zip') -Value 'cross reference bytes' -NoNewline
    Set-Content -LiteralPath (Join-Path $raw 'catechism-mapping.zip') -Value 'catechism bytes' -NoNewline
    $bless = Invoke-ProcessCapture -FilePath $script:BibexPath -ArgumentList @('--json', 'raw', 'bless', '--data-dir', $compiled)
    if ($bless.ExitCode -ne 0) { throw "fixture bless failed: $($bless.StdErr)$($bless.StdOut)" }
    [pscustomobject]@{ Compiled = $compiled; Raw = $raw; RawRoot = ($bless.StdOut | ConvertFrom-Json).root }
}

function New-DestinationDir {
    param([string]$Name)
    $destination = Join-Path $env:TEMP "archive-raw-dest-$Name-$PID"
    Remove-Item -LiteralPath $destination -Recurse -Force -ErrorAction SilentlyContinue
    $destination
}

Describe 'archive-raw' {
    BeforeAll {
        Remove-Item -LiteralPath $script:SharedCompiledDir -Recurse -Force -ErrorAction SilentlyContinue
        Copy-Item -LiteralPath (Join-Path $RepoRoot 'data\compiled') -Destination $script:SharedCompiledDir -Recurse
    }

    AfterAll {
        Remove-Item -LiteralPath $script:SharedCompiledDir -Recurse -Force -ErrorAction SilentlyContinue
        Get-ChildItem -Path $env:TEMP -Directory -Filter 'archive-raw-*' -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match "-$PID$" } |
            Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
    }

    It 'refuses to archive a tree bibex verify rejects, naming the mismatched file' {
        # Arrange
        $fixture = New-RawFixture -Name 'mismatch'
        Set-Content -LiteralPath (Join-Path $fixture.Raw 'geo\ancient.jsonl') -Value 'ANCIENT PLACES' -NoNewline
        $destination = New-DestinationDir -Name 'mismatch'

        # Act
        $action = { Invoke-ArchiveRaw -RawDir $fixture.Raw -Destination $destination -BibexPath $script:BibexPath -SevenZipExe '7z' -ArchivedEntries $ArchivedEntries }

        # Assert
        $action | Should Throw 'data/raw does not verify -- refusing to archive it -- integrity_failed: '
        $action | Should Throw 'raw geo/ancient.jsonl: MISMATCH'
        Test-Path -LiteralPath $destination | Should Be $false
    }

    It 'refuses to archive a tree that has no manifest, naming the manifest it looked for' {
        # Arrange
        $fixture = New-RawFixture -Name 'unrecorded'
        $manifestPath = Join-Path $fixture.Raw 'MANIFEST.toml'
        Remove-Item -LiteralPath $manifestPath
        $destination = New-DestinationDir -Name 'unrecorded'

        # Act
        $action = { Invoke-ArchiveRaw -RawDir $fixture.Raw -Destination $destination -BibexPath $script:BibexPath -SevenZipExe '7z' -ArchivedEntries $ArchivedEntries }

        # Assert
        $action | Should Throw "data/raw is not recorded -- refusing to archive it -- no MANIFEST.toml at $manifestPath"
        Test-Path -LiteralPath $destination | Should Be $false
    }

    It 'refuses to overwrite an archive whose root already exists at the destination' {
        # Arrange
        $fixture = New-RawFixture -Name 'existing-root'
        $destination = New-DestinationDir -Name 'existing-root'
        New-Item -ItemType Directory -Force -Path $destination | Out-Null
        $existingPath = Join-Path $destination "bible-atlas-data-raw-2020-01-01-raw-$($fixture.RawRoot).7z"
        Set-Content -LiteralPath $existingPath -Value 'placeholder' -NoNewline

        # Act
        $action = { Invoke-ArchiveRaw -RawDir $fixture.Raw -Destination $destination -BibexPath $script:BibexPath -SevenZipExe '7z' -ArchivedEntries $ArchivedEntries }

        # Assert
        $action | Should Throw "an archive for root $($fixture.RawRoot) already exists at $existingPath -- refusing to overwrite it"
        (Get-Content -LiteralPath $existingPath -Raw) | Should Be 'placeholder'
        (Get-ChildItem -Path $destination -File).Count | Should Be 1
    }

    It 'archives the six datasets, verifies the round trip, and prints the summary' {
        # Arrange
        $fixture = New-RawFixture -Name 'happy'
        $destination = New-DestinationDir -Name 'happy'

        # Act
        $output = & $PSScriptRoot\archive-raw.ps1 -RawDir $fixture.Raw -Destination $destination -BibexPath $script:BibexPath

        # Assert
        $expectedArchive = Join-Path $destination "bible-atlas-data-raw-$(Get-Date -Format 'yyyy-MM-dd')-raw-$($fixture.RawRoot).7z"
        Test-Path -LiteralPath $expectedArchive | Should Be $true
        $size = (Get-Item -LiteralPath $expectedArchive).Length
        $sizeMiB = [math]::Round($size / 1MB, 1)
        $expected = @(
            "archive $expectedArchive"
            "size $sizeMiB MiB ($size bytes)"
            "root $($fixture.RawRoot)"
            "restore: 7z x `"$expectedArchive`" -o`"$($fixture.Raw)`" -y"
        ) -join "`n"
        ($output -join "`n") | Should Be $expected
    }

    It 'falls back to Compress-Archive and warns when 7z is unavailable' {
        # Arrange
        $fixture = New-RawFixture -Name 'fallback'
        $destination = New-DestinationDir -Name 'fallback'

        # Act
        $result = Invoke-ArchiveRaw -RawDir $fixture.Raw -Destination $destination -BibexPath $script:BibexPath `
            -SevenZipExe 'archive-raw-no-such-7z-tool' -ArchivedEntries $ArchivedEntries `
            -WarningVariable warnings -WarningAction SilentlyContinue

        # Assert
        $result.Path | Should Match '\.zip$'
        Test-Path -LiteralPath $result.Path | Should Be $true
        ($warnings -join ' ') | Should Match '7z not found on PATH -- falling back to Compress-Archive'
    }
}
