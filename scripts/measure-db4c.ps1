# DB-4c, spec 12: the measurements the report must state, by the methods
# earlier batches used. Run from the repo root AFTER `cargo build -p
# atlas-server -p atlas-cli` (debug; never --release while 8080 runs).
#   (a) peak resident memory of one `bibex verse` run (PeakWorkingSet64,
#       sampled every 20 ms -- the OVERLAY-1 method), now over the sections;
#   (b) cold start (unpack cache emptied) and warm start of atlas-server on
#       port 8000, process start -> first /health 200.
# Port 8000 only; 8080 is never touched.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$server = Join-Path $root "server"
$bibex = Join-Path $server "target\debug\bibex.exe"
$atlas = Join-Path $server "target\debug\atlas-server.exe"
$data = Join-Path $root "data\compiled"
$cache = Join-Path $root "data\cache\sections"

function Peak-Rss($exe, $argList) {
    $p = Start-Process -FilePath $exe -ArgumentList $argList -WorkingDirectory $server -PassThru -WindowStyle Hidden -RedirectStandardOutput "$env:TEMP\db4c-rss-out.txt" -RedirectStandardError "$env:TEMP\db4c-rss-err.txt"
    $peak = 0
    while (-not $p.HasExited) {
        try { $p.Refresh(); if ($p.PeakWorkingSet64 -gt $peak) { $peak = $p.PeakWorkingSet64 } } catch {}
        Start-Sleep -Milliseconds 20
    }
    try { $p.Refresh(); if ($p.PeakWorkingSet64 -gt $peak) { $peak = $p.PeakWorkingSet64 } } catch {}
    return $peak
}

function Start-To-Health($label) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $atlas -ArgumentList @("--data-dir", $data, "--port", "8000") -WorkingDirectory $server -PassThru -WindowStyle Hidden -RedirectStandardOutput "$env:TEMP\db4c-server-$label.txt" -RedirectStandardError "$env:TEMP\db4c-server-$label-err.txt"
    $ok = $false
    while ($sw.Elapsed.TotalSeconds -lt 60) {
        try {
            $r = Invoke-WebRequest -Uri "http://127.0.0.1:8000/health" -UseBasicParsing -TimeoutSec 1
            if ($r.StatusCode -eq 200) { $ok = $true; break }
        } catch {}
        if ($p.HasExited) { break }
        Start-Sleep -Milliseconds 25
    }
    $t = $sw.Elapsed.TotalMilliseconds
    $peak = 0
    try { $p.Refresh(); $peak = $p.PeakWorkingSet64 } catch {}
    try { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue } catch {}
    Start-Sleep -Milliseconds 300
    if (-not $ok) { throw "atlas-server did not answer /health within 60 s ($label); see $env:TEMP\db4c-server-$label-err.txt" }
    "{0} start -> /health 200: {1:N0} ms (server peak working set {2:N1} MiB)" -f $label, $t, ($peak / 1MB)
}

"== DB-4c measurements (debug build) =="
$rssMiB = (Peak-Rss $bibex @("--data-dir", $data, "verse", "JHN.3.16")) / 1MB
"bibex verse JHN.3.16 peak working set: {0:N1} MiB (OVERLAY-1 baseline 755.0 MiB over graph.bin + JSON)" -f $rssMiB
$t = Measure-Command { & $bibex --data-dir $data verse JHN.3.16 | Out-Null }
"bibex verse JHN.3.16 wall: {0:N0} ms" -f $t.TotalMilliseconds
if (Test-Path $cache) { Remove-Item (Join-Path $cache "*.sqlite") -Force -ErrorAction SilentlyContinue }
Start-To-Health "cold-cache"
Start-To-Health "warm-cache"
