//! Batch CORP-1a ("brain-fuel editions: the ingestion half"): parser for
//! `data/raw/brain-fuel-bible/` -- a vendored subset of
//! github.com/brain-fuel/bible (pinned commit `94d44842cb242e8aa840330748e03d2803f2a7c1`;
//! see `data/raw/README.md` for the fetch/vendor procedure and `LICENSES.md`
//! for the per-edition license disposition). Owner order (verbatim, via the
//! controller): "3 - take all. no apocrypha for now."
//!
//! ONE parser, edition-parameterized (controller decision 3) -- never six
//! copies. The shape below was verified DIRECTLY against the real vendored
//! files (929 OT + 260 NT chapter JSONs, every one of the 31,102 aligned
//! verse positions swept programmatically), not assumed from the source
//! repo's own README, per this batch's own brief ("the controller's
//! scouting has been wrong twice from sampling too little").
//!
//! # The real, verified shape
//!
//! One JSON object per `bible/{ot,nt}/{CODE}/{NNN}.json` (`CODE` is
//! brain-fuel's OWN three-letter book code -- NOT this app's OSIS-ish
//! `atlas_core::canon::BOOKS` codes; e.g. `JOH` not `JHN`, `MAR` not `MRK`,
//! `SOS` not `SNG`, `JDE` not `JUD`. Resolved via `data/books.json`'s own
//! `kjv_name` field instead of a hand-maintained code table: that field uses
//! the IDENTICAL old-style KJV book-name convention
//! (`server/atlas-etl/src/kjv.rs`'s own `kjv.json` source already parses --
//! "I Samuel", "Revelation of John", etc.) -- one normalizer
//! (`kjv::normalize_book_name`), reused, not forked):
//!
//! ```json
//! { "book_id": "GEN", "chapter": 1, "verses": [
//!   { "verse": 1,
//!     "latin_vulgate": "In principio creavit Deus...",
//!     "hebrew_masoretic": "בְּרֵאשִׁ֖ית...",
//!     "douay_rheims": "In the beginning God created...",
//!     "finnish_biblia": "Alussa loi Jumala...",
//!     "swedish_karl_xii": "J Begynnelsen skapade...",
//!     "king_james": "In the beginning God created the heaven and the earth."
//!   }, ...
//! ] }
//! ```
//!
//! `verse` is ALREADY the KJV skeleton position (controller decision 3: "the
//! brain-fuel chapter JSONs share one shape (parallel verse columns
//! PRE-ALIGNED at KJV positions...)") -- every chapter file's own verse
//! numbers run contiguously `1..=N` with no gaps, matching this app's own
//! KJV verse counts exactly (swept: 23,145 OT + 7,957 NT = 31,102, the SAME
//! total `atlas_etl::kjv::parse` produces from `data/raw/kjv.json`).
//!
//! An edition's own field key is present on EVERY verse of a chapter file,
//! or on NONE of them -- a testament-level fact (which editions
//! `data/editions.json` upstream registers for `ot`/`nt`), never a
//! per-verse one. Swept and confirmed over the real data: OT chapters carry
//! `latin_vulgate`/`hebrew_masoretic`/`douay_rheims`/`finnish_biblia`/
//! `swedish_karl_xii`/`king_james` (never `greek_textus_receptus`); NT
//! chapters carry `latin_vulgate`/`greek_textus_receptus`/`finnish_biblia`/
//! `swedish_karl_xii`/`king_james` (never `hebrew_masoretic` or
//! `douay_rheims` -- Douay-Rheims ships OT-only in this dataset, confirmed
//! against both `data/editions.json`'s own manifest row and the real files,
//! disclosed in this batch's report). `RawVerse`'s six non-KJV fields are
//! all `Option<String>` for exactly this reason: serde leaves a field `None`
//! when its key is simply absent from the JSON object, no per-testament
//! branching needed in this parser at all.
//!
//! An `ABSENT` marker (`refs.<edition>.absent: true`) is a PER-VERSE gap
//! within an edition that otherwise applies to the testament (e.g. ten OT
//! verses merged into the preceding verse in the Vulgate tradition). Loud
//! gotcha, verified on the real data (`bible/ot/1CH/011.json` verse 47,
//! among others): when a verse is `absent`-marked, its own text KEY IS
//! STILL PRESENT in the JSON, holding an EMPTY STRING (`""`) -- never
//! simply omitted. This parser reads `refs.<edition>.absent` FIRST and, if
//! true, produces NO rendering at all for that edition at that position
//! (`RenderingOutcome::Absent`) -- the empty string is never treated as
//! content (batch brief requirement 4: "absence is data, never an empty
//! string"). Swept over the entire real dataset: every `absent`-marked
//! field's own text value is exactly `""` (zero anomalies -- confirmed by
//! `read_all`'s own `stats.anomalies`, asserted `0` in this batch's
//! real-data tests) and no edition is ever empty WITHOUT an `absent`
//! marker; `RenderingOutcome::Anomaly` exists purely as an honest fallback
//! for a future drift this parser has never actually observed, not a
//! shape this batch's own data needs.
//!
//! `refs.<edition>.src` is versification PROVENANCE only (the source
//! tradition's own local `"chapter:verse"` label for a position already
//! aligned to the KJV skeleton, e.g. Hebrew/Latin Psalm-title-shifted
//! numbering) -- per batch brief requirement 4, it imports as NOTHING this
//! batch (the cross-corpus correspondence design is CORP-2's scope): this
//! parser reads the rendering at its ALREADY-CORRECT KJV position and
//! never stores or acts on `src` beyond disclosing its count in
//! `ParseStats::src_notes` (2,835 Vulgate + 1,971 Hebrew + 1,303+1 Swedish
//! + 2,835 Douay-Rheims over the real data -- Psalm-title shifts and
//! similar, not alignment errors).
//!
//! `king_james` is parsed (`VerseRow.king_james`) ONLY for the owner-ordered
//! cross-check (`kjv_cross_check` below) against this app's own canonical
//! KJV text -- per the KJV INERRANCY DIRECTIVE and batch brief requirement
//! 5, it is NEVER imported into this app's graph; our KJV base is
//! authoritative and stays the only KJV rendering the graph ever carries.
//!
//! # Apocrypha
//!
//! `bible/apo/` and `bible/lxx/` are not read here at all -- not vendored
//! (`data/raw/brain-fuel-bible/` carries only `data/books.json` + `ot/` +
//! `nt/`), per the owner's own ruling ("no apocrypha for now"). KJVA
//! (`king_james_apocrypha`) is SKIPPED outright, not just descoped: its
//! canonical 66-book coverage would exactly duplicate this app's own KJV
//! base, and its only UNIQUE content is the apocryphal books this batch
//! already excludes -- ingesting it would add no real content, disclosed
//! in this batch's own report rather than silently omitted.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{bail, Context, Result};
use atlas_core::canon::resolve_alias;
use atlas_core::refs::BookId;
use serde::Deserialize;

use crate::kjv::normalize_book_name;

