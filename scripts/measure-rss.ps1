# OVERLAY-1: peak resident memory of one process run, by the method DB-1 used
# (appendix M2): Start-Process -PassThru, sample PeakWorkingSet64 every 20 ms.
# Usage: powershell -File scripts/measure-rss.ps1 <exe> [args...]
param([Parameter(Mandatory=$true)][string]$Exe, [Parameter(ValueFromRemainingArguments=$true)][string[]]$Args)
$p = Start-Process -FilePath $Exe -ArgumentList $Args -PassThru -NoNewWindow -RedirectStandardOutput "$env:TEMP\measure-rss.out"
$peak = 0L
while (-not $p.HasExited) { try { $p.Refresh(); if ($p.PeakWorkingSet64 -gt $peak) { $peak = $p.PeakWorkingSet64 } } catch {} ; Start-Sleep -Milliseconds 20 }
try { $p.Refresh(); if ($p.PeakWorkingSet64 -gt $peak) { $peak = $p.PeakWorkingSet64 } } catch {}
"PEAK_WORKING_SET_MIB={0:N1}" -f ($peak / 1MB)
"EXIT_CODE=$($p.ExitCode)"
