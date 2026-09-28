//! Parser for the vendored brain-fuel editions: ONE parser, edition-parameterized. Its verse numbers are
//! ALREADY KJV skeleton positions, and an edition's field key is present on every verse of a chapter file or on
//! none of them, a testament-level fact. An `absent`-marked verse still carries its key, holding an EMPTY STRING.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{bail, Context, Result};
use atlas_core::canon::resolve_alias;
use atlas_core::refs::BookId;
use serde::Deserialize;

use crate::kjv::normalize_book_name;

/// The editions ingested as renderings. Each string is BOTH the exact JSON field name and the translation slug
/// the graph adapter stamps onto the layer: one vocabulary, not two. The KJV base and the apocrypha edition are
/// deliberately absent.
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

/// The brain-fuel book codes are not this app's, so each row's own `kjv_name` -- which uses the identical
/// old-style convention the KJV source does -- is resolved through the same normalizer, and the apocrypha rows
/// are skipped. Fails loudly naming every unresolved row: a gap would drop that book from every edition.
pub(crate) fn book_code_map(books_json: &str) -> Result<HashMap<String, BookId>> {
    let manifest: RawBooksManifest = serde_json::from_str(books_json).context("brain-fuel data/books.json is not valid JSON")?;
    let mut map = HashMap::new();
    let mut unresolved: Vec<String> = Vec::new();
    for b in &manifest.books {
        if b.testament != "ot" && b.testament != "nt" {
            continue;
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

    fn outcome(&self, edition: &str) -> RenderingOutcome<'_> {
        let Some(text) = self.field(edition) else {
            return RenderingOutcome::NotApplicable;
        };
        let absent = self.refs.get(edition).map(|r| r.absent).unwrap_or(false);
        if absent {
            return RenderingOutcome::Absent;
        }
        if text.is_empty() {
            // Never observed in the real data. Refusing to import an unmarked empty string keeps "absence is
            // data, never an empty string" true even if it ever happened.
            return RenderingOutcome::Anomaly;
        }
        RenderingOutcome::Present(text)
    }
}

enum RenderingOutcome<'a> {
    /// The edition's field key is absent entirely: it does not apply to this verse's own testament.
    NotApplicable,
    /// `refs.<edition>.absent: true` -- a real, disclosed gap.
    Absent,
    Present(&'a str),
    /// Text present and not `absent`-marked, but empty: disclosed rather than imported as content.
    Anomaly,
}

/// One fully resolved row. `renderings` carries only the present outcomes, and the KJV column is carried
/// SEPARATELY and only for the cross-check -- never merged in, never imported into the graph.
pub struct VerseRow {
    pub book: BookId,
    pub chapter: u16,
    pub verse: u16,
    pub king_james: Option<String>,
    pub renderings: Vec<(&'static str, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct ParseStats {
    pub ot_chapters: usize,
    pub nt_chapters: usize,
    /// Present-outcome count per edition: the positions this edition actually gains a rendering at.
    pub per_edition_present: BTreeMap<&'static str, usize>,
    /// `refs.<edition>.absent` count per edition -- disclosed gaps.
    pub per_edition_absent: BTreeMap<&'static str, usize>,
    /// Versification provenance notes per edition: disclosed, never imported.
    pub per_edition_src_notes: BTreeMap<&'static str, usize>,
    /// Must be 0 over the real data, and never silently imported if it ever is not.
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

/// Sorted, numbered chapter-file paths under one book's directory. Lexicographic order already sorts correctly,
/// since every real name is zero-padded, but this sorts explicitly rather than trusting the OS listing.
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

/// Reads the whole vendored corpus: the one filesystem-touching function here, failing loudly on any read,
/// parse or book-resolution error, naming the file.
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

/// One disclosed mismatch: the dot-ref, our canonical text, and the vendored KJV column at the same position.
pub struct KjvMismatch {
    pub dot_ref: String,
    pub ours: String,
    pub theirs: String,
}

pub struct KjvCrossCheckReport {
    pub compared: usize,
    /// RAW byte-for-byte mismatches, the pinned regression number. Every one was confirmed a typographic or
    /// transcription-convention difference, never verse-content substitution or versification drift.
    pub raw_mismatches: usize,
    pub examples: Vec<KjvMismatch>,
}

/// Compares the vendored KJV column against our own canonical text at every aligned position, by RAW byte
/// equality with no normalization: this reports the honest literal number and a caller may categorize further.
/// Mismatches are DISCLOSED, never imported -- our base is authoritative -- and `example_cap` bounds only the
/// examples, never the count.
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

// The case-only law: at any position this pass TOUCHES, before and after must be identical under ASCII
// case-folding, and at any position it SKIPS they must be byte-identical.

/// Transfers the vendored CASE onto `ours` wherever the two are equal under ASCII case-folding, and `None` where
/// they are not -- a caller MUST treat `None` as "touch nothing here". It walks `ours`'s own bytes and flips only
/// the ones the other side disagrees on, so "characters unchanged, case only" is provable byte by byte. Every
/// byte it changes is single-byte ASCII on both sides, so the result is always valid UTF-8.
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

// Our own text folds a book or Psalm superscription into verse 1 where the vendored column carries the verse
// body alone, so the whole-verse gate above never fires there. The superscription itself is Scripture and stays
// byte-for-byte untouched; only the ALIGNED TAIL is ever eligible, through the same primitive.

/// Positions the tail-alignment sweep would classify as superscription but which real-data inspection showed are
/// vendored-column artifacts rather than a genuine folded-in superscription, each with its reason. Checked BEFORE
/// the alignment, so an excluded position restores nothing whatever the alignment would decide.
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

/// One position's tail-alignment outcome, computed once the whole verse is known NOT to be case-fold-equal. It
/// folds BOTH texts with the IDENTICAL ASCII fold, never a broader Unicode one, keeping the UTF-8 guarantee the
/// primitive has. `pub` so the real-data sweep calls this same check rather than keeping a second copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TailAlignment {
    /// `ours`'s folded text ends with the other's -- the expected shape -- carrying the untouched prefix's own
    /// BYTE length.
    OursSuffix { prefix_len: usize },
    /// The other's folded text ends with `ours` -- NOT expected: count and disclose, restore nothing.
    TheirsSuffix,
    /// Neither aligns: true residue, the whitespace and spelling classes this pass does not touch.
    NoAlignment,
}

/// A pure alignment check; no case transform happens here. `is_char_boundary` guards every byte slice: the real
/// text is ASCII at every observed boundary, but a non-boundary split is treated as no alignment, never a panic.
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

/// The per-position classification: ONE function, shared by both real call sites that need this decision, so the
/// law stays provably one mechanism rather than two.
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

/// `dot_ref` is needed only for the exclusion lookup: every byte-level decision still flows through the
/// case-transfer primitive alone.
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

/// Per-class tallies. The two whole-verse buckets are the positions where the case genuinely differed and where
/// the bytes already agreed; the other four subdivide the rest by the tail-alignment rule -- a restored
/// superscription tail, an excluded position, the unexpected mirror case, and true residue. Every compared
/// position falls into exactly one bucket, so `compared` is always their sum.
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

/// Returns a full clone of the input map with ONLY the restored positions replaced, so a skipped, excluded or
/// uncovered position passes through byte-identical by construction -- the second half of the case-only law.
/// Iteration order cannot change the result: every write lands at a distinct key.
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

/// dot-ref -> the vendored KJV column text, for every row that carries one: the one lookup the fidelity law needs
/// cross-crate, reached from typed book/chapter/verse fields rather than from a corpus row.
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
        let ours_map: HashMap<String, String> = HashMap::new();
        let report = kjv_cross_check(&corpus, &ours_map, 10);
        assert_eq!(report.compared, 0, "a position absent from OUR side is skipped, never counted as a mismatch");
        assert_eq!(report.raw_mismatches, 0);
    }

    #[test]
    fn restore_verse_case_none_when_lengths_differ() {
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
        assert_eq!(restore_verse_case("The lord said", "the LORD said").as_deref(), Some("the LORD said"));
    }

    #[test]
    fn restore_kjv_case_buckets_every_compared_position_exactly_once() {
        let rows = vec![
            VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 1, verse: 1, king_james: Some("In the beginning".into()), renderings: vec![] },
            VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 2, verse: 4, king_james: Some("the LORD God made".into()), renderings: vec![] },
            VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 3, verse: 1, king_james: Some("Now the serpent was subtil indeed".into()), renderings: vec![] },
            VerseRow { book: resolve_alias("Genesis").unwrap(), chapter: 4, verse: 1, king_james: Some("x".into()), renderings: vec![] },
        ];
        let corpus = BrainFuelCorpus { rows, stats: ParseStats::default() };
        let mut ours = HashMap::new();
        ours.insert("GEN.1.1".to_string(), "In the beginning".to_string());
        ours.insert("GEN.2.4".to_string(), "the Lord God made".to_string());
        ours.insert("GEN.3.1".to_string(), "Now the serpent was subtil".to_string());

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

    #[test]
    fn classify_and_restore_restores_a_folded_in_superscription_tail_only() {
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
        assert!(SUPERSCRIPTION_EXCLUSIONS.iter().any(|(d, _)| *d == "PSA.70.1"), "this test targets a real exclusion-table entry -- update if PSA.70.1 is ever removed from the table");
        let ours = "To the chief Musician, A Psalm of David, to bring to remembrance. Make haste, O God, to deliver me.";
        let theirs = "MAKE HASTE, O GOD, TO DELIVER ME.";
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