/// The six editions this batch ingests as renderings (controller decision
/// 2) -- each string is BOTH the exact JSON field name on `RawVerse` AND
/// the `TranslationId` slug the graph adapter stamps onto the corresponding
/// `LayerMap` entry (one vocabulary, not two). `king_james` (the base/
/// canonical layer, already ingested since M-A) and `king_james_apocrypha`
/// (skipped, see module doc comment) are deliberately absent from this
/// list.
pub const EDITIONS: &[&str] =
    &["latin_vulgate", "hebrew_masoretic", "douay_rheims", "finnish_biblia", "swedish_karl_xii", "greek_textus_receptus"];

#[derive(Deserialize)]
struct RawBooksManifest {
    books: Vec<RawBookMeta>,
}

#[derive(Deserialize)]
struct RawBookMeta {
    code: String,
    testament: String,
    #[serde(default)]
    kjv_name: Option<String>,
}

/// Parses `data/raw/brain-fuel-bible/data/books.json` into a `{brain-fuel
/// code -> BookId}` map, covering exactly the 66 `ot`/`nt` rows (the 14
/// `apo` rows are skipped -- this batch never reads `bible/apo/`). Resolves
/// each row's own `kjv_name` through the SAME `kjv::normalize_book_name` +
/// `canon::resolve_alias` pipeline `kjv.rs` already uses for
/// `data/raw/kjv.json`'s own old-style names -- verified: brain-fuel's
/// `kjv_name` field uses the IDENTICAL convention ("I Samuel", "Revelation
/// of John", ...). Fails loudly (names every unresolved row) rather than
/// silently dropping a book -- a gap here would silently drop that book's
/// entire chapter set from every one of the six editions.
pub(crate) fn book_code_map(books_json: &str) -> Result<HashMap<String, BookId>> {
    let manifest: RawBooksManifest = serde_json::from_str(books_json).context("brain-fuel data/books.json is not valid JSON")?;
    let mut map = HashMap::new();
    let mut unresolved: Vec<String> = Vec::new();
    for b in &manifest.books {
        if b.testament != "ot" && b.testament != "nt" {
            continue; // apo -- out of scope this batch, never read
        }
        let Some(kjv_name) = &b.kjv_name else {
            unresolved.push(format!("{} (testament {}, no kjv_name field)", b.code, b.testament));
            continue;
        };
        let normalized = normalize_book_name(kjv_name);
        match resolve_alias(&normalized) {
            Some(id) => {
                map.insert(b.code.clone(), id);
            }
            None => unresolved.push(format!("{} (kjv_name '{kjv_name}', normalized '{normalized}')", b.code)),
        }
    }
    if !unresolved.is_empty() {
        bail!("brain-fuel books.json: {} ot/nt row(s) failed to resolve to a canonical book: {}", unresolved.len(), unresolved.join("; "));
    }
    Ok(map)
}

#[derive(Deserialize)]
struct RawChapterFile {
    book_id: String,
    chapter: u16,
    verses: Vec<RawVerse>,
}

#[derive(Deserialize)]
struct RawVerse {
    verse: u16,
    #[serde(default)]
    king_james: Option<String>,
    #[serde(default)]
    latin_vulgate: Option<String>,
    #[serde(default)]
    hebrew_masoretic: Option<String>,
    #[serde(default)]
    douay_rheims: Option<String>,
    #[serde(default)]
    finnish_biblia: Option<String>,
    #[serde(default)]
    swedish_karl_xii: Option<String>,
    #[serde(default)]
    greek_textus_receptus: Option<String>,
    #[serde(default)]
    refs: BTreeMap<String, RawRefEntry>,
}

#[derive(Deserialize, Default)]
struct RawRefEntry {
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    absent: bool,
}

impl RawVerse {
    fn field(&self, edition: &str) -> Option<&str> {
        match edition {
            "latin_vulgate" => self.latin_vulgate.as_deref(),
            "hebrew_masoretic" => self.hebrew_masoretic.as_deref(),
            "douay_rheims" => self.douay_rheims.as_deref(),
            "finnish_biblia" => self.finnish_biblia.as_deref(),
            "swedish_karl_xii" => self.swedish_karl_xii.as_deref(),
            "greek_textus_receptus" => self.greek_textus_receptus.as_deref(),
            other => unreachable!("brainfuel::EDITIONS names no '{other}' field -- internal caller bug, not real data"),
        }
    }

    /// One edition's own outcome at this verse position -- see this
    /// module's own doc comment for the full `absent`/`src`/testament-gap
    /// catalog this classifies.
    fn outcome(&self, edition: &str) -> RenderingOutcome<'_> {
        let Some(text) = self.field(edition) else {
            return RenderingOutcome::NotApplicable;
        };
        let absent = self.refs.get(edition).map(|r| r.absent).unwrap_or(false);
        if absent {
            return RenderingOutcome::Absent;
        }
        if text.is_empty() {
            // Never observed in the real vendored data (swept: 0 occurrences,
            // asserted by this batch's own real-data test) -- an honest
            // fallback, not a shape this batch's data needs. Refusing to
            // import an unmarked empty string keeps the "absence is data,
            // never an empty string" law even if this ever DID happen.
            return RenderingOutcome::Anomaly;
        }
        RenderingOutcome::Present(text)
    }
}

enum RenderingOutcome<'a> {
    /// This edition's field key is absent from the JSON entirely -- the
    /// edition doesn't apply to this verse's own testament.
    NotApplicable,
    /// `refs.<edition>.absent: true` -- a real, disclosed gap.
    Absent,
    /// A real rendering, ready to import verbatim.
    Present(&'a str),
    /// Text present, non-`absent`-marked, but empty -- never observed;
    /// disclosed rather than silently imported as content.
    Anomaly,
}

