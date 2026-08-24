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

# Batch F2: the user's own catechism verse-mapping repo (brain-fuel/catechism)
# -- "I gave you the mapping very explicitly in the catechism repo" (user
# direction 2026-08-20). Fetched as a GitHub commit-archive zip, PINNED at a
# specific SHA (not a branch) for reproducibility -- same "whole repo" zip
# pattern the Theographic fetch above already uses, just keyed to a commit
# rather than a branch name. See LICENSES.md for the full provenance/license
# disposition (controller ruling) and data/curated/catechism-mapping.toml's
# own header for exactly which files this ingests vs. deliberately defers.
$catechismSha = '0be24fee92e6333f817c4c2a08f99cf7c5274295'
Fetch "https://github.com/brain-fuel/catechism/archive/$catechismSha.zip" 'catechism-mapping.zip'
if (-not (Test-Path (Join-Path $raw "catechism-mapping\catechism-$catechismSha"))) {
  Expand-Archive (Join-Path $raw 'catechism-mapping.zip') (Join-Path $raw 'catechism-mapping')
}
# Batch CORP-1a: brain-fuel/bible editions (Clementine Vulgate, Westminster
# Leningrad Codex, Douay-Rheims, Biblia 1776, Karl XII:s Bibel, Greek Textus
# Receptus) -- owner order (verbatim, via the controller): "3 - take all.
# no apocrypha for now." Fetched as a GitHub commit-archive zip, PINNED at a
# specific SHA (not a branch), same pattern as the catechism-mapping fetch
# above -- except this repo is large (~29k files: apocrypha/Septuagint
# texts, morphology, lexicon, relation-graph data, Go/Python tooling this
# app never reads), so only `data/books.json` (the book-code/kjv_name
# manifest this app's own parser needs -- see server/atlas-etl/src/
# brainfuel.rs) and the `bible/ot/`+`bible/nt/` chapter JSONs (929+260
# files) are copied into data/raw/brain-fuel-bible/; the rest of the
# extracted zip is discarded. See LICENSES.md for the full provenance/
# license disposition and data/raw/README.md for the verified JSON shape.
$bibleSha = '94d44842cb242e8aa840330748e03d2803f2a7c1'
$bibleVendored = Join-Path $raw 'brain-fuel-bible'
if (-not (Test-Path $bibleVendored)) {
  Fetch "https://github.com/brain-fuel/bible/archive/$bibleSha.zip" 'brain-fuel-bible-src.zip'
  $bibleExtractTmp = Join-Path $raw 'brain-fuel-bible-src-extract'
  Expand-Archive (Join-Path $raw 'brain-fuel-bible-src.zip') $bibleExtractTmp
  $srcRoot = Join-Path $bibleExtractTmp "bible-$bibleSha"
  New-Item -ItemType Directory -Force (Join-Path $bibleVendored 'data') | Out-Null
  Copy-Item (Join-Path $srcRoot 'data\books.json') (Join-Path $bibleVendored 'data\books.json')
  Copy-Item (Join-Path $srcRoot 'bible\ot') (Join-Path $bibleVendored 'ot') -Recurse
  Copy-Item (Join-Path $srcRoot 'bible\nt') (Join-Path $bibleVendored 'nt') -Recurse
  Remove-Item $bibleExtractTmp -Recurse -Force
  Remove-Item (Join-Path $raw 'brain-fuel-bible-src.zip') -Force
}

# Historical border snapshots are NOT fetched -- Batch L (license
# remediation) removed the aourednik/historical-basemaps (GPL-3.0) source
# that used to be fetched here. Historical borders are now this project's
# own hand-curated, CC0 data at data/curated/borders/*.geojson, committed
# to the repo like the rest of data/curated/. See LICENSES.md.

# Vendor Leaflet 1.9.4 into the client (deterministic, offline-friendly)
$vendor = Join-Path $PSScriptRoot '..\client\wwwroot\vendor\leaflet'
New-Item -ItemType Directory -Force $vendor | Out-Null
foreach ($f in 'leaflet.js','leaflet.css') {
  $p = Join-Path $vendor $f
  if (-not (Test-Path $p)) { Invoke-WebRequest "https://unpkg.com/leaflet@1.9.4/dist/$f" -OutFile $p -UseBasicParsing }
}
Write-Output 'fetch-raw complete'
