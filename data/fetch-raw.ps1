$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$raw = Join-Path $PSScriptRoot 'raw'
New-Item -ItemType Directory -Force $raw | Out-Null

function Fetch($url, $out) {
  $path = Join-Path $raw $out
  if (Test-Path $path) { Write-Output "have $out"; return }
  Write-Output "fetch $url"
  Invoke-WebRequest -Uri $url -OutFile $path -UseBasicParsing
}

# KJV text (single-file JSON; fallback repo noted in raw/README.md)
# NOTE: upstream renamed formats/json/kjv.json -> formats/json/KJV.json (uppercase); see data/raw/README.md
Fetch 'https://raw.githubusercontent.com/scrollmapper/bible_databases/master/formats/json/KJV.json' 'kjv.json'
# OpenBible geocoding bundle
# NOTE: the old a.openbible.info/geo/data.zip bundle now 403s (removed from S3/CloudFront).
# openbible.info's geocoding data moved to GitHub as JSON Lines files (no single zip); see data/raw/README.md.
$geoDir = Join-Path $raw 'geo'
New-Item -ItemType Directory -Force $geoDir | Out-Null
foreach ($f in 'ancient.jsonl','modern.jsonl','geometry.jsonl','image.jsonl','source.jsonl') {
  Fetch "https://raw.githubusercontent.com/openbibleinfo/Bible-Geocoding-Data/master/data/$f" "geo\$f"
}
# Theographic metadata (whole repo)
Fetch 'https://github.com/robertrouse/theographic-bible-metadata/archive/refs/heads/master.zip' 'theographic.zip'
if (-not (Test-Path (Join-Path $raw 'theographic'))) { Expand-Archive (Join-Path $raw 'theographic.zip') (Join-Path $raw 'theographic') }
# Cross references (TSV with votes)
Fetch 'https://a.openbible.info/data/cross-references.zip' 'cross-references.zip'
if (-not (Test-Path (Join-Path $raw 'xrefs'))) { Expand-Archive (Join-Path $raw 'cross-references.zip') (Join-Path $raw 'xrefs') }
# Historical border snapshots (aourednik/historical-basemaps, GPL-3.0 --
# license terms + attribution recorded in data/raw/README.md's Borders
# section). 12 snapshots covering/nearest our [-4004,100] span: filenames
# are "world_bc{N}.geojson" (N BC) or "world_{N}.geojson" (AD N); the
# repo's actual file list was verified against its GitHub tree before
# picking these (see data/raw/README.md).
$bordersDir = Join-Path $raw 'borders'
New-Item -ItemType Directory -Force $bordersDir | Out-Null
$borderYears = 'bc4000', 'bc3000', 'bc2000', 'bc1500', 'bc1000', 'bc700', 'bc500', 'bc323', 'bc200', 'bc100', 'bc1', '100'
foreach ($y in $borderYears) {
  Fetch "https://raw.githubusercontent.com/aourednik/historical-basemaps/master/geojson/world_$y.geojson" "borders\world_$y.geojson"
}

# Vendor Leaflet 1.9.4 into the client (deterministic, offline-friendly)
$vendor = Join-Path $PSScriptRoot '..\client\wwwroot\vendor\leaflet'
New-Item -ItemType Directory -Force $vendor | Out-Null
foreach ($f in 'leaflet.js','leaflet.css') {
  $p = Join-Path $vendor $f
  if (-not (Test-Path $p)) { Invoke-WebRequest "https://unpkg.com/leaflet@1.9.4/dist/$f" -OutFile $p -UseBasicParsing }
}
Write-Output 'fetch-raw complete'