/// One (book, chapter, verse) row, fully resolved -- what the graph adapter
/// consumes. `renderings` carries only `Present` outcomes (edition id,
/// verbatim text); `king_james` is carried SEPARATELY and ONLY for the
/// cross-check (`kjv_cross_check` below) -- never merged into
/// `renderings`, never imported into the graph (module doc comment).
pub struct VerseRow {
    pub book: BookId,
    pub chapter: u16,
    pub verse: u16,
    pub king_james: Option<String>,
    pub renderings: Vec<(&'static str, String)>,
}

/// The parser's own tally (batch brief requirement 9: "per-edition ingested
/// verse counts, exact totals asserted, from the parser's own tally").
#[derive(Debug, Clone, Default)]
pub struct ParseStats {
    pub ot_chapters: usize,
    pub nt_chapters: usize,
    /// Present-outcome count per edition -- the number of TextUnit
    /// positions this edition actually gains a rendering at.
    pub per_edition_present: BTreeMap<&'static str, usize>,
    /// `refs.<edition>.absent` count per edition -- disclosed gaps.
    pub per_edition_absent: BTreeMap<&'static str, usize>,
    /// `refs.<edition>.src` count per edition -- versification provenance
    /// notes, disclosed, never imported (module doc comment).
    pub per_edition_src_notes: BTreeMap<&'static str, usize>,
    /// `RenderingOutcome::Anomaly` count -- must be 0 over real data
    /// (asserted by this batch's own real-data test); never silently
    /// imported if it ever isn't.
    pub anomalies: usize,
}

#[derive(Default)]
pub struct BrainFuelCorpus {
    pub rows: Vec<VerseRow>,
    pub stats: ParseStats,
}

fn parse_chapter_file(json: &str, book_codes: &HashMap<String, BookId>, stats: &mut ParseStats, rows: &mut Vec<VerseRow>) -> Result<()> {
    let raw: RawChapterFile = serde_json::from_str(json).context("brain-fuel chapter JSON is not valid")?;
    let book = *book_codes
        .get(&raw.book_id)
        .with_context(|| format!("brain-fuel chapter file names book_id '{}', not present in books.json's own ot/nt rows", raw.book_id))?;

    for v in &raw.verses {
        let mut renderings = Vec::with_capacity(EDITIONS.len());
        for &edition in EDITIONS {
            match v.outcome(edition) {
                RenderingOutcome::NotApplicable => {}
                RenderingOutcome::Absent => {
                    *stats.per_edition_absent.entry(edition).or_insert(0) += 1;
                }
                RenderingOutcome::Present(text) => {
                    renderings.push((edition, text.to_string()));
                    *stats.per_edition_present.entry(edition).or_insert(0) += 1;
                }
                RenderingOutcome::Anomaly => {
                    stats.anomalies += 1;
                }
            }
            if v.refs.get(edition).and_then(|r| r.src.as_ref()).is_some() {
                *stats.per_edition_src_notes.entry(edition).or_insert(0) += 1;
            }
        }
        rows.push(VerseRow { book, chapter: raw.chapter, verse: v.verse, king_james: v.king_james.clone(), renderings });
    }
    Ok(())
}

/// Sorted, numbered `NNN.json` chapter file paths directly under `dir`
/// (one book's own chapter directory, e.g. `.../ot/GEN/`) -- lexicographic
/// `read_dir` order already sorts correctly since every real file name is
/// zero-padded to three digits (the source repo's own stated convention,
/// module doc comment), but this sorts explicitly rather than depending on
/// the OS's own directory-listing order.
fn chapter_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading directory {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    paths.sort();
    Ok(paths)
}

/// Reads the whole vendored corpus: `root/data/books.json` (the code map)
/// plus every `root/ot/*/*.json` and `root/nt/*/*.json` chapter file --
/// `root` is `data/raw/brain-fuel-bible` (see `data/raw/README.md`). The
/// one disclosed filesystem-touching function in this module (mirrors
/// `atlas_etl::catechism_map::build_questions_from_mapping`'s own precedent
/// for a multi-file curated/vendored source -- see that function's own doc
/// comment for the "atlas-etl does no networking, but a simple directory
/// walk over already-vendored files is the one standing exception" note).
/// Fails loudly on any read/parse/book-resolution error, naming the file.
pub fn read_all(root: &Path) -> Result<BrainFuelCorpus> {
    let books_json = std::fs::read_to_string(root.join("data/books.json")).with_context(|| format!("reading {}", root.join("data/books.json").display()))?;
    let book_codes = book_code_map(&books_json)?;

    let mut stats = ParseStats::default();
    let mut rows = Vec::new();

    for testament in ["ot", "nt"] {
        let testament_dir = root.join(testament);
        let mut book_dirs: Vec<std::path::PathBuf> =
            std::fs::read_dir(&testament_dir).with_context(|| format!("reading directory {}", testament_dir.display()))?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
        book_dirs.sort();
        for book_dir in book_dirs {
            for chapter_path in chapter_files(&book_dir)? {
                let json = std::fs::read_to_string(&chapter_path).with_context(|| format!("reading {}", chapter_path.display()))?;
                parse_chapter_file(&json, &book_codes, &mut stats, &mut rows).with_context(|| format!("parsing {}", chapter_path.display()))?;
                match testament {
                    "ot" => stats.ot_chapters += 1,
                    "nt" => stats.nt_chapters += 1,
                    _ => unreachable!(),
                }
            }
        }
    }

    Ok(BrainFuelCorpus { rows, stats })
}

// ---------------------------------------------------------------------
// The owner-ordered KJV column cross-check (batch brief requirement 5).
// ---------------------------------------------------------------------

/// One disclosed mismatch: dot-ref, our canonical text, brain-fuel's own
/// `king_james` column text at the same aligned position.
pub struct KjvMismatch {
    pub dot_ref: String,
    pub ours: String,
    pub theirs: String,
}

pub struct KjvCrossCheckReport {
    pub compared: usize,
    /// RAW byte-for-byte mismatches -- the pinned regression number (batch
    /// brief requirement 9: "KJV cross-check test, mismatch count asserted
    /// at the disclosed number"). See `brainfuel_real_data.rs`'s own test
    /// for the full categorized breakdown (whitespace, the Tetragrammaton
    /// `LORD`/`Lord` case convention, Psalm-superscription/Ps119-acrostic/
    /// epistle-subscription folding, and a small residue of genuine
    /// spelling variants) -- every single one confirmed, by manual
    /// inspection across the full raw-to-normalized funnel, to be a
    /// typographic/transcription-convention difference, NEVER a case of
    /// verse-content substitution or versification drift.
    pub raw_mismatches: usize,
    pub examples: Vec<KjvMismatch>,
}

/// Compares brain-fuel's own `king_james` column against `our_kjv_verses`
/// (this app's own canonical `"BOOK.CH.V" -> text` map, from
/// `atlas_etl::kjv::parse`) at every aligned position -- RAW byte equality,
/// no normalization (this function's own job is to report the honest,
/// literal number; a caller may categorize it further). Mismatches are
/// DISCLOSED here, never imported anywhere -- our KJV base is
/// authoritative (KJV INERRANCY DIRECTIVE; batch brief requirement 5).
/// `example_cap` bounds how many mismatches are collected into
/// `examples` (the full COUNT is always exact regardless of the cap).
pub fn kjv_cross_check(corpus: &BrainFuelCorpus, our_kjv_verses: &HashMap<String, String>, example_cap: usize) -> KjvCrossCheckReport {
    let mut compared = 0;
    let mut raw_mismatches = 0;
    let mut examples = Vec::new();
    for row in &corpus.rows {
        let Some(theirs) = &row.king_james else { continue };
        let dot_ref = format!("{}.{}.{}", row.book.code(), row.chapter, row.verse);
        let Some(ours) = our_kjv_verses.get(&dot_ref) else { continue };
        compared += 1;
        if ours != theirs {
            raw_mismatches += 1;
            if examples.len() < example_cap {
                examples.push(KjvMismatch { dot_ref, ours: ours.clone(), theirs: theirs.clone() });
            }
        }
    }
    KjvCrossCheckReport { compared, raw_mismatches, examples }
}

// ---------------------------------------------------------------------
// Batch KJV-CASE (owner ruling, "KJV-CASE-1 APPROVED... 3. yes" --
// .superpowers/sdd/2026-08-17-bible-atlas-m1/batch-kjv-case-brief.md): the
// Tetragrammaton case-restoration pass. Built BESIDE `kjv_cross_check`
// above -- the SAME per-row loop, walking the SAME already-parsed
// `corpus.rows`/`our_kjv_verses`, at the SAME dot-ref-aligned positions
// that function's own report already proved line up 31,102/31,102 -- not
// a second parser, a sibling pass over identical already-parsed data.
//
// KJV INERRANCY DIRECTIVE: restoration, never revision. The law this pass
// exists to satisfy (batch brief controller decision 2): for any position
// this pass TOUCHES, before/after must be identical under ASCII
// case-folding (a byte difference that is not purely a case difference is
// a bug, not a restoration); for any position this pass SKIPS (the folded
// texts themselves disagree -- whitespace conventions, Psalm-superscription/
// Ps119-acrostic-header folding, spelling residue -- `kjv_cross_check`'s
// own doc comment has the full catalog `kjv_column_cross_check_mismatch_
// count_is_pinned` categorizes), before/after must be BYTE-IDENTICAL: this
// pass provably never touches them.
// ---------------------------------------------------------------------

/// Transfers brain-fuel's own CASE onto `ours`, byte-for-byte, wherever
/// the two are equal under ASCII case-folding -- `None` when they are
/// NOT (a caller MUST treat `None` as "touch nothing at this position";
/// this is the case-only law's own enforcement point).
///
/// Deliberately NOT "return `theirs.to_string()` when case-fold-equal":
/// even though case-fold equality already proves `ours`/`theirs` share
/// the identical character sequence up to ASCII case, this walks `ours`'s
/// OWN bytes and flips only the ones brain-fuel's own corresponding byte
/// says should change -- "characters unchanged, case only" (batch brief
/// controller decision 1) provable byte-by-byte, not merely true by a
/// construction a reader would have to re-derive.
///
/// `str::eq_ignore_ascii_case` compares byte-for-byte and requires equal
/// length, so a `Some` result implies `ours.len() == theirs.len()` and the
/// two zipped byte streams stay aligned throughout. The output is
/// guaranteed valid UTF-8: every byte this loop CHANGES is a single-byte
/// ASCII alphabetic character on BOTH sides (a non-ASCII UTF-8
/// continuation byte, always `>= 0x80`, can never be
/// `is_ascii_uppercase`/`is_ascii_lowercase`, so this transform never
/// touches one), and every byte it leaves alone was already valid UTF-8
/// in `ours`.
pub fn restore_verse_case(ours: &str, theirs: &str) -> Option<String> {
    if !ours.eq_ignore_ascii_case(theirs) {
        return None;
    }
    let restored: Vec<u8> = ours
        .bytes()
        .zip(theirs.bytes())
        .map(|(o, t)| if t.is_ascii_uppercase() { o.to_ascii_uppercase() } else if t.is_ascii_lowercase() { o.to_ascii_lowercase() } else { o })
        .collect();
    Some(String::from_utf8(restored).expect("ASCII-case-only transform of valid UTF-8 stays valid UTF-8 (fn doc comment)"))
}

// ---------------------------------------------------------------------
// Batch KJV-CASE-2 (owner ruling, verbatim "do it (superscript fix)" --
// .superpowers/sdd/2026-08-17-bible-atlas-m1/batch-kjv-case2-brief.md):
// extends the pass above to the positions it provably and correctly
// SKIPPED because our own canonical text folds a book/Psalm superscription
// (e.g. "A Psalm of David.") into verse 1 while brain-fuel's own column
// carries the verse body alone -- so the whole-verse case-fold-equality
// gate above never fires there. PSA 110:1 is the flagship: "The LORD said
// unto my Lord" (both Tetragrammaton-referent forms, one verse).
//
// KJV INERRANCY DIRECTIVE still binds: restoration, never revision -- the
// superscription text itself IS Scripture and keeps its place, characters
// AND casing both, byte-for-byte untouched; only the ALIGNED TAIL (the
// verse body brain-fuel's own column actually carries) is ever eligible
// for case restoration, via the SAME `restore_verse_case` primitive pass 1
// already proved correct -- this extension adds an ALIGNMENT step ahead
// of that primitive, it never forks a second byte-transform mechanism.
// ---------------------------------------------------------------------

/// EXCLUSION LIST (batch KJV-CASE-2 controller decision 3): positions
/// this pass's own tail-alignment sweep (`tail_align` below) would
/// otherwise classify `OursSuffix` (superscription-class, eligible for
/// restoration), but which this batch's own real-data inspection showed
/// are NOT a genuine folded-in superscription -- brain-fuel `king_james`
/// column data artifacts of the same general kind batch KJV-CASE's own
/// report self-review catalogued (transcription quirks, never a real KJV
/// convention). Authored from what THIS run's own data showed
/// (batch-kjv-case2-brief.md controller decision 3: "the code's table is
/// authored from what the DATA shows... this run is the authority") --
/// position + one-line reason each, from real inspection of the aligned
/// prefix/tail this run produced. Checked BEFORE `tail_align` runs
/// (`classify_and_restore` below): an excluded position restores nothing
/// regardless of what the alignment check would otherwise decide, and is
/// asserted untouched by `brainfuel_real_data.rs`'s own
/// `superscription_exclusions_are_provably_untouched`.
/// Authored from a full real-data sweep of the 3,801 pass-1-skipped
/// positions (batch-kjv-case2-report.md has the full methodology): 139
/// positions tail-aligned (`OursSuffix`), a word-level diff against each
/// one's own restored tail flagged exactly these three as changing a word
/// OTHER than a Tetragrammaton-pattern one (LORD/GOD/JAH/JEHOVAH, or the
/// unrestored Adonai-class "Lord"/"God" left alone) -- the report's own
/// "~136-class" estimate reconciles exactly: 139 - 3 = 136.
pub const SUPERSCRIPTION_EXCLUSIONS: &[(&str, &str)] = &[
    (
        "PSA.70.1",
        "brain-fuel's own king_james column renders this verse's ENTIRE body in spurious ALL-CAPS (not just the Tetragrammaton word) -- the same brain-fuel transcription-artifact class batch KJV-CASE's own report already catalogued at PRO.22.1/LAM.3.1, never a genuine KJV casing convention.",
    ),
    (
        "PSA.92.1",
        "brain-fuel's own king_james column renders this verse's ENTIRE body in spurious ALL-CAPS (not just the Tetragrammaton word) -- the same brain-fuel transcription-artifact class batch KJV-CASE's own report already catalogued at PRO.22.1/LAM.3.1, never a genuine KJV casing convention.",
    ),
    (
        "ACT.9.29",
        "brain-fuel's own versification SPLITS verses 28/29 differently from ours (their v28 absorbs our v29's own first clause, 'And he spake boldly in the name of the Lord Jesus,') -- the folded-suffix match is a versification-boundary coincidence, never a folded-in superscription (Acts carries no book/Psalm-style superscriptions at all).",
    ),
];

/// One position's own tail-alignment outcome (batch KJV-CASE-2 controller
/// decision 1), computed once the whole verse is already known NOT to be
/// case-fold-equal to brain-fuel's own column (`restore_verse_case`
/// already returned `None`). ASCII-folds BOTH texts -- the IDENTICAL fold
/// `restore_verse_case`'s own `eq_ignore_ascii_case` uses, never a
/// broader Unicode fold, keeping this pass's UTF-8 safety guarantee
/// identical to pass 1's -- and checks for a SUFFIX alignment.
///
/// batch-polish1-brief.md KJVCASE2-m1: `pub`, alongside `tail_align`
/// below, as a test-support export -- `tests/brainfuel_real_data.rs`'s own
/// real-data law sweep calls this SAME fn directly now (rather than
/// maintaining its own second copy of the alignment check), the same
/// "one alignment implementation beats two" discipline `classify_and_
/// restore`'s own doc comment already establishes for `restore_verse_
/// case` between its two real call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TailAlignment {
    /// `ours`'s own folded text ends with `theirs`'s folded text -- the
    /// expected shape (a folded-in superscription prefix ahead of the
    /// verse body). Carries the untouched prefix's own BYTE length.
    OursSuffix { prefix_len: usize },
    /// `theirs`'s own folded text ends with `ours`'s -- NOT expected
    /// (controller decision 1: "Mirror-case... is NOT expected -- if
    /// found, count + disclose, restore nothing there").
    TheirsSuffix,
    /// Neither aligns -- true residue (whitespace conventions, spelling
    /// residue, etc: the non-superscription mismatch classes this batch
    /// does not touch).
    NoAlignment,
}

