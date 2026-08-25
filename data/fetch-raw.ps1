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

# Batch CORP-2a: the Book of Concord (1921 Bente-Dau translation, Concordia
# Triglotta's English column), vendored from bookofconcord.org -- the
# ONLY complete Bente-Dau set online (see corp2-scouting.md's 2026-08-24
# RE-SCOUT: Project Wittenberg is missing the Apology/Formula of Concord/
# Creeds/Preface entirely, and its Small Catechism is the copyrighted
# Smith 1994 translation -- NEVER vendor that). DISCOVERY (verified by
# fetching a document root and inspecting its HTML): each document's own
# ROOT page already embeds the FULL text of every one of its articles
# inline (heading + numbered paragraphs), not just a table of contents --
# so only the 10 document ROOTS are fetched here, never the ~150
# individual per-article pages nested under them (confirmed redundant:
# fetching e.g. /augsburg-confession/of-justification/ returns the exact
# same paragraph text /augsburg-confession/ already carries for Article
# IV). Ten document roots = the traditional Book of Concord order
# (Preface to the whole 1580 volume, then the Three Ecumenical Creeds,
# then the six 16th-century confessional documents, then the Formula of
# Concord's two forms) -- see server/atlas-etl/src/concord.rs's own
# module doc comment for the part-numbering this order feeds.
$concordDir = Join-Path $raw 'concord'
New-Item -ItemType Directory -Force $concordDir | Out-Null
$concordDocs = @(
  'preface',
  'ecumenical-creeds',
  'augsburg-confession',
  'defense',
  'smalcald-articles',
  'power-and-primacy',
  'small-catechism',
  'large-catechism',
  'epitome',
  'solid-declaration'
)
foreach ($doc in $concordDocs) {
  Fetch "https://bookofconcord.org/$doc/" "concord\$doc.html"
}
# Smalcald Articles EXCEPTION (discovered parsing the vendored root page,
# not assumed up front): unlike every other document, `/smalcald-articles/`
# does NOT embed Parts I/II/III's own 4+4+15 named articles inline -- each
# Part's own root section is just a one-paragraph blurb, and the real,
# numbered article text lives ONLY on 23 separate per-article pages one
# level deeper (e.g. `/smalcald-articles/iii/of-sin/`), a genuinely
# different page template (`<h2>TITLE</h2>` + numbered paragraphs, no
# `<a href><h3>...</section>` wrapper at all -- see concord.rs's own
# module doc comment). Vendored into their own subdirectory so the
# document-root fetch above stays uniform across all ten documents.
$smalcaldSubDir = Join-Path $raw 'concord\smalcald-sub'
New-Item -ItemType Directory -Force $smalcaldSubDir | Out-Null
$smalcaldSubArticles = @(
  'i/nature-of-god', 'i/the-father', 'i/the-son', 'i/the-work-of-salvation',
  'ii/first-and-chief-article', 'ii/of-the-mass', 'ii/of-chapters-and-cloisters', 'ii/of-the-papacy',
  'iii/of-sin', 'iii/of-the-law', 'iii/of-repentance', 'iii/of-the-gospel', 'iii/of-baptism',
  'iii/of-the-scarament-of-the-altar', 'iii/of-the-keys', 'iii/of-confession', 'iii/of-excommunication',
  'iii/of-ordination', 'iii/of-the-marriage-of-priests', 'iii/of-the-church', 'iii/of-good-works',
  'iii/of-monastic-vows', 'iii/of-human-tradition'
)
foreach ($sub in $smalcaldSubArticles) {
  $slug = $sub.Split('/')[-1]
  Fetch "https://bookofconcord.org/smalcald-articles/$sub/" "concord\smalcald-sub\$slug.html"
}

