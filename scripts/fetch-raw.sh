#!/usr/bin/env bash
# macOS/Linux port of data/fetch-raw.ps1 -- same sources, same destination
# filenames, same idempotency (anything already present is left alone; delete
# it and re-run to refresh). data/raw/README.md documents what each artifact
# is and why its URL is what it is; keep the two scripts in sync.
#
#   scripts/fetch-raw.sh                 # everything
#   scripts/fetch-raw.sh kjv xrefs       # just those artifacts
#
# The per-artifact form is what the Makefile's own fetch rules call, so each
# raw file is a real make target with a real recipe rather than one
# all-or-nothing script invocation.
set -euo pipefail

ARTIFACTS=(kjv geo theographic xrefs catechism bible concord redletter kretzmann leaflet)
CATECHISM_SHA='0be24fee92e6333f817c4c2a08f99cf7c5274295'
BIBLE_SHA='94d44842cb242e8aa840330748e03d2803f2a7c1'

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
raw="$repo_dir/data/raw"

fetch() { # fetch <url> <path-relative-to-data/raw>
  local url="$1" out="$raw/$2"
  if [[ -s "$out" ]]; then echo "have $2"; return; fi
  echo "fetch $url"
  mkdir -p "$(dirname "$out")"
  curl -fsSL --retry 3 -o "$out" "$url" || { rm -f "$out"; return 1; }
}

# KJV text. Upstream renamed kjv.json -> KJV.json; saved locally under the
# original lowercase name every consumer expects.
get_kjv() {
  fetch 'https://raw.githubusercontent.com/scrollmapper/bible_databases/master/formats/json/KJV.json' 'kjv.json'
}

# OpenBible geocoding data -- JSON Lines on GitHub (the old data.zip 403s).
get_geo() {
  local f
  for f in ancient modern geometry image source; do
    fetch "https://raw.githubusercontent.com/openbibleinfo/Bible-Geocoding-Data/master/data/$f.jsonl" "geo/$f.jsonl"
  done
}

get_theographic() {
  fetch 'https://github.com/robertrouse/theographic-bible-metadata/archive/refs/heads/master.zip' 'theographic.zip'
  [[ -d "$raw/theographic" ]] || unzip -q "$raw/theographic.zip" -d "$raw/theographic"
}

get_xrefs() {
  fetch 'https://a.openbible.info/data/cross-references.zip' 'cross-references.zip'
  [[ -d "$raw/xrefs" ]] || unzip -q "$raw/cross-references.zip" -d "$raw/xrefs"
}

# brain-fuel/catechism, pinned at a commit rather than a branch.
get_catechism() {
  fetch "https://github.com/brain-fuel/catechism/archive/$CATECHISM_SHA.zip" 'catechism-mapping.zip'
  [[ -d "$raw/catechism-mapping/catechism-$CATECHISM_SHA" ]] || \
    unzip -q "$raw/catechism-mapping.zip" -d "$raw/catechism-mapping"
}

# Batch CORP-1a: brain-fuel/bible parallel editions, pinned at a commit. The
# upstream repo is ~29k files (apocrypha, morphology, lexicon, tooling this
# app never reads), so only data/books.json plus the ot/ and nt/ chapter
# JSONs are kept -- the same subset data/fetch-raw.ps1 vendors, and the
# reason data/raw/brain-fuel-bible/ is a curated copy rather than the whole
# extracted archive.
get_bible() {
  local dest="$raw/brain-fuel-bible"
  if [[ -d "$dest" ]]; then echo "have brain-fuel-bible"; return; fi
  fetch "https://github.com/brain-fuel/bible/archive/$BIBLE_SHA.zip" 'brain-fuel-bible-src.zip'
  local tmp="$raw/brain-fuel-bible-src-extract"
  rm -rf "$tmp"
  unzip -q "$raw/brain-fuel-bible-src.zip" -d "$tmp"
  local src="$tmp/bible-$BIBLE_SHA"
  mkdir -p "$dest/data"
  cp "$src/data/books.json" "$dest/data/books.json"
  cp -R "$src/bible/ot" "$dest/ot"
  cp -R "$src/bible/nt" "$dest/nt"
  rm -rf "$tmp" "$raw/brain-fuel-bible-src.zip"
}