/// Pure, dependency-free alignment check -- no case transform happens
/// here (`classify_and_restore` below does that, via `restore_verse_case`
/// on the aligned tail only, never a bespoke second transform).
/// `is_char_boundary` guards every byte slice: real KJV/brain-fuel text is
/// ASCII at every observed superscription boundary, but this never
/// assumes it -- a non-boundary split is treated as no alignment, never a
/// panic.
pub fn tail_align(ours: &str, theirs: &str) -> TailAlignment {
    if ours.len() > theirs.len() {
        let prefix_len = ours.len() - theirs.len();
        if ours.is_char_boundary(prefix_len) && ours[prefix_len..].eq_ignore_ascii_case(theirs) {
            return TailAlignment::OursSuffix { prefix_len };
        }
    } else if theirs.len() > ours.len() {
        let prefix_len = theirs.len() - ours.len();
        if theirs.is_char_boundary(prefix_len) && theirs[prefix_len..].eq_ignore_ascii_case(ours) {
            return TailAlignment::TheirsSuffix;
        }
    }
    TailAlignment::NoAlignment
}

/// The full, richer per-position classification (batch KJV-CASE-2): pass
/// 1's own binary `restore_verse_case` `Some`/`None` outcome, extended
/// with the tail-alignment/exclusion/mirror-case buckets. ONE function,
/// shared by BOTH real call sites that need this decision
/// (`restore_kjv_case` below, and `atlas_graph::fidelity::
/// check_kjv_fidelity`'s own independent "expected"-side re-derivation) --
/// mirrors pass 1's own precedent of sharing `restore_verse_case` itself
/// between those same two call sites, so the law stays provably ONE
/// mechanism, never two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestorationOutcome {
    /// Pass-1 class: whole verse case-fold-equal. Carries the fully
    /// restored text (byte-identical to `ours` when already agreeing).
    WholeVerse(String),
    /// Pass-2 class (batch KJV-CASE-2): a folded-in superscription
    /// prefix, kept byte-identical, ahead of a case-restored tail.
    /// Carries the fully restored text (untouched prefix + restored tail).
    Superscription(String),
    /// Named in `SUPERSCRIPTION_EXCLUSIONS` -- restore nothing.
    Excluded,
    /// Mirror-case (brain-fuel longer) -- NOT expected; restore nothing.
    MirrorCase,
    /// True residue -- neither whole-verse-equal nor tail-aligned.
    Residue,
}