# Batch KRETZ-1: Kretzmann's Popular Commentary of the Bible (Paul E.
# Kretzmann, Concordia Publishing House, 1921-1924) -- owner order
# (2026-08-24, via the controller): "pull kretzmann commentary (public
# domain version) into our corpora". PRIMARY source kretzmanncommentary.org
# (a modern digital edition; the site's own footer/about text states the
# work is PD, doubly grounded per kretzmann-scouting.md: published without
# copyright notice AND all four original volumes predate 1930 regardless).
# Controller decision 1: "all 66 books, every chapter page" -- book INTRO
# pages (`/{slug}/intro`) are explicitly out of scope, chapter pages only.
# `$kretzmannBooks` is (slug, chapter-count), in the SAME 66-book canonical
# order as `atlas_core::canon::BOOKS` (server/atlas-core/src/canon.rs) --
# verified 2026-08-25 by scraping kretzmanncommentary.org/bible's own
# book/chapter link listing: both lists' chapter counts match position-for-
# position, Genesis(50)..Revelation(22), summing to exactly 1,189 (the
# standard KJV chapter total), so `BOOKS[i]` <-> `$kretzmannBooks[i]`
# requires no name-fuzzy-matching join. FALLBACK (kretzmannproject.org, per
# controller decision 1) was live-probed 2026-08-25 and TIMED OUT (matches
# kretzmann-scouting.md's own prior finding, "server rejected our fetcher on
# first probe") -- no verified URL scheme exists to fall back to, so a
# primary-fetch failure (after retries) is a genuine MISSING page, disclosed
# below, never silently guessed at with an unverified fallback URL.
$kretzmannDir = Join-Path $raw 'kretzmann'
New-Item -ItemType Directory -Force $kretzmannDir | Out-Null
$kretzmannBooks = @(
  @('genesis',50), @('exodus',40), @('leviticus',27), @('numbers',36), @('deuteronomy',34),
  @('joshua',24), @('judges',21), @('ruth',4), @('1-samuel',31), @('2-samuel',24),
  @('1-kings',22), @('2-kings',25), @('1-chronicles',29), @('2-chronicles',36), @('ezra',10),
  @('nehemiah',13), @('esther',10), @('job',42), @('psalms',150), @('proverbs',31),
  @('ecclesiastes',12), @('song-of-solomon',8), @('isaiah',66), @('jeremiah',52), @('lamentations',5),
  @('ezekiel',48), @('daniel',12), @('hosea',14), @('joel',3), @('amos',9),
  @('obadiah',1), @('jonah',4), @('micah',7), @('nahum',3), @('habakkuk',3),
  @('zephaniah',3), @('haggai',2), @('zechariah',14), @('malachi',4), @('matthew',28),
  @('mark',16), @('luke',24), @('john',21), @('acts',28), @('romans',16),
  @('1-corinthians',16), @('2-corinthians',13), @('galatians',6), @('ephesians',6), @('philippians',4),
  @('colossians',4), @('1-thessalonians',5), @('2-thessalonians',3), @('1-timothy',6), @('2-timothy',4),
  @('titus',3), @('philemon',1), @('hebrews',13), @('james',5), @('1-peter',5),
  @('2-peter',3), @('1-john',5), @('2-john',1), @('3-john',1), @('jude',1),
  @('revelation',22)
)
$kretzmannTotal = ($kretzmannBooks | ForEach-Object { $_[1] } | Measure-Object -Sum).Sum
$kretzmannFetched = 0
$kretzmannHad = 0
$kretzmannMissing = @()
foreach ($book in $kretzmannBooks) {
  $slug = $book[0]; $chapters = $book[1]
  $bookDir = Join-Path $kretzmannDir $slug
  New-Item -ItemType Directory -Force $bookDir | Out-Null
  for ($c = 1; $c -le $chapters; $c++) {
    $out = Join-Path $bookDir "$c.html"
    if (Test-Path $out) { $kretzmannHad++; continue }
    $url = "https://kretzmanncommentary.org/$slug/$c"
    $ok = $false
    for ($attempt = 1; $attempt -le 3 -and -not $ok; $attempt++) {
      try {
        Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing -TimeoutSec 30
        $ok = $true
      } catch {
        if ($attempt -lt 3) { Start-Sleep -Milliseconds (500 * $attempt) }
      }
    }
    if ($ok) {
      $kretzmannFetched++
    } else {
      Write-Output "kretzmann MISSING (primary failed x3, no verified fallback): $slug/$c"
      $kretzmannMissing += "$slug/$c"
    }
    Start-Sleep -Milliseconds 200
  }
}
Write-Output "kretzmann fetch: $kretzmannFetched fetched, $kretzmannHad already cached, $($kretzmannMissing.Count) missing of $kretzmannTotal total pages"

# Vendor Leaflet 1.9.4 into the client (deterministic, offline-friendly)
$vendor = Join-Path $PSScriptRoot '..\client\wwwroot\vendor\leaflet'
New-Item -ItemType Directory -Force $vendor | Out-Null
foreach ($f in 'leaflet.js','leaflet.css') {
  $p = Join-Path $vendor $f
  if (-not (Test-Path $p)) { Invoke-WebRequest "https://unpkg.com/leaflet@1.9.4/dist/$f" -OutFile $p -UseBasicParsing }
}
Write-Output 'fetch-raw complete'