# Batch CORP-2a: the Book of Concord (1921 Bente-Dau, bookofconcord.org).
# Only the TEN document ROOTS -- each already embeds the full text of its
# own articles inline, so the ~150 per-article pages are redundant. The
# Smalcald Articles are the one exception: its root carries only blurbs,
# and the real numbered text lives on 23 deeper pages, vendored into their
# own subdirectory so the root fetch stays uniform. See
# server/atlas-etl/src/concord.rs and data/fetch-raw.ps1 for the full
# discovery record.
get_concord() {
  local doc slug sub
  for doc in preface ecumenical-creeds augsburg-confession defense smalcald-articles \
             power-and-primacy small-catechism large-catechism epitome solid-declaration; do
    fetch "https://bookofconcord.org/$doc/" "concord/$doc.html"
  done
  for sub in i/nature-of-god i/the-father i/the-son i/the-work-of-salvation \
             ii/first-and-chief-article ii/of-the-mass ii/of-chapters-and-cloisters ii/of-the-papacy \
             iii/of-sin iii/of-the-law iii/of-repentance iii/of-the-gospel iii/of-baptism \
             iii/of-the-scarament-of-the-altar iii/of-the-keys iii/of-confession iii/of-excommunication \
             iii/of-ordination iii/of-the-marriage-of-priests iii/of-the-church iii/of-good-works \
             iii/of-monastic-vows iii/of-human-tradition; do
    slug="${sub##*/}"
    fetch "https://bookofconcord.org/smalcald-articles/$sub/" "concord/smalcald-sub/$slug.html"
  done
}

# RED-1: the KJV OSIS with red-letter (words-of-Christ) markup, pinned at a
# commit.
get_redletter() {
  fetch 'https://raw.githubusercontent.com/seven1m/open-bibles/f257a3559025c3f873b48a75019f53a9354ed7de/eng-kjv.osis.xml' 'red-letter/eng-kjv.osis.xml'
}

# KRETZ-1: Kretzmann's Popular Commentary, one page per chapter from
# kretzmanncommentary.org -- 1,189 requests. Rate-limited at 200ms like the
# PowerShell original (a small site; do not hammer it), so a cold fetch
# takes several minutes. Missing chapters are REPORTED, never silently
# skipped: the parser's own coverage law would otherwise read a gap as an
# absent commentary rather than a failed download.
get_kretzmann() {
  local spec slug chapters c out missing=0
  for spec in genesis:50 exodus:40 leviticus:27 numbers:36 deuteronomy:34 \
    joshua:24 judges:21 ruth:4 1-samuel:31 2-samuel:24 1-kings:22 2-kings:25 \
    1-chronicles:29 2-chronicles:36 ezra:10 nehemiah:13 esther:10 job:42 \
    psalms:150 proverbs:31 ecclesiastes:12 song-of-solomon:8 isaiah:66 \
    jeremiah:52 lamentations:5 ezekiel:48 daniel:12 hosea:14 joel:3 amos:9 \
    obadiah:1 jonah:4 micah:7 nahum:3 habakkuk:3 zephaniah:3 haggai:2 \
    zechariah:14 malachi:4 matthew:28 mark:16 luke:24 john:21 acts:28 \
    romans:16 1-corinthians:16 2-corinthians:13 galatians:6 ephesians:6 \
    philippians:4 colossians:4 1-thessalonians:5 2-thessalonians:3 \
    1-timothy:6 2-timothy:4 titus:3 philemon:1 hebrews:13 james:5 1-peter:5 \
    2-peter:3 1-john:5 2-john:1 3-john:1 jude:1 revelation:22; do
    slug="${spec%%:*}"; chapters="${spec##*:}"
    mkdir -p "$raw/kretzmann/$slug"
    for ((c = 1; c <= chapters; c++)); do
      out="$raw/kretzmann/$slug/$c.html"
      [[ -s "$out" ]] && continue
      if ! curl -fsSL --retry 3 --max-time 30 -o "$out" "https://kretzmanncommentary.org/$slug/$c"; then
        rm -f "$out"
        echo "kretzmann MISSING (primary failed): $slug/$c" >&2
        missing=$((missing + 1))
      fi
      sleep 0.2
    done
  done
  echo "kretzmann: $missing chapter(s) missing"
}

# Leaflet 1.9.4 is committed to the repo; fetched only if the vendor dir is bare.
get_leaflet() {
  local vendor="$repo_dir/client/wwwroot/vendor/leaflet" f
  mkdir -p "$vendor"
  for f in leaflet.js leaflet.css; do
    if [[ -s "$vendor/$f" ]]; then echo "have vendor/leaflet/$f"; continue; fi
    echo "fetch leaflet 1.9.4 $f"
    curl -fsSL --retry 3 -o "$vendor/$f" "https://unpkg.com/leaflet@1.9.4/dist/$f" \
      || { rm -f "$vendor/$f"; return 1; }
  done
}

wanted=("$@")
[[ ${#wanted[@]} -gt 0 ]] || wanted=("${ARTIFACTS[@]}")

mkdir -p "$raw"
for key in "${wanted[@]}"; do
  case " ${ARTIFACTS[*]} " in
    *" $key "*) "get_$key" ;;
    *) echo "unknown artifact '$key' (known: ${ARTIFACTS[*]})" >&2; exit 2 ;;
  esac
done