/// `dot_ref` is needed only for the `SUPERSCRIPTION_EXCLUSIONS` lookup --
/// every byte-level decision still flows through `restore_verse_case`
/// alone, exactly as pass 1 established.
pub fn classify_and_restore(dot_ref: &str, ours: &str, theirs: &str) -> RestorationOutcome {
    if let Some(new_text) = restore_verse_case(ours, theirs) {
        return RestorationOutcome::WholeVerse(new_text);
    }
    if SUPERSCRIPTION_EXCLUSIONS.iter().any(|(excluded_ref, _reason)| *excluded_ref == dot_ref) {
        return RestorationOutcome::Excluded;
    }
    match tail_align(ours, theirs) {
        TailAlignment::OursSuffix { prefix_len } => {
            let tail = restore_verse_case(&ours[prefix_len..], theirs).expect(
                "tail_align's own OursSuffix match guarantees the aligned tail is case-fold-equal to theirs -- restore_verse_case cannot return None here",
            );
            RestorationOutcome::Superscription(format!("{}{}", &ours[..prefix_len], tail))
        }
        TailAlignment::TheirsSuffix => RestorationOutcome::MirrorCase,
        TailAlignment::NoAlignment => RestorationOutcome::Residue,
    }
}

/// Per-class tallies (batch brief controller decision 4; buckets 3-6
/// added by batch KJV-CASE-2, controller decisions 1-3). `restored`/
/// `already_agreeing` are PASS-1's own two whole-verse buckets, byte-for-
/// byte UNCHANGED by this extension (KJV-CASE-2 never touches a position
/// pass 1 already restored or already-agreed): `restored` -- positions
/// where `restore_verse_case` produced a genuinely different string
/// (brain-fuel's own casing disagreed with ours). `already_agreeing` --
/// positions where the folded texts matched AND the bytes were ALREADY
/// identical -- no case drift to restore, the bulk of the KJV.
///
/// The remaining four fields subdivide PASS-1's own single
/// `skipped_mismatch` bucket via KJV-CASE-2's own TAIL-ALIGNMENT RULE:
/// `superscription_restored` -- `ours`'s own folded text ends with
/// `theirs`'s folded text (a folded-in superscription prefix): case
/// restored over the aligned TAIL only, the prefix kept byte-identical.
/// `excluded` -- named in `SUPERSCRIPTION_EXCLUSIONS` (a brain-fuel
/// artifact, not a genuine superscription) -- restore nothing.
/// `mirror_case_found` -- `theirs`'s own folded text ends with `ours`'s
/// (brain-fuel longer) -- NOT expected; disclosed, never restored.
/// `skipped_mismatch` -- true residue: neither whole-verse-equal nor
/// tail-aligned at all (whitespace conventions, spelling residue, etc.) --
/// KJV-CASE-2 does NOT touch this class, per its own scope.
///
/// `compared == restored + already_agreeing + superscription_restored +
/// excluded + mirror_case_found + skipped_mismatch` always (every compared
/// position falls into exactly one of the six buckets) -- and `compared`
/// itself matches `KjvCrossCheckReport.compared` exactly, over the same
/// two inputs (both walk the identical `corpus.rows`/`our_kjv_verses`
/// alignment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CaseRestorationReport {
    pub compared: usize,
    pub restored: usize,
    pub already_agreeing: usize,
    pub superscription_restored: usize,
    pub excluded: usize,
    pub mirror_case_found: usize,
    pub skipped_mismatch: usize,
}

