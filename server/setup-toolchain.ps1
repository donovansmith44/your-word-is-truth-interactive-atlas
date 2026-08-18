# Provisions a self-contained `llvm-dlltool.exe` at
# $env:USERPROFILE\.cargo\atlas-tools\llvm-dlltool.exe, used by ../.cargo/config.toml
# via `-C dlltool=...` to build raw-dylib import libs (windows-sys >= 0.60) on the
# x86_64-pc-windows-gnu target.
#
# Why: this target's bundled GNU `dlltool` shells out to `as` (GNU assembler) to
# synthesize import libraries, and this machine has no `as`/C compiler/binutils by
# design (see server/.cargo/config.toml and .superpowers/sdd/.../task-1-report.md
# for the full diagnosis). LLVM's dlltool needs no `as`. It isn't shipped under that
# name, but the `llvm-tools` rustup component's `llvm-ar.exe` is a multicall binary
# that dispatches based on its own file name — a copy renamed `llvm-dlltool.exe`
# runs as dlltool.
#
# Idempotent: safe to re-run. Run this once per machine (or whenever
# $env:USERPROFILE\.cargo\atlas-tools\llvm-dlltool.exe is missing, e.g. after a
# fresh clone or a new user account) before `cargo build`/`cargo test` in server/.

$ErrorActionPreference = 'Stop'

# rustup/cargo/rustc live in ~/.cargo/bin, which is not always on PATH outside an
# interactive profile-loaded shell (e.g. when this script is run with -NoProfile).
$env:Path = "$env:Path;$env:USERPROFILE\.cargo\bin"

$toolsDir = Join-Path $env:USERPROFILE '.cargo\atlas-tools'
$dlltool = Join-Path $toolsDir 'llvm-dlltool.exe'

if (Test-Path $dlltool) {
  Write-Output 'have llvm-dlltool'
  exit 0
}

Write-Output "provisioning $dlltool ..."

# 1. Ensure the llvm-tools rustup component is installed (ships llvm-ar.exe).
Write-Output 'rustup component add llvm-tools'
& rustup component add llvm-tools
if ($LASTEXITCODE -ne 0) {
  Write-Output "'llvm-tools' failed (exit $LASTEXITCODE); trying 'llvm-tools-preview' name instead..."
  & rustup component add llvm-tools-preview
  if ($LASTEXITCODE -ne 0) {
    throw "rustup component add failed for both 'llvm-tools' and 'llvm-tools-preview' (exit $LASTEXITCODE). Cannot provision llvm-dlltool. Check 'rustup show' and network access."
  }
}

# 2. Locate llvm-ar.exe under this toolchain's sysroot.
$sysroot = (& rustc --print sysroot | Out-String).Trim()
if (-not $sysroot) {
  throw "'rustc --print sysroot' returned nothing. Is rustc installed and on PATH?"
}
$binDir = Join-Path $sysroot 'lib\rustlib\x86_64-pc-windows-gnu\bin'
$arPath = Join-Path $binDir 'llvm-ar.exe'
if (-not (Test-Path $arPath)) {
  throw "Expected llvm-ar.exe at '$arPath' after installing the llvm-tools component, but it's not there. The component layout may have changed for this rustc version -- inspect '$binDir' manually for an ar/dlltool-capable binary."
}

# 3. Copy it into atlas-tools, renamed llvm-dlltool.exe. LLVM ships ar/dlltool/nm/etc
#    as one multicall binary that dispatches on its own program name, so the renamed
#    copy runs as dlltool without needing a separate download.
New-Item -ItemType Directory -Force -Path $toolsDir | Out-Null
Copy-Item -Path $arPath -Destination $dlltool -Force

# 4. Verify the renamed copy actually behaves as dlltool (prints dlltool usage, not
#    ar usage) before declaring success -- don't trust the rename blindly.
$helpText = (& $dlltool --help 2>&1 | Out-String)
if ($helpText -notmatch 'llvm-dlltool') {
  Remove-Item -Force $dlltool -ErrorAction SilentlyContinue
  throw "Copied '$arPath' to '$dlltool' but it does not behave as dlltool (--help output did not mention 'llvm-dlltool'). Removed the bad copy. Output was:`n$helpText"
}

Write-Output "provisioned $dlltool"
