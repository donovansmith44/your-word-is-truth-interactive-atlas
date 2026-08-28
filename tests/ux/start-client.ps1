$env:Path = "$env:Path;$env:LOCALAPPDATA\Microsoft\dotnet"
$env:DOTNET_ROOT = "$env:LOCALAPPDATA\Microsoft\dotnet"
Set-Location "$PSScriptRoot\..\.."
# Batch CHRON-1: appending to $env:Path (above) does not change resolution
# ORDER -- a machine-wide "C:\Program Files\dotnet\dotnet.exe" (SDK-less,
# --list-sdks returns empty) sits earlier in the inherited system PATH and
# still wins `Get-Command dotnet`/bare `dotnet` regardless, causing
# "No .NET SDKs were found" under Playwright's own webServer spawn (verified
# directly: a fresh PowerShell running this exact append still resolved to
# the Program Files copy). Calling the LOCALAPPDATA dotnet.exe by its own
# full path sidesteps PATH resolution order entirely -- the one genuinely
# reliable fix, not order-dependent on whatever this machine's system PATH
# happens to contain.
# T-4 (fix round 1 review, Trivia): fall back to plain `dotnet` (PATH
# resolution) if the LOCALAPPDATA copy this machine happens to need isn't
# there at all, rather than hard-failing on a path that only this one
# machine is known to require.
$dotnetExe = "$env:LOCALAPPDATA\Microsoft\dotnet\dotnet.exe"
if (Test-Path $dotnetExe) {
    & $dotnetExe run --project client --launch-profile http
} else {
    dotnet run --project client --launch-profile http
}