/// The owner-ordered case-restoration pass (batch brief controller
/// decision 1; extended by batch KJV-CASE-2's own superscription-aware
/// tail alignment, controller decisions 1-3). Returns a NEW
/// `"BOOK.CH.V" -> text` map: a full clone of `our_kjv_verses` with ONLY
/// the restored positions' values replaced (whole-verse OR
/// superscription-tail class). Every other entry -- including any
/// position brain-fuel doesn't cover at all -- passes through
/// byte-identical by construction (this starts from
/// `our_kjv_verses.clone()` and only ever `.insert()`s at a position this
/// loop actually visits and restores), which is exactly the case-only
/// law's second half: a skipped/excluded/uncovered position is untouched,
/// not merely "close."
///
/// Same per-row loop shape as `kjv_cross_check` above (the batch brief's
/// own instruction: "build the restoration pass beside it"), now
/// delegating the per-position DECISION to `classify_and_restore` above
/// (the ONE mechanism this whole module's own KJV-CASE-2 section extends,
/// never a second one). Iteration order over `corpus.rows` cannot change
/// the result: every write lands at a distinct `dot_ref` key.
pub fn restore_kjv_case(corpus: &BrainFuelCorpus, our_kjv_verses: &HashMap<String, String>) -> (HashMap<String, String>, CaseRestorationReport) {
    let mut restored_verses = our_kjv_verses.clone();
    let mut report = CaseRestorationReport::default();
    for row in &corpus.rows {
        let Some(theirs) = &row.king_james else { continue };
        let dot_ref = format!("{}.{}.{}", row.book.code(), row.chapter, row.verse);
        let Some(ours) = our_kjv_verses.get(&dot_ref) else { continue };
        report.compared += 1;
        match classify_and_restore(&dot_ref, ours, theirs) {
            RestorationOutcome::WholeVerse(new_text) => {
                if &new_text == ours {
                    report.already_agreeing += 1;
                } else {
                    report.restored += 1;
                    restored_verses.insert(dot_ref, new_text);
                }
            }
            RestorationOutcome::Superscription(new_text) => {
                report.superscription_restored += 1;
                restored_verses.insert(dot_ref, new_text);
            }
            RestorationOutcome::Excluded => report.excluded += 1,
            RestorationOutcome::MirrorCase => report.mirror_case_found += 1,
            RestorationOutcome::Residue => report.skipped_mismatch += 1,
        }
    }
    (restored_verses, report)
}

/// dot-ref (`"BOOK.CH.V"`) -> brain-fuel's own `king_james` column text,
/// for every row that carries one. The one lookup `atlas_graph::fidelity`
/// needs cross-crate: its own independent re-derivation of "expected" KJV
/// text must apply the IDENTICAL `restore_verse_case` transform this pass
/// does (see that module's own doc comment for why), reached from typed
/// book-index/chapter/verse fields rather than a `BrainFuelCorpus` row --
/// `kjv_cross_check`/`restore_kjv_case` above need no such map themselves
/// (they already iterate `corpus.rows` directly, one row at a time).
pub fn king_james_by_dot_ref(corpus: &BrainFuelCorpus) -> HashMap<String, &str> {
    corpus.rows.iter().filter_map(|r| r.king_james.as_deref().map(|kjv| (format!("{}.{}.{}", r.book.code(), r.chapter, r.verse), kjv))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOKS_JSON: &str = r#"{"books":[
        {"code":"GEN","testament":"ot","kjv_name":"Genesis"},
        {"code":"1SA","testament":"ot","kjv_name":"I Samuel"},
        {"code":"JOH","testament":"nt","kjv_name":"John"},
        {"code":"REV","testament":"nt","kjv_name":"Revelation of John"},
        {"code":"TOB","testament":"apo","kjv_name":"Tobit"}
    ]}"#;

    #[test]
    fn book_code_map_resolves_old_style_names_and_skips_apocrypha() {
        let map = book_code_map(BOOKS_JSON).unwrap();
        assert_eq!(map.len(), 4, "TOB (apo) must be skipped");
        assert_eq!(map.get("GEN").unwrap().code(), "GEN");
        assert_eq!(map.get("1SA").unwrap().code(), "1SA", "brain-fuel's own 'I Samuel' must resolve via the shared normalizer");
        assert_eq!(map.get("JOH").unwrap().code(), "JHN", "brain-fuel's own 3-letter code (JOH) differs from ours (JHN) -- resolved via kjv_name, not code");
        assert_eq!(map.get("REV").unwrap().code(), "REV", "'Revelation of John' -> 'Revelation'");
        assert!(!map.contains_key("TOB"));
    }

    #[test]
    fn book_code_map_fails_loud_on_unresolvable_name() {
        let bad = r#"{"books":[{"code":"XXX","testament":"ot","kjv_name":"Not A Real Book"}]}"#;
        let err = book_code_map(bad).unwrap_err().to_string();
        assert!(err.contains("XXX"), "{err}");
    }

    fn gen_book_codes() -> HashMap<String, BookId> {
        let mut m = HashMap::new();
        m.insert("GEN".to_string(), resolve_alias("Genesis").unwrap());
        m.insert("1CH".to_string(), resolve_alias("1 Chronicles").unwrap());
        m
    }

    #[test]
    fn present_rendering_imports_verbatim_including_trailing_whitespace() {
        let json = r#"{"book_id":"GEN","chapter":1,"verses":[
            {"verse":1,"latin_vulgate":"In principio creavit Deus. ","king_james":"In the beginning God created the heaven and the earth."}
        ]}"#;
        let mut stats = ParseStats::default();
        let mut rows = Vec::new();
        parse_chapter_file(json, &gen_book_codes(), &mut stats, &mut rows).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].renderings, vec![("latin_vulgate", "In principio creavit Deus. ".to_string())], "byte-verbatim, trailing space kept");
        assert_eq!(stats.per_edition_present.get("latin_vulgate"), Some(&1));
    }

    #[test]
    fn absent_marker_produces_no_rendering_never_an_empty_string() {
        // Real shape from bible/ot/1CH/011.json verse 47 (module doc
        // comment): the text key is STILL PRESENT, holding "".
        let json = r#"{"book_id":"1CH","chapter":11,"verses":[
            {"verse":47,"latin_vulgate":"","douay_rheims":"","hebrew_masoretic":"real text","king_james":"kjv text",
             "refs":{"latin_vulgate":{"absent":true},"douay_rheims":{"absent":true}}}
        ]}"#;
        let mut stats = ParseStats::default();
        let mut rows = Vec::new();
        parse_chapter_file(json, &gen_book_codes(), &mut stats, &mut rows).unwrap();
        assert_eq!(rows.len(), 1);
        let editions: Vec<&str> = rows[0].renderings.iter().map(|(e, _)| *e).collect();
        assert!(!editions.contains(&"latin_vulgate"), "absent-marked -- must carry NO rendering, not an empty one: {editions:?}");
        assert!(!editions.contains(&"douay_rheims"), "{editions:?}");
        assert!(editions.contains(&"hebrew_masoretic"), "an unmarked edition on the SAME verse must still import normally");
        assert_eq!(stats.per_edition_absent.get("latin_vulgate"), Some(&1));
        assert_eq!(stats.per_edition_absent.get("douay_rheims"), Some(&1));
        assert_eq!(stats.anomalies, 0);
    }

    #[test]
    fn edition_not_applicable_to_testament_is_silently_absent_not_flagged() {
        // greek_textus_receptus never appears as a KEY at all on an OT
        // verse -- NotApplicable, distinct from an explicit absent marker,
        // but with the identical "no rendering" outcome.
        let json = r#"{"book_id":"GEN","chapter":1,"verses":[
            {"verse":1,"latin_vulgate":"txt","king_james":"kjv"}
        ]}"#;
        let mut stats = ParseStats::default();
        let mut rows = Vec::new();
        parse_chapter_file(json, &gen_book_codes(), &mut stats, &mut rows).unwrap();
        let editions: Vec<&str> = rows[0].renderings.iter().map(|(e, _)| *e).collect();
        assert_eq!(editions, vec!["latin_vulgate"]);
        assert_eq!(stats.per_edition_absent.get("greek_textus_receptus"), None, "NotApplicable must never be counted as an absent MARKER");
    }

    #[test]
    fn refs_src_is_counted_but_never_imported() {
        let json = r#"{"book_id":"GEN","chapter":3,"verses":[
            {"verse":1,"hebrew_masoretic":"txt","king_james":"kjv","refs":{"hebrew_masoretic":{"src":"3:2"}}}
        ]}"#;
        let mut stats = ParseStats::default();
        let mut rows = Vec::new();
        parse_chapter_file(json, &gen_book_codes(), &mut stats, &mut rows).unwrap();
        assert_eq!(rows[0].renderings, vec![("hebrew_masoretic", "txt".to_string())], "src is provenance-only -- the rendering imports normally");
        assert_eq!(stats.per_edition_src_notes.get("hebrew_masoretic"), Some(&1));
    }

    #[test]
    fn king_james_column_is_carried_for_cross_check_only() {
        let json = r#"{"book_id":"GEN","chapter":1,"verses":[{"verse":1,"king_james":"their kjv text"}]}"#;
        let mut stats = ParseStats::default();
        let mut rows = Vec::new();
        parse_chapter_file(json, &gen_book_codes(), &mut stats, &mut rows).unwrap();
        assert_eq!(rows[0].king_james.as_deref(), Some("their kjv text"));
        assert!(rows[0].renderings.is_empty(), "king_james must NEVER land in renderings -- it is not one of the six ingested editions");
    }

    #[test]
    fn kjv_cross_check_counts_raw_mismatches_and_caps_examples() {
        let mut rows = Vec::new();
        for (v, _ours, theirs) in [(1, "In the beginning", "In the beginning"), (2, "And the earth was", "And the earth WAS")] {
            rows.push(VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 1, verse: v, king_james: Some(theirs.to_string()), renderings: vec![] });
        }
        let corpus = BrainFuelCorpus { rows, stats: ParseStats::default() };
        let mut ours_map = HashMap::new();
        ours_map.insert("GEN.1.1".to_string(), "In the beginning".to_string());
        ours_map.insert("GEN.1.2".to_string(), "And the earth was".to_string());
        let report = kjv_cross_check(&corpus, &ours_map, 10);
        assert_eq!(report.compared, 2);
        assert_eq!(report.raw_mismatches, 1);
        assert_eq!(report.examples.len(), 1);
        assert_eq!(report.examples[0].dot_ref, "GEN.1.2");
    }

    #[test]
    fn kjv_cross_check_skips_positions_missing_from_either_side() {
        let corpus = BrainFuelCorpus {
            rows: vec![VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 1, verse: 99, king_james: Some("x".into()), renderings: vec![] }],
            stats: ParseStats::default(),
        };
        let ours_map: HashMap<String, String> = HashMap::new(); // GEN.1.99 not present on our side
        let report = kjv_cross_check(&corpus, &ours_map, 10);
        assert_eq!(report.compared, 0, "a position absent from OUR side is skipped, never counted as a mismatch");
        assert_eq!(report.raw_mismatches, 0);
    }

    // -------------------------------------------------------------------
    // Batch KJV-CASE: restore_verse_case / restore_kjv_case.
    // -------------------------------------------------------------------

    #[test]
    fn restore_verse_case_none_when_lengths_differ() {
        // The real PSA 110:1 shape (this batch's own report): our source
        // folds the Psalm superscription into verse 1 ("A Psalm of
        // David. The Lord said..."), brain-fuel's king_james column does
        // not -- longer string, never case-fold-equal, must be a hard
        // `None`, not a best-effort partial match.
        assert_eq!(restore_verse_case("A Psalm of David. The Lord said unto my Lord.", "The LORD said unto my Lord."), None);
    }

    #[test]
    fn restore_verse_case_none_when_a_non_case_byte_differs() {
        assert_eq!(restore_verse_case("hello world", "hello there"), None, "same length, but a real content difference, not case");
        assert_eq!(restore_verse_case("don't stop", "dont stop"), None, "punctuation difference is not a case difference");
    }

    #[test]
    fn restore_verse_case_is_a_true_no_op_when_already_byte_identical() {
        assert_eq!(restore_verse_case("In the beginning", "In the beginning").as_deref(), Some("In the beginning"));
    }

    #[test]
    fn restore_verse_case_transfers_theirs_case_pattern_onto_ours_own_characters() {
        // Ours starts capitalized ("The"), theirs does not; ours' own
        // "lord" is lowercase, theirs' is upper -- the output must follow
        // BRAIN-FUEL's own case at every position, not just fill gaps.
        assert_eq!(restore_verse_case("The lord said", "the LORD said").as_deref(), Some("the LORD said"));
    }

    #[test]
    fn restore_kjv_case_buckets_every_compared_position_exactly_once() {
        let rows = vec![
            // already_agreeing: byte-identical already.
            VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 1, verse: 1, king_james: Some("In the beginning".into()), renderings: vec![] },
            // restored: a genuine LORD/Lord case difference.
            VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 2, verse: 4, king_james: Some("the LORD God made".into()), renderings: vec![] },
            // skipped_mismatch: folded texts disagree (extra word).
            VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 3, verse: 1, king_james: Some("Now the serpent was subtil indeed".into()), renderings: vec![] },
            // not compared: absent from our own side.
            VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 4, verse: 1, king_james: Some("x".into()), renderings: vec![] },
        ];
        let corpus = BrainFuelCorpus { rows, stats: ParseStats::default() };
        let mut ours = HashMap::new();
        ours.insert("GEN.1.1".to_string(), "In the beginning".to_string());
        ours.insert("GEN.2.4".to_string(), "the Lord God made".to_string());
        ours.insert("GEN.3.1".to_string(), "Now the serpent was subtil".to_string());
        // GEN.4.1 deliberately absent from `ours`.

        let (restored, report) = restore_kjv_case(&corpus, &ours);

        assert_eq!(report.compared, 3, "GEN.4.1 (absent from our side) is not compared");
        assert_eq!(report.already_agreeing, 1);
        assert_eq!(report.restored, 1);
        assert_eq!(report.skipped_mismatch, 1, "GEN.3.1's extra word 'indeed' is a PREFIX-side difference on theirs, not a suffix alignment on ours -- true residue");
        assert_eq!(report.superscription_restored, 0);
        assert_eq!(report.excluded, 0);
        assert_eq!(report.mirror_case_found, 0);
        assert_eq!(
            report.compared,
            report.restored + report.already_agreeing + report.superscription_restored + report.excluded + report.mirror_case_found + report.skipped_mismatch,
            "every compared position falls into exactly one of the six buckets"
        );

        assert_eq!(restored.get("GEN.1.1").map(String::as_str), Some("In the beginning"), "already-agreeing position is untouched");
        assert_eq!(restored.get("GEN.2.4").map(String::as_str), Some("the LORD God made"), "case genuinely restored");
        assert_eq!(restored.get("GEN.3.1").map(String::as_str), Some("Now the serpent was subtil"), "skipped position is BYTE-IDENTICAL to before -- never touched");
        assert_eq!(restored.len(), ours.len(), "no keys added or removed, only values at restored positions");
    }

    // -------------------------------------------------------------------
    // Batch KJV-CASE-2: tail_align / classify_and_restore / the
    // superscription-aware extension of restore_kjv_case.
    // -------------------------------------------------------------------

    #[test]
    fn classify_and_restore_restores_a_folded_in_superscription_tail_only() {
        // The real PSA 110:1 shape: our own text folds "A Psalm of
        // David. " into verse 1 ahead of the body; brain-fuel's column
        // carries the body alone, cased correctly. The prefix must come
        // through BYTE-IDENTICAL (including its own "Lord" -- Adonai, not
        // Tetragrammaton, correctly lowercase-l already) while the tail's
        // "lord" (Tetragrammaton) gets promoted to "LORD".
        let ours = "A Psalm of David. The lord said unto my Lord.";
        let theirs = "The LORD said unto my Lord.";
        match classify_and_restore("PSA.110.1", ours, theirs) {
            RestorationOutcome::Superscription(text) => {
                assert_eq!(text, "A Psalm of David. The LORD said unto my Lord.");
            }
            other => panic!("expected Superscription, got {other:?}"),
        }
    }

    #[test]
    fn classify_and_restore_leaves_the_prefix_byte_identical_even_when_it_itself_contains_mixed_case() {
        // The prefix (superscription) must never be touched, even where
        // it contains letters that coincidentally look Tetragrammaton-ish
        // -- only the ALIGNED TAIL is ever eligible.
        let ours = "To the chief Musician, A Psalm of david. the LORD reigneth.";
        let theirs = "The LORD reigneth.";
        match classify_and_restore("PSA.99.1", ours, theirs) {
            RestorationOutcome::Superscription(text) => {
                assert_eq!(text, "To the chief Musician, A Psalm of david. The LORD reigneth.", "prefix bytes ('david.', lowercase d) survive untouched; only the tail's case follows theirs");
                assert!(text.starts_with("To the chief Musician, A Psalm of david. "), "prefix region byte-identical before/after");
            }
            other => panic!("expected Superscription, got {other:?}"),
        }
    }

    #[test]
    fn classify_and_restore_finds_mirror_case_and_restores_nothing() {
        // brain-fuel's own column LONGER than ours, folded-suffix-equal --
        // the NOT-expected mirror shape (controller decision 1). Must be
        // disclosed as MirrorCase, never restored.
        let ours = "The LORD reigneth.";
        let theirs = "Unto the end, A Psalm of David. The LORD reigneth.";
        assert_eq!(classify_and_restore("PSA.X.1", ours, theirs), RestorationOutcome::MirrorCase);
    }

    #[test]
    fn classify_and_restore_finds_no_alignment_when_neither_side_is_a_folded_suffix() {
        let ours = "Now the serpent was subtil";
        let theirs = "Now the serpent was subtil indeed";
        assert_eq!(classify_and_restore("GEN.3.1", ours, theirs), RestorationOutcome::Residue);
    }

    #[test]
    fn classify_and_restore_honors_the_exclusion_table_over_an_otherwise_valid_alignment() {
        // A position that WOULD tail-align (ours' folded text ends with
        // theirs') but is named in SUPERSCRIPTION_EXCLUSIONS must restore
        // nothing regardless -- the exclusion check runs BEFORE tail_align.
        // Uses PSA.70.1, one of the three REAL excluded positions
        // (brainfuel_real_data.rs's own real-data test proves the actual
        // corpus shape; this unit test proves the MECHANISM in isolation
        // with a synthetic fixture standing in for the real ALL-CAPS text).
        assert!(SUPERSCRIPTION_EXCLUSIONS.iter().any(|(d, _)| *d == "PSA.70.1"), "this test targets a real exclusion-table entry -- update if PSA.70.1 is ever removed from the table");
        let ours = "To the chief Musician, A Psalm of David, to bring to remembrance. Make haste, O God, to deliver me.";
        let theirs = "MAKE HASTE, O GOD, TO DELIVER ME.";
        // Sanity: without the exclusion, this WOULD tail-align (folded
        // suffix match holds regardless of the all-caps artifact).
        assert!(matches!(classify_and_restore("PSA.NOT.EXCLUDED", ours, theirs), RestorationOutcome::Superscription(_)));
        assert_eq!(classify_and_restore("PSA.70.1", ours, theirs), RestorationOutcome::Excluded);
    }

    #[test]
    fn restore_kjv_case_wires_the_superscription_class_through_the_whole_corpus_pass() {
        let rows = vec![VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 1, verse: 1, king_james: Some("The LORD reigneth.".into()), renderings: vec![] }];
        let corpus = BrainFuelCorpus { rows, stats: ParseStats::default() };
        let mut ours = HashMap::new();
        ours.insert("GEN.1.1".to_string(), "A Psalm of David. The lord reigneth.".to_string());

        let (restored, report) = restore_kjv_case(&corpus, &ours);

        assert_eq!(report.compared, 1);
        assert_eq!(report.superscription_restored, 1);
        assert_eq!(report.restored, 0, "the whole-verse bucket must not also count this -- it is a distinct bucket");
        assert_eq!(report.compared, report.restored + report.already_agreeing + report.superscription_restored + report.excluded + report.mirror_case_found + report.skipped_mismatch);
        assert_eq!(restored.get("GEN.1.1").map(String::as_str), Some("A Psalm of David. The LORD reigneth."));
    }

    #[test]
    fn king_james_by_dot_ref_maps_every_row_that_carries_a_column() {
        let corpus = BrainFuelCorpus {
            rows: vec![
                VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 1, verse: 1, king_james: Some("kjv text".into()), renderings: vec![] },
                VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 1, verse: 2, king_james: None, renderings: vec![] },
            ],
            stats: ParseStats::default(),
        };
        let map = king_james_by_dot_ref(&corpus);
        assert_eq!(map.get("GEN.1.1").copied(), Some("kjv text"));
        assert!(!map.contains_key("GEN.1.2"), "a row with no king_james column contributes no entry");
    }
}
