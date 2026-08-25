//! Batch KRETZ-1: Kretzmann's Popular Commentary of the Bible parser --
//! reads the vendored `data/raw/kretzmann/{slug}/{chapter}.html` pages
//! (kretzmanncommentary.org, see that directory's own README section) into
//! verse-anchored `KretzUnit`s, per the owner-ruled ANNOTATION shape:
//! "a comprehensive commentary without the verses interleaved into it, and
//! it's indexed so that each verse mapped bit of commentary is mapped to
//! the appropriate verse in our graph."
//!
//! TWO REAL PAGE TEMPLATES (discovered fetching the real corpus, not
//! assumed up front -- verified over ALL 1,189 real pages, an exact,
//! disjoint split): the source's own digital edition renders the Old
//! Testament (929 chapters) and New Testament (260 chapters) with two
//! DIFFERENT HTML shapes for the identical underlying idea (KJV lemma
//! quoted, then Kretzmann's own prose discussing it):
//!
//! - **Type A ("interleaved lemma"), the OT shape**: `<strong><sup>N</sup>
//!   LEMMA TEXT</strong> commentary prose <strong>NEXT LEMMA</strong>
//!   commentary prose ...` -- the bold KJV lemma is SUB-VERSE granular
//!   (Kretzmann splits a verse at a comma/semicolon and comments between
//!   the pieces; GEN 1:2's own three-fragment split is the canonical
//!   example), each fragment optionally opening with its own `<sup>N</sup>`
//!   verse-number marker (absent = the SAME verse continues, or -- only at
//!   a chapter's very first lemma, before ANY numbered marker has appeared
//!   -- an unnumbered Psalm-superscription-class lemma that folds into the
//!   FIRST numbered verse that follows it, matching this app's own
//!   canonical layer's fold convention, see `fold_leading_unnumbered_lemma`
//!   below).
//! - **Type B ("block quote + flowing commentary"), the NT shape**: `<p
//!   class="bible"><sup id="vN">N</sup>FULL VERSE TEXT<sup id="vM">M</sup>
//!   FULL VERSE TEXT...</p><p>commentary prose discussing the whole
//!   span</p>` -- the KJV text is quoted WHOLE (not sub-verse-fragmented)
//!   for every verse a pericope's own discussion covers, in one block,
//!   followed by one or more plain paragraphs of flowing commentary. One
//!   `KretzUnit` per block (range = that block's own min..max verse), not
//!   one per verse -- the SAME "a unit's own comments-on target is a
//!   RANGE" shape `Attests`/`Fulfills`/`Typology` already use elsewhere in
//!   this graph for a multi-verse span, never verse-duplicated commentary.
//!
//! Both shapes lower into the SAME internal model (`KretzUnit` + the
//! excised `ExcisedFragment`s KRETZ-ACCEPT-1 checks) via one unified
//! document-order walk (`parse_chapter`) over four recognized markers --
//! `<h3>`/`<h4>` (heading), a `<strong>...</strong>` span (Type A lemma),
//! a `<p class="bible">...</p>` block (Type B quote) -- plus the plain text
//! between them (commentary prose). Nothing here branches on "which
//! template is this page" up front; the SAME per-marker handling runs
//! whichever markers a given page actually contains (defensive against a
//! future mixed page, never observed in the real corpus but never assumed
//! impossible either).
//!
//! LEMMA-EXCISION (owner-ruled, binding): the bold/quoted KJV text is the
//! parser's join key and is EXCISED -- `KretzUnit.text` carries Kretzmann's
//! OWN prose only, never a byte of the quoted KJV lemma. The excised text
//! survives ONLY as `ExcisedFragment`s, consumed exclusively by
//! `check_conservation` (KRETZ-ACCEPT-1) and never stored on the graph.
//!
//! FOOTNOTES (verified structure: a trailing `<section data-footnotes>`
//! block, GitHub-Flavored-Markdown-style, `<sup><a href="#user-content-
//! fnref-N" data-footnote-ref>N</a></sup>` inline reference markers): kept
//! VERBATIM IN PLACE (decision 2) -- `strip_footnotes` replaces each inline
//! reference marker with a private-use sentinel carrying the footnote's own
//! full text, resolved to the disclosed `" [Footnote N: TEXT]"` form when
//! the marker lands in stored PROSE, or silently excised (counted as an
//! anomaly, never stored) on the rare/unobserved case it lands inside an
//! excised LEMMA span instead (a footnote is never genuine KJV content, so
//! folding its text into the conservation check's own comparison target
//! would be a parser bug, not a real deviation).
//!
//! CROSS-CHECKED AGAINST THE REAL, FULL, COMMITTED CORPUS before this
//! module's own scanning choices were finalized (never assumed): `<strong>`
//! is bare (zero attributed occurrences, all 929 Type-A pages); `<h3>`/
//! `<h4>` always carry an `id` attribute (zero bare occurrences); `<p
//! class="bible"` is the one, unvarying class spelling (1,939 occurrences
//! across all 260 Type-B pages); no `<h5>`/`<h6>` and no nested
//! `<strong><strong>` anywhere in the real corpus.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

// -----------------------------------------------------------------------
// Book manifest -- SAME 66-book canonical order/index as
// `atlas_core::canon::BOOKS` (verified 2026-08-25 against
// kretzmanncommentary.org/bible's own book/chapter listing: both lists'
// chapter counts match position-for-position, Genesis(50)..Revelation(22),
// summing to exactly 1,189 -- the standard KJV chapter total). `book_index`
// here IS that same 0-based global index, so this module never needs an
// `atlas_core` dependency (mirrors `concord.rs`'s own self-contained
// `DOCUMENTS` table -- a pure parser, no cross-crate canon coupling).
// -----------------------------------------------------------------------

pub struct KretzmannBookSpec {
    pub book_index: u8,
    pub slug: &'static str,
    pub chapters: u16,
}

pub const BOOKS: &[KretzmannBookSpec] = &[
    KretzmannBookSpec { book_index: 0, slug: "genesis", chapters: 50 },
    KretzmannBookSpec { book_index: 1, slug: "exodus", chapters: 40 },
    KretzmannBookSpec { book_index: 2, slug: "leviticus", chapters: 27 },
    KretzmannBookSpec { book_index: 3, slug: "numbers", chapters: 36 },
    KretzmannBookSpec { book_index: 4, slug: "deuteronomy", chapters: 34 },
    KretzmannBookSpec { book_index: 5, slug: "joshua", chapters: 24 },
    KretzmannBookSpec { book_index: 6, slug: "judges", chapters: 21 },
    KretzmannBookSpec { book_index: 7, slug: "ruth", chapters: 4 },
    KretzmannBookSpec { book_index: 8, slug: "1-samuel", chapters: 31 },
    KretzmannBookSpec { book_index: 9, slug: "2-samuel", chapters: 24 },
    KretzmannBookSpec { book_index: 10, slug: "1-kings", chapters: 22 },
    KretzmannBookSpec { book_index: 11, slug: "2-kings", chapters: 25 },
    KretzmannBookSpec { book_index: 12, slug: "1-chronicles", chapters: 29 },
    KretzmannBookSpec { book_index: 13, slug: "2-chronicles", chapters: 36 },
    KretzmannBookSpec { book_index: 14, slug: "ezra", chapters: 10 },
    KretzmannBookSpec { book_index: 15, slug: "nehemiah", chapters: 13 },
    KretzmannBookSpec { book_index: 16, slug: "esther", chapters: 10 },
    KretzmannBookSpec { book_index: 17, slug: "job", chapters: 42 },
    KretzmannBookSpec { book_index: 18, slug: "psalms", chapters: 150 },
    KretzmannBookSpec { book_index: 19, slug: "proverbs", chapters: 31 },
    KretzmannBookSpec { book_index: 20, slug: "ecclesiastes", chapters: 12 },
    KretzmannBookSpec { book_index: 21, slug: "song-of-solomon", chapters: 8 },
    KretzmannBookSpec { book_index: 22, slug: "isaiah", chapters: 66 },
    KretzmannBookSpec { book_index: 23, slug: "jeremiah", chapters: 52 },
    KretzmannBookSpec { book_index: 24, slug: "lamentations", chapters: 5 },
    KretzmannBookSpec { book_index: 25, slug: "ezekiel", chapters: 48 },
    KretzmannBookSpec { book_index: 26, slug: "daniel", chapters: 12 },
    KretzmannBookSpec { book_index: 27, slug: "hosea", chapters: 14 },
    KretzmannBookSpec { book_index: 28, slug: "joel", chapters: 3 },
    KretzmannBookSpec { book_index: 29, slug: "amos", chapters: 9 },
    KretzmannBookSpec { book_index: 30, slug: "obadiah", chapters: 1 },
    KretzmannBookSpec { book_index: 31, slug: "jonah", chapters: 4 },
    KretzmannBookSpec { book_index: 32, slug: "micah", chapters: 7 },
    KretzmannBookSpec { book_index: 33, slug: "nahum", chapters: 3 },
    KretzmannBookSpec { book_index: 34, slug: "habakkuk", chapters: 3 },
    KretzmannBookSpec { book_index: 35, slug: "zephaniah", chapters: 3 },
    KretzmannBookSpec { book_index: 36, slug: "haggai", chapters: 2 },
    KretzmannBookSpec { book_index: 37, slug: "zechariah", chapters: 14 },
    KretzmannBookSpec { book_index: 38, slug: "malachi", chapters: 4 },
    KretzmannBookSpec { book_index: 39, slug: "matthew", chapters: 28 },
    KretzmannBookSpec { book_index: 40, slug: "mark", chapters: 16 },
    KretzmannBookSpec { book_index: 41, slug: "luke", chapters: 24 },
    KretzmannBookSpec { book_index: 42, slug: "john", chapters: 21 },
    KretzmannBookSpec { book_index: 43, slug: "acts", chapters: 28 },
    KretzmannBookSpec { book_index: 44, slug: "romans", chapters: 16 },
    KretzmannBookSpec { book_index: 45, slug: "1-corinthians", chapters: 16 },
    KretzmannBookSpec { book_index: 46, slug: "2-corinthians", chapters: 13 },
    KretzmannBookSpec { book_index: 47, slug: "galatians", chapters: 6 },
    KretzmannBookSpec { book_index: 48, slug: "ephesians", chapters: 6 },
    KretzmannBookSpec { book_index: 49, slug: "philippians", chapters: 4 },
    KretzmannBookSpec { book_index: 50, slug: "colossians", chapters: 4 },
    KretzmannBookSpec { book_index: 51, slug: "1-thessalonians", chapters: 5 },
    KretzmannBookSpec { book_index: 52, slug: "2-thessalonians", chapters: 3 },
    KretzmannBookSpec { book_index: 53, slug: "1-timothy", chapters: 6 },
    KretzmannBookSpec { book_index: 54, slug: "2-timothy", chapters: 4 },
    KretzmannBookSpec { book_index: 55, slug: "titus", chapters: 3 },
    KretzmannBookSpec { book_index: 56, slug: "philemon", chapters: 1 },
    KretzmannBookSpec { book_index: 57, slug: "hebrews", chapters: 13 },
    KretzmannBookSpec { book_index: 58, slug: "james", chapters: 5 },
    KretzmannBookSpec { book_index: 59, slug: "1-peter", chapters: 5 },
    KretzmannBookSpec { book_index: 60, slug: "2-peter", chapters: 3 },
    KretzmannBookSpec { book_index: 61, slug: "1-john", chapters: 5 },
    KretzmannBookSpec { book_index: 62, slug: "2-john", chapters: 1 },
    KretzmannBookSpec { book_index: 63, slug: "3-john", chapters: 1 },
    KretzmannBookSpec { book_index: 64, slug: "jude", chapters: 1 },
    KretzmannBookSpec { book_index: 65, slug: "revelation", chapters: 22 },
];

// -----------------------------------------------------------------------
// Public data shapes.
// -----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    /// A lemma/quote-derived unit -- comments on the verse range its own
    /// excised KJV text covers (a single verse for a Type-A fragment; a
    /// possibly-wider range for a Type-B block).
    Verse,
    /// Prose before the chapter's own FIRST heading -- maps to the whole
    /// chapter's verse range (decision 2: "chapter-intro prose... becomes
    /// its own unit(s) mapped to the chapter's full range").
    ChapterIntro,
    /// Prose after a heading, before that section's own first lemma/quote
    /// -- maps to the section's own covered verse range (decision 2:
    /// "pericope-intro prose maps to the pericope range").
    PericopeIntro,
}

#[derive(Debug, Clone)]
pub struct KretzUnit {
    /// Stable within one chapter's own parse: `"kretzmann/{book}.{chapter}.{ordinal}"`,
    /// document order, 0-based -- this module's own id scheme (mirrors
    /// `kjv_adapter::verse_node_id`/`concord_adapter::text_unit_id`'s own
    /// "one deterministic id format per corpus" precedent, computed here
    /// rather than left to the graph adapter, since the ordinal is a fact
    /// about THIS parse, not about graph construction).
    pub id: String,
    pub book_index: u8,
    pub chapter: u16,
    pub verse_from: u16,
    pub verse_to: u16,
    pub kind: UnitKind,
    pub heading: Option<String>,
    /// Kretzmann's own prose, LEMMA-EXCISED -- never a byte of quoted KJV
    /// text (footnote markers already resolved to `" [Footnote N: ...]"`).
    pub text: String,
}

/// One excised lemma/quote fragment -- kept ONLY for `check_conservation`
/// (KRETZ-ACCEPT-1); never stored on the graph (LEMMA-EXCISION).
#[derive(Debug, Clone)]
pub struct ExcisedFragment {
    pub book_index: u8,
    pub chapter: u16,
    pub verse: u16,
    /// Document order within `(book_index, chapter, verse)` -- multiple
    /// fragments per verse are legitimate (GEN 1:2's own three-way split);
    /// concatenation order for the conservation check is THIS order.
    pub order: u32,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct ChapterStats {
    pub footnotes: usize,
    /// A footnote reference marker that landed INSIDE an excised lemma/
    /// quote span rather than in stored prose (never observed in the real
    /// corpus, but never silently assumed impossible -- the footnote's own
    /// text is excised from the comparison target either way, this just
    /// counts how often that excision fired).
    pub footnotes_in_lemma: usize,
    /// One line per disclosed structural anomaly (an orphaned/malformed
    /// marker, a leading-unnumbered lemma with no following numbered lemma
    /// to fold into, a pericope-intro unit whose own section carries zero
    /// lemma/quote units to derive a range from) -- named by chapter,
    /// never silent (the SAME "one disclosure line, never a guess" law
    /// `concord.rs`'s own `group_and_number_paragraphs` establishes).
    pub disclosures: Vec<String>,
}

pub struct ParsedChapter {
    pub book_index: u8,
    pub chapter: u16,
    pub units: Vec<KretzUnit>,
    /// Document order overall (not just per-verse) -- `check_conservation`
    /// groups by verse itself.
    pub fragments: Vec<ExcisedFragment>,
    pub stats: ChapterStats,
}

#[derive(Debug, Clone, Default)]
pub struct CorpusStats {
    pub pages: usize,
    pub units: usize,
    pub fragments: usize,
    pub footnotes: usize,
    pub footnotes_in_lemma: usize,
    pub disclosures: Vec<String>,
}

pub struct KretzmannCorpus {
    pub chapters: Vec<ParsedChapter>,
    pub stats: CorpusStats,
}

/// The one filesystem-touching entry point (mirrors `concord::read_all`'s
/// own "reads `root`'s own vendored files, parses each" shape) -- every
/// OTHER function in this module is pure `&str`-in/data-out.
pub fn read_all(root: &Path) -> Result<KretzmannCorpus> {
    let mut chapters = Vec::with_capacity(1189);
    let mut stats = CorpusStats::default();
    for book in BOOKS {
        for chapter in 1..=book.chapters {
            let path = root.join(book.slug).join(format!("{chapter}.html"));
            let html = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            let parsed = parse_chapter(&html, book.book_index, chapter, book.chapters)
                .with_context(|| format!("parsing {} (book_index {}, chapter {chapter})", path.display(), book.book_index))?;
            stats.pages += 1;
            stats.units += parsed.units.len();
            stats.fragments += parsed.fragments.len();
            stats.footnotes += parsed.stats.footnotes;
            stats.footnotes_in_lemma += parsed.stats.footnotes_in_lemma;
            for d in &parsed.stats.disclosures {
                stats.disclosures.push(format!("{}/{}: {}", book.slug, chapter, d));
            }
            chapters.push(parsed);
        }
    }
    Ok(KretzmannCorpus { chapters, stats })
}

// -----------------------------------------------------------------------
// Per-chapter parse.
// -----------------------------------------------------------------------

/// Restricts to the page's own real content: `<article ...>` (the
/// `data-pagefind-body` article shell every real chapter page carries) up
/// to its own `</article>` close. Defensive against chrome outside the
/// article (nav/footer) leaking a spurious marker match -- verified
/// unnecessary on the real pages (nothing outside `<article>` matches
/// `<h3`/`<h4`/`<strong>`/`<p class="bible"` in any real fetched page), but
/// cheap and honest to keep explicit, the SAME discipline `concord.rs`'s
/// own `main_content_slice` already establishes.
fn article_slice(html: &str) -> Result<&str> {
    let start = html.find("<article").context("no <article> tag found -- not a real chapter page")?;
    let gt = html[start..].find('>').map(|p| start + p + 1).context("malformed <article> opening tag")?;
    let end = html[gt..].find("</article>").map(|p| gt + p).context("no closing </article> tag found")?;
    Ok(&html[gt..end])
}

/// Splits `body` into the footnote-definition section (if any) and the
/// remaining main content (with the section's own markup removed).
fn split_off_footnotes(body: &str) -> (&str, Option<&str>) {
    match body.find(r#"<section data-footnotes"#) {
        None => (body, None),
        Some(start) => {
            let end = body[start..].find("</section>").map(|p| start + p + "</section>".len()).unwrap_or(body.len());
            (&body[..start], Some(&body[start..end]))
        }
    }
}

/// Parses the footnote-definition section into `footnote id -> cleaned
/// text` (backref arrow + its wrapping `<a>` stripped; every other tag
/// stripped generically; entities decoded; whitespace collapsed).
fn parse_footnote_definitions(section: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut search_from = 0usize;
    const NEEDLE: &str = r#"<li id="user-content-fn-"#;
    while let Some(rel) = section[search_from..].find(NEEDLE) {
        let li_start = search_from + rel;
        let id_val_start = li_start + r#"<li id="user-content-fn-"#.len();
        let Some(quote_rel) = section[id_val_start..].find('"') else { break };
        let fn_id = section[id_val_start..id_val_start + quote_rel].to_string();
        let Some(li_close_rel) = section[li_start..].find("</li>") else { break };
        let li_body = &section[li_start..li_start + li_close_rel];
        // The backref link (a bare arrow glyph inside its own <a>) is the
        // ONLY sub-element besides the footnote's own prose -- excised the
        // same way `concord.rs` excises a structural, non-prose marker.
        let no_backref = strip_between(li_body, "<a href=\"#user-content-fnref-", "</a>");
        let text = collapse_ws(&decode_entities(&strip_tags(&no_backref)));
        out.insert(fn_id, text);
        search_from = li_start + li_close_rel + "</li>".len();
    }
    out
}

/// Excises every `START..</a>` span whose `START` matches the given prefix
/// (used for the footnote backref link only -- narrowly scoped, unlike the
/// generic `strip_tags` pass that follows it).
fn strip_between(s: &str, start_prefix: &str, end_marker: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        let Some(rel) = rest.find(start_prefix) else {
            out.push_str(rest);
            break;
        };
        let Some(end_rel) = rest[rel..].find(end_marker) else {
            out.push_str(&rest[..rel]);
            break;
        };
        out.push_str(&rest[..rel]);
        rest = &rest[rel + end_rel + end_marker.len()..];
    }
    out
}

/// Sentinel wrapping a footnote's own text inline, replacing its inline
/// reference marker (module doc comment's own "FOOTNOTES" section) --
/// private-use characters, guaranteed absent from real source prose, so
/// the later resolve pass can find them exactly.
const FN_SENTINEL_OPEN: char = '\u{E000}';
const FN_SENTINEL_CLOSE: char = '\u{E001}';

/// Replaces every inline footnote-reference marker (`<sup><a href="#user-
/// content-fn-N" id="user-content-fnref-N" data-footnote-ref ...>N</a></sup>`
/// -- the href names the DEFINITION's own id, "fn-N"; the marker's OWN id,
/// "fnref-N", is what the definition's own backref link points back to)
/// with
/// `\u{E000}N:TEXT\u{E001}` (the looked-up definition's own cleaned text) --
/// resolved to its final, disclosed form later by `resolve_footnote_sentinels`
/// once the surrounding text's own role (stored prose vs. excised lemma) is
/// known. A reference naming an id with no matching definition (never
/// observed in the real corpus) falls back to `"missing"`, disclosed via
/// the returned count.
fn inline_footnote_refs(body: &str, defs: &BTreeMap<String, String>) -> (String, usize) {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    let mut count = 0usize;
    loop {
        let Some(rel) = rest.find(r##"<sup><a href="#user-content-fn-"##) else {
            out.push_str(rest);
            break;
        };
        let tag_start = rel;
        let id_val_start = tag_start + r##"<sup><a href="#user-content-fn-"##.len();
        let Some(quote_rel) = rest[id_val_start..].find('"') else {
            out.push_str(&rest[..tag_start + 1]);
            rest = &rest[tag_start + 1..];
            continue;
        };
        let fn_id = &rest[id_val_start..id_val_start + quote_rel];
        let Some(close_rel) = rest[id_val_start..].find("</sup>") else {
            out.push_str(&rest[..tag_start + 1]);
            rest = &rest[tag_start + 1..];
            continue;
        };
        let marker_end = id_val_start + close_rel + "</sup>".len();
        let text = defs.get(fn_id).cloned().unwrap_or_else(|| "missing".to_string());
        out.push_str(&rest[..tag_start]);
        out.push(FN_SENTINEL_OPEN);
        out.push_str(fn_id);
        out.push(':');
        out.push_str(&text);
        out.push(FN_SENTINEL_CLOSE);
        count += 1;
        rest = &rest[marker_end..];
    }
    (out, count)
}

/// Resolves every `\u{E000}N:TEXT\u{E001}` sentinel in `s` to its final
/// form: `" [Footnote N: TEXT]"` when `in_lemma` is false (stored prose,
/// decision 2's own "keep verbatim in place"); silently excised (counted)
/// when `in_lemma` is true (module doc comment's own "never observed, never
/// assumed impossible" footnote-inside-lemma anomaly).
fn resolve_footnote_sentinels(s: &str, in_lemma: bool) -> (String, usize) {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    let mut anomalies = 0usize;
    loop {
        let Some(open_rel) = rest.find(FN_SENTINEL_OPEN) else {
            out.push_str(rest);
            break;
        };
        let Some(close_rel) = rest[open_rel..].find(FN_SENTINEL_CLOSE) else {
            out.push_str(rest);
            break;
        };
        let inner = &rest[open_rel + FN_SENTINEL_OPEN.len_utf8()..open_rel + close_rel];
        let (fn_id, text) = inner.split_once(':').unwrap_or((inner, ""));
        out.push_str(&rest[..open_rel]);
        if in_lemma {
            anomalies += 1;
        } else {
            out.push_str(&format!(" [Footnote {fn_id}: {text}]"));
        }
        rest = &rest[open_rel + close_rel + FN_SENTINEL_CLOSE.len_utf8()..];
    }
    (out, anomalies)
}

/// The four recognized document-order markers -- module doc comment's own
/// "unified document-order walk".
enum Segment<'a> {
    H3(&'a str),
    H4(&'a str),
    /// One `<strong>...</strong>` span's own raw inner HTML (Type A).
    Lemma(&'a str),
    /// One `<p class="bible"...>...</p>` block's own raw inner HTML (Type B).
    Quote(&'a str),
    /// Plain text/other tags between recognized markers.
    Gap(&'a str),
}

/// Splits `body` into `Segment`s in document order.
fn segment(body: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    loop {
        let candidates = [
            body[pos..].find("<h3").map(|p| (pos + p, 0u8)),
            body[pos..].find("<h4").map(|p| (pos + p, 1u8)),
            body[pos..].find("<strong>").map(|p| (pos + p, 2u8)),
            body[pos..].find(r#"<p class="bible""#).map(|p| (pos + p, 3u8)),
        ];
        let next = candidates.into_iter().flatten().min_by_key(|&(p, _)| p);
        let Some((start, kind)) = next else {
            if pos < body.len() {
                out.push(Segment::Gap(&body[pos..]));
            }
            break;
        };
        if start > pos {
            out.push(Segment::Gap(&body[pos..start]));
        }
        match kind {
            0 => {
                let Some(gt) = body[start..].find('>').map(|p| start + p + 1) else { break };
                let Some(close) = body[gt..].find("</h3>").map(|p| gt + p) else { break };
                out.push(Segment::H3(&body[gt..close]));
                pos = close + "</h3>".len();
            }
            1 => {
                let Some(gt) = body[start..].find('>').map(|p| start + p + 1) else { break };
                let Some(close) = body[gt..].find("</h4>").map(|p| gt + p) else { break };
                out.push(Segment::H4(&body[gt..close]));
                pos = close + "</h4>".len();
            }
            2 => {
                let inner_start = start + "<strong>".len();
                let Some(close) = body[inner_start..].find("</strong>").map(|p| inner_start + p) else { break };
                out.push(Segment::Lemma(&body[inner_start..close]));
                pos = close + "</strong>".len();
            }
            _ => {
                let Some(gt) = body[start..].find('>').map(|p| start + p + 1) else { break };
                let Some(close) = body[gt..].find("</p>").map(|p| gt + p) else { break };
                out.push(Segment::Quote(&body[gt..close]));
                pos = close + "</p>".len();
            }
        }
    }
    out
}

/// Splits a Lemma/Quote span's own raw inner HTML at each bare verse-number
/// `<sup>` marker (`<sup>N</sup>` Type A or `<sup id="vN">N</sup>` Type B --
/// footnote sups are already gone by this point, replaced by
/// `inline_footnote_refs` before segmentation ever runs) into consecutive
/// `(verse, text)` fragments. The FIRST fragment carries `None` when the
/// span does not open with a marker (a continuation fragment, or a leading-
/// unnumbered lemma -- disambiguated by the caller's own `current_verse`
/// state, module doc comment's own "Type A" section).
fn split_by_verse_markers(raw: &str) -> (Vec<(Option<u16>, String)>, usize) {
    let mut fragments: Vec<(Option<u16>, String)> = Vec::new();
    let mut anomalies = 0usize;
    let mut cur_verse: Option<u16> = None;
    let mut cur_text = String::new();
    let mut rest = raw;
    loop {
        let Some(rel) = rest.find("<sup") else {
            cur_text.push_str(rest);
            break;
        };
        cur_text.push_str(&rest[..rel]);
        let Some(gt) = rest[rel..].find('>').map(|p| rel + p + 1) else {
            cur_text.push_str(&rest[rel..]);
            break;
        };
        let Some(close) = rest[gt..].find("</sup>").map(|p| gt + p) else {
            cur_text.push_str(&rest[rel..]);
            break;
        };
        let inner = &rest[gt..close];
        let after = close + "</sup>".len();
        // LEADING digit run only (mirrors `concord.rs`'s own `leading_
        // digits`): the real corpus sub-letters some verse markers
        // ("5a"/"5b", a finer split within one verse, the SAME idea as
        // GEN 1:2's own unlettered multi-fragment split, just explicitly
        // labeled here) -- the trailing letter carries no locus meaning of
        // its own, both "5a" and "5b" target verse 5.
        let digit_run: String = inner.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digit_run.is_empty() {
            // A real verse-number marker: close out the fragment so far,
            // open a new one at this number.
            fragments.push((cur_verse, std::mem::take(&mut cur_text)));
            cur_verse = digit_run.parse::<u16>().ok();
        } else {
            // No leading digit at all -- not a marker this parser
            // recognizes; disclosed, kept as ordinary text rather than
            // dropped.
            anomalies += 1;
            cur_text.push_str(&rest[rel..after]);
        }
        rest = &rest[after..];
    }
    fragments.push((cur_verse, cur_text));
    // The very first fragment is `(None, "")` whenever the span opens
    // exactly on a marker (the common case) -- an empty leading fragment
    // carries no content and is dropped rather than surfaced as a unit,
    // UNLESS it is the only fragment at all (a span with no marker and no
    // text -- kept so the caller sees a real, if empty, fragment rather
    // than a silently vanished span).
    let len_is_one = fragments.len() == 1;
    fragments.retain(|(_, t)| !t.trim().is_empty() || len_is_one);
    (fragments, anomalies)
}

/// If `gap`'s own raw HTML ends with a COMPLETE, bare verse-number `<sup>`
/// marker followed only by whitespace, extracts it (module doc comment's
/// own "floating sup before strong" quirk, e.g. `<sup>3</sup><strong>...`)
/// -- returns the remaining gap text (marker + trailing whitespace removed)
/// and the extracted verse number, if any.
fn extract_trailing_floating_verse(gap: &str) -> (&str, Option<u16>) {
    let trimmed_end = gap.trim_end();
    if !trimmed_end.ends_with("</sup>") {
        return (gap, None);
    }
    let Some(sup_start) = trimmed_end.rfind("<sup") else { return (gap, None) };
    let Some(gt) = trimmed_end[sup_start..].find('>').map(|p| sup_start + p + 1) else { return (gap, None) };
    let inner = &trimmed_end[gt..trimmed_end.len() - "</sup>".len()];
    let digit_run: String = inner.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digit_run.is_empty() {
        (&gap[..sup_start], digit_run.parse::<u16>().ok())
    } else {
        (gap, None)
    }
}

fn clean_heading(raw: &str) -> String {
    collapse_ws(&decode_entities(&strip_tags(raw)))
}

fn clean_prose(raw: &str) -> String {
    let (resolved, _anomalies) = resolve_footnote_sentinels(raw, false);
    collapse_ws(&decode_entities(&strip_tags(&resolved)))
}

/// Cleans an EXCISED lemma/quote fragment's own text -- same pipeline as
/// prose, but footnote sentinels (never observed here, module doc comment's
/// own disclosure) are silently excised, not rendered, and the excision is
/// counted so the caller can fold it into `ChapterStats.footnotes_in_lemma`.
fn clean_lemma(raw: &str) -> (String, usize) {
    let (resolved, anomalies) = resolve_footnote_sentinels(raw, true);
    (collapse_ws(&decode_entities(&strip_tags(&resolved))), anomalies)
}

/// Combines the active h3/pericope heading and h4/sub-heading into one
/// display string (module doc comment's own "heading" design: the more
/// specific h4, when present, is prefixed by its own parent h3 for full
/// context -- `CommentaryItem.heading` is one `Option<String>` field, not a
/// path, so this is the richest single-string composition available).
fn compose_heading(h3: &Option<String>, h4: &Option<String>) -> Option<String> {
    match (h3, h4) {
        (Some(a), Some(b)) => Some(format!("{a}: {b}")),
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (None, None) => None,
    }
}

/// Parses one chapter page's own article body into `KretzUnit`s + the
/// excised fragments KRETZ-ACCEPT-1 checks. `chapter_verse_count` is the
/// SAME real chapter-verse-count the caller's own canonical KJV source
/// knows (passed in, never re-derived from what Kretzmann happens to
/// cover -- decision 2's own "chapter intros to the chapter's full range"
/// needs the TRUE range, not an approximation).
pub fn parse_chapter(html: &str, book_index: u8, chapter: u16, chapter_verse_count: u16) -> Result<ParsedChapter> {
    let article = article_slice(html)?;
    let (main, footnote_section) = split_off_footnotes(article);
    let defs = footnote_section.map(parse_footnote_definitions).unwrap_or_default();
    let (main_resolved, footnote_count) = inline_footnote_refs(main, &defs);

    let segments = segment(&main_resolved);

    #[derive(Clone)]
    struct RawUnit {
        kind: UnitKind,
        heading: Option<String>,
        verse_from: Option<u16>,
        verse_to: Option<u16>,
        text: String,
        section: usize,
    }

    let mut raw_units: Vec<RawUnit> = Vec::new();
    let mut fragments: Vec<ExcisedFragment> = Vec::new();
    let mut disclosures: Vec<String> = Vec::new();
    let mut footnotes_in_lemma = 0usize;

    let mut h3: Option<String> = None;
    let mut h4: Option<String> = None;
    let mut section: usize = 0;
    let mut current_verse: Option<u16> = None;
    let mut pending_unnumbered: Vec<(usize, String)> = Vec::new(); // (raw_units index, excised lemma text) awaiting a fold-forward verse
    let mut open_unit: Option<usize> = None; // index into raw_units currently absorbing trailing prose
    let mut pending_prose = String::new();
    let mut frag_order: u32 = 0;

    let flush_prose = |pending_prose: &mut String, open_unit: Option<usize>, raw_units: &mut Vec<RawUnit>, heading: &Option<String>, section: usize| {
        let text = clean_prose(pending_prose);
        pending_prose.clear();
        if text.trim().is_empty() {
            return;
        }
        match open_unit {
            Some(idx) => {
                if !raw_units[idx].text.is_empty() {
                    raw_units[idx].text.push(' ');
                }
                raw_units[idx].text.push_str(&text);
            }
            None => {
                let kind = if heading.is_none() { UnitKind::ChapterIntro } else { UnitKind::PericopeIntro };
                raw_units.push(RawUnit { kind, heading: heading.clone(), verse_from: None, verse_to: None, text, section });
            }
        }
    };

    for seg in segments {
        match seg {
            Segment::Gap(g) => {
                let (g, floating) = extract_trailing_floating_verse(g);
                pending_prose.push_str(g);
                if let Some(v) = floating {
                    // The floating marker belongs to the NEXT lemma/quote
                    // span, not to this gap's own prose -- stash it by
                    // pre-registering `current_verse` so the following
                    // Lemma/Quote arm's own "no leading marker" branch
                    // still resolves it correctly (the SAME `None`-vs-
                    // `Some` decision either way; here it is already
                    // `Some` before that span is even reached).
                    flush_prose(&mut pending_prose, open_unit, &mut raw_units, &compose_heading(&h3, &h4), section);
                    current_verse = Some(v);
                    open_unit = None; // force the next span to open fresh
                }
            }
            Segment::H3(inner) => {
                flush_prose(&mut pending_prose, open_unit, &mut raw_units, &compose_heading(&h3, &h4), section);
                h3 = Some(clean_heading(inner));
                h4 = None;
                section += 1;
                open_unit = None;
            }
            Segment::H4(inner) => {
                flush_prose(&mut pending_prose, open_unit, &mut raw_units, &compose_heading(&h3, &h4), section);
                h4 = Some(clean_heading(inner));
                section += 1;
                open_unit = None;
            }
            Segment::Lemma(raw) => {
                flush_prose(&mut pending_prose, open_unit, &mut raw_units, &compose_heading(&h3, &h4), section);
                let (parts, anomalies) = split_by_verse_markers(raw);
                if anomalies > 0 {
                    disclosures.push(format!("chapter {chapter}: {anomalies} non-digit <sup> marker(s) inside a lemma span, kept as text"));
                }
                for (i, (verse_opt, text_raw)) in parts.into_iter().enumerate() {
                    let (text, fl) = clean_lemma(&text_raw);
                    footnotes_in_lemma += fl;
                    if text.trim().is_empty() {
                        continue;
                    }
                    let resolved_verse = if i == 0 {
                        match verse_opt.or(current_verse) {
                            Some(v) => Some(v),
                            None => None, // still unknown -- deferred (leading superscription)
                        }
                    } else {
                        verse_opt
                    };
                    let heading = compose_heading(&h3, &h4);
                    let unit_idx = raw_units.len();
                    // `text` (this string) IS the excised lemma -- it feeds
                    // `fragments` ONLY below, per LEMMA-EXCISION; the unit's
                    // own `.text` starts (and, for a Verse-kind unit, stays)
                    // empty here, filled ONLY by a later `flush_prose` call
                    // reading the COMMENTARY that follows this span.
                    raw_units.push(RawUnit { kind: UnitKind::Verse, heading, verse_from: resolved_verse, verse_to: resolved_verse, text: String::new(), section });
                    open_unit = Some(unit_idx);
                    match resolved_verse {
                        None => pending_unnumbered.push((unit_idx, text)),
                        Some(v) => {
                            current_verse = Some(v);
                            // A fresh numbered marker resolves every
                            // fragment still waiting on one FIRST, in their
                            // own original document order (module doc
                            // comment's own Psalm-superscription fold rule
                            // -- the leading unnumbered lemma(s) fold into
                            // the FIRST numbered verse that follows, and
                            // must keep the LOWER `order` -- they were
                            // read first).
                            for (p_idx, p_text) in pending_unnumbered.drain(..) {
                                raw_units[p_idx].verse_from = Some(v);
                                raw_units[p_idx].verse_to = Some(v);
                                fragments.push(ExcisedFragment { book_index, chapter, verse: v, order: frag_order, text: p_text });
                                frag_order += 1;
                            }
                            fragments.push(ExcisedFragment { book_index, chapter, verse: v, order: frag_order, text });
                            frag_order += 1;
                        }
                    }
                }
            }
            Segment::Quote(raw) => {
                flush_prose(&mut pending_prose, open_unit, &mut raw_units, &compose_heading(&h3, &h4), section);
                let (parts, anomalies) = split_by_verse_markers(raw);
                if anomalies > 0 {
                    disclosures.push(format!("chapter {chapter}: {anomalies} non-digit <sup> marker(s) inside a quote block, kept as text"));
                }
                let heading = compose_heading(&h3, &h4);
                let unit_idx = raw_units.len();
                raw_units.push(RawUnit { kind: UnitKind::Verse, heading, verse_from: None, verse_to: None, text: String::new(), section });
                let mut min_v: Option<u16> = None;
                let mut max_v: Option<u16> = None;
                for (i, (verse_opt, text_raw)) in parts.into_iter().enumerate() {
                    let (text, fl) = clean_lemma(&text_raw);
                    footnotes_in_lemma += fl;
                    if text.trim().is_empty() {
                        continue;
                    }
                    let resolved_verse = if i == 0 { verse_opt.or(current_verse) } else { verse_opt };
                    if resolved_verse.is_none() {
                        disclosures.push(format!("chapter {chapter}: a quote block fragment carries no verse number (unmappable residue, dropped)"));
                        continue;
                    }
                    let v = resolved_verse.unwrap();
                    current_verse = Some(v);
                    min_v = Some(min_v.map_or(v, |m| m.min(v)));
                    max_v = Some(max_v.map_or(v, |m| m.max(v)));
                    fragments.push(ExcisedFragment { book_index, chapter, verse: v, order: frag_order, text });
                    frag_order += 1;
                }
                if let (Some(a), Some(b)) = (min_v, max_v) {
                    raw_units[unit_idx].verse_from = Some(a);
                    raw_units[unit_idx].verse_to = Some(b);
                    open_unit = Some(unit_idx);
                } else {
                    // A quote block with zero resolvable verses -- disclosed
                    // above per-fragment; drop the empty shell unit rather
                    // than emit a rangeless CommentaryItem.
                    raw_units.pop();
                    open_unit = None;
                }
            }
        }
    }
    flush_prose(&mut pending_prose, open_unit, &mut raw_units, &compose_heading(&h3, &h4), section);

    if !pending_unnumbered.is_empty() {
        disclosures.push(format!(
            "chapter {chapter}: {} leading unnumbered lemma(s) never resolved to a verse (no numbered marker followed anywhere in the chapter) -- unmappable residue, dropped",
            pending_unnumbered.len()
        ));
    }

    // Pericope/chapter-intro range backfill: each intro unit's own range =
    // its SECTION's min..max verse among that section's own Verse-kind
    // units (module doc comment's own two-pass reasoning) -- ChapterIntro
    // (section 0, heading None) instead gets the TRUE full-chapter range.
    let mut section_ranges: BTreeMap<usize, (u16, u16)> = BTreeMap::new();
    for u in &raw_units {
        if u.kind == UnitKind::Verse {
            if let (Some(f), Some(t)) = (u.verse_from, u.verse_to) {
                let entry = section_ranges.entry(u.section).or_insert((f, t));
                entry.0 = entry.0.min(f);
                entry.1 = entry.1.max(t);
            }
        }
    }

    let mut units: Vec<KretzUnit> = Vec::new();
    let mut ordinal = 0usize;
    for u in raw_units.into_iter().filter(|u| !u.text.trim().is_empty()) {
        let (verse_from, verse_to) = match u.kind {
            UnitKind::ChapterIntro => (1, chapter_verse_count),
            UnitKind::PericopeIntro => match section_ranges.get(&u.section) {
                Some(&(f, t)) => (f, t),
                None => {
                    disclosures.push(format!("chapter {chapter}: a pericope-intro unit's own section carries zero lemma/quote units -- unmappable residue, dropped"));
                    continue;
                }
            },
            UnitKind::Verse => match (u.verse_from, u.verse_to) {
                (Some(f), Some(t)) => (f, t),
                _ => continue, // resolved above (dropped, disclosed) if unresolved
            },
        };
        units.push(KretzUnit {
            id: format!("kretzmann/{book_index}.{chapter}.{ordinal}"),
            book_index,
            chapter,
            verse_from,
            verse_to,
            kind: u.kind,
            heading: u.heading,
            text: u.text,
        });
        ordinal += 1;
    }

    Ok(ParsedChapter {
        book_index,
        chapter,
        units,
        fragments,
        stats: ChapterStats { footnotes: footnote_count, footnotes_in_lemma, disclosures },
    })
}

// -----------------------------------------------------------------------
// HTML micro-utilities -- SAME hand-written-scan house style `concord.rs`
// already establishes for this crate (no regex dependency).
// -----------------------------------------------------------------------

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        match rest.find('<') {
            None => {
                out.push_str(rest);
                break;
            }
            Some(start) => {
                out.push_str(&rest[..start]);
                match rest[start..].find('>') {
                    Some(end) => rest = &rest[start + end + 1..],
                    None => break,
                }
            }
        }
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decodes HTML entities -- the observed set across the real vendored
/// corpus (a superset of `concord.rs`'s own table: Kretzmann's prose adds
/// a plain hyphen-adjacent en/em dash usage already covered, plus
/// double-low-9 quotation marks not seen in the Book of Concord's own
/// pages) plus generic numeric `&#NNN;`/`&#xHHH;` escapes.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp_rel) = rest.find('&') {
        out.push_str(&rest[..amp_rel]);
        let after = &rest[amp_rel + 1..];
        if let Some(semi_rel) = after.find(';').filter(|&r| r <= 10) {
            let name = &after[..semi_rel];
            if let Some(ch) = decode_one_entity(name) {
                out.push(ch);
                rest = &after[semi_rel + 1..];
                continue;
            }
        }
        out.push('&');
        rest = after;
    }
    out.push_str(rest);
    out
}

fn decode_one_entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "ndash" => '\u{2013}',
        "mdash" => '\u{2014}',
        "hellip" => '\u{2026}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "sbquo" => '\u{201A}',
        "bdquo" => '\u{201E}',
        "middot" => '\u{B7}',
        "nbsp" => ' ',
        other if other.starts_with('#') => {
            let numeric = &other[1..];
            let code = if let Some(hex) = numeric.strip_prefix('x').or_else(|| numeric.strip_prefix('X')) {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                numeric.parse::<u32>().ok()?
            };
            char::from_u32(code)?
        }
        _ => return None,
    })
}

// -----------------------------------------------------------------------
// KRETZ-ACCEPT-1: the conservation law.
// -----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviationClass {
    /// Exact byte match against the canonical (restored) verse text.
    Exact,
    /// A DISCLOSED EQUIVALENCE (decision 3): the naive concatenation
    /// differs from canonical only by case and/or punctuation -- casefold
    /// plus ASCII/typographic-punctuation-stripped comparison matches
    /// exactly. Covers BOTH observed real classes: (1) Tetragrammaton/
    /// reverential-pronoun case convention (PSA 110:1's own "The Lord" vs.
    /// our restored "The LORD", "Thou"/"My"/"Thine"/"Thy" vs. lowercase),
    /// and (2) fragment/quote-boundary punctuation normalization (the
    /// digital edition renders each excised lemma/quote fragment as its
    /// own typographic "sentence" -- a comma/semicolon that continues one
    /// KJV sentence across a fragment boundary becomes a period + capital,
    /// GEN 1:2's own flagship case). Mechanical and symmetric -- NEVER
    /// masks a word/content difference, since the underlying word sequence
    /// must still match exactly.
    MechanicalCaseAndPunct,
    /// A THIRD disclosed equivalence class, found empirically (never
    /// assumed up front) mining the real corpus's own remaining mismatches:
    /// the digital edition systematically MODERNIZES the KJV's own archaic/
    /// British spelling to modern American spelling (`shew`->`show`,
    /// `honour`->`honor`, `sepulchre`->`sepulcher`, `worshipped`->
    /// `worshiped`, ...) -- decision 3's own named allowance is explicitly
    /// "case/SPELLING variance," not case alone. `SPELLING_VARIANTS` below
    /// is a CURATED, auditable table (never a fuzzy/edit-distance guess,
    /// which risks silently equating two DIFFERENT words) built from the
    /// real corpus's own high-frequency (>=2 occurrences), manually vetted
    /// word pairs -- every entry is the SAME word, differently spelled,
    /// never a different word that happens to have a similar shape (a
    /// handful of superficially similar real pairs were DELIBERATELY
    /// excluded for exactly this reason -- see the table's own doc
    /// comment). Applied on top of case+punctuation normalization (a
    /// strict superset of `MechanicalCaseAndPunct`), so this class also
    /// still requires the SAME underlying word sequence, position for
    /// position -- never masks an added/removed/reordered word.
    MechanicalCaseSpellingAndPunct,
    /// Neither of the above -- a genuine content deviation, collected for
    /// per-case resolution (decision 3's own deviation policy).
    Mismatch,
}

#[derive(Debug, Clone)]
pub struct VerseCheck {
    pub book_index: u8,
    pub chapter: u16,
    pub verse: u16,
    pub concatenated: String,
    pub canonical: String,
    pub class: DeviationClass,
}

#[derive(Debug, Clone, Default)]
pub struct ConservationReport {
    pub checked: usize,
    pub exact: usize,
    pub mechanical: usize,
    pub mechanical_spelling: usize,
    pub mismatches: Vec<VerseCheck>,
    /// A canonical verse with zero excised fragments at all -- lawful
    /// (decision 3: "he summarizes some spans"), asserted + disclosed, not
    /// an error.
    pub uncovered: Vec<(u8, u16, u16)>,
}

/// Mechanical comparison key (decision 3's own "e.g. case-fold", widened --
/// module's own `DeviationClass::MechanicalCaseAndPunct` doc comment):
/// lowercase, then every ASCII + common Unicode punctuation/quote/dash
/// character replaced with a SPACE (never simply deleted -- deleting a
/// hyphen would wrongly merge "fruit-tree" into the single token
/// "fruittree", which could never equal canonical's own two-word "fruit
/// tree"; a space keeps both sides' own WORD BOUNDARIES intact, which is
/// the whole point of a mechanical, word-content-preserving comparison),
/// then whitespace-collapsed. Symmetric: applied identically to both
/// sides, so a genuine word/content difference still fails this
/// comparison too (only case/punctuation are ever ignored).
fn mechanical_key(s: &str) -> String {
    let spaced: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_punctuation() || matches!(c, '\u{2013}' | '\u{2014}' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{2026}') {
                ' '
            } else {
                c
            }
        })
        .collect();
    collapse_ws(&spaced.to_lowercase())
}

/// Curated (digital-edition American spelling -> our KJV/archaic-British
/// canonical spelling) pairs -- `DeviationClass::MechanicalCaseSpellingAndPunct`'s
/// own doc comment has the full derivation/vetting discipline. Built
/// EXCLUSIVELY from real corpus mismatches occurring >=2 times, each
/// manually confirmed to be the SAME WORD, never a different word of
/// similar shape. Deliberately EXCLUDED, despite appearing in the mined
/// data (disclosed here, not silently dropped): grammatical/word-choice
/// pairs with no spelling relationship at all ("into"/"unto", "shall"/"ye",
/// "thy"/"thine", "has"/"hath", "farther"/"further" -- real word-choice,
/// not orthography); "chapters"/"chapiters" (a MEANING change -- book
/// divisions vs. a pillar's own capital, never conflated); "nebuchadnezzar"/
/// "nebuchadrezzar" (a genuine KJV-internal name-form variance across
/// different books, not a transcription convention -- collapsing it here
/// could mask a genuine cross-book citation issue). Every one of THOSE
/// stays a disclosed `Mismatch`, individually reviewable, never silently
/// absorbed.
const SPELLING_VARIANTS: &[(&str, &str)] = &[
    ("show", "shew"), ("shows", "shews"), ("showed", "shewed"), ("showeth", "sheweth"), ("showing", "shewing"), ("showest", "shewest"), ("showbread", "shewbread"),
    ("honor", "honour"), ("honors", "honours"), ("honored", "honoured"), ("honorable", "honourable"), ("honoreth", "honoureth"), ("honorest", "honourest"),
    ("dishonor", "dishonour"), ("dishonoreth", "dishonoureth"), ("dishonorest", "dishonourest"),
    ("neighbor", "neighbour"), ("neighbors", "neighbours"),
    ("labor", "labour"), ("labors", "labours"), ("labored", "laboured"), ("laboring", "labouring"), ("laborer", "labourer"), ("laborers", "labourers"), ("laboreth", "laboureth"),
    ("favor", "favour"), ("favored", "favoured"), ("favorable", "favourable"), ("favorest", "favourest"),
    ("worshiped", "worshipped"), ("worshipers", "worshippers"), ("worshiper", "worshipper"), ("worshiping", "worshipping"), ("worshipeth", "worshippeth"),
    ("burned", "burnt"),
    ("cherubim", "cherubims"),
    ("sepulcher", "sepulchre"), ("sepulchers", "sepulchres"),
    ("savor", "savour"), ("savory", "savoury"), ("savorest", "savourest"),
    ("naught", "nought"),
    ("savior", "saviour"), ("saviors", "saviours"),
    ("aught", "ought"),
    ("synagog", "synagogue"), ("synagogs", "synagogues"),
    ("valor", "valour"),
    ("carcass", "carcase"), ("carcasses", "carcases"),
    ("veil", "vail"),
    ("brazen", "brasen"),
    ("caesar", "cesar"), ("caesarea", "cesarea"),
    ("armor", "armour"), ("armory", "armoury"),
    ("defense", "defence"), ("defensed", "defenced"),
    ("jubilee", "jubile"),
    ("marvelous", "marvellous"), ("marveled", "marvelled"), ("marvelously", "marvellously"),
    ("counselors", "counsellors"), ("counselor", "counsellor"), ("counseled", "counselled"),
    ("caesarea", "cesarea"),
    ("nethinim", "nethinims"),
    ("recompense", "recompence"), ("recompenses", "recompences"),
    ("offense", "offence"), ("offenses", "offences"),
    ("color", "colour"), ("colors", "colours"), ("colored", "coloured"),
    ("music", "musick"),
    ("basins", "basons"), ("basin", "bason"),
    ("miter", "mitre"),
    ("ax", "axe"),
    ("mortar", "morter"),
    ("rumor", "rumour"), ("rumors", "rumours"),
    ("scepter", "sceptre"),
    ("steadfastly", "stedfastly"), ("steadfast", "stedfast"), ("steadfastness", "stedfastness"),
    ("cloak", "cloke"),
    ("plaster", "plaister"), ("plastered", "plaistered"),
    ("odors", "odours"),
    ("rearward", "rereward"),
    ("alphaeus", "alpheus"),
    ("anakim", "anakims"),
    ("forbade", "forbad"),
    ("gomorrah", "gomorrha"),
    ("melchizedek", "melchisedec"),
    ("o", "oh"),
    ("thoroughly", "throughly"),
    ("traffic", "traffick"),
    ("behavior", "behaviour"),
    ("caterpillars", "caterpillers"), ("caterpillar", "caterpiller"),
    ("ceiled", "cieled"),
    ("enclosed", "inclosed"), ("enclose", "inclose"),
    ("entreated", "intreated"), ("entreaty", "intreaty"),
    ("inquire", "enquire"),
    ("lentils", "lentiles"),
    ("loathe", "lothe"), ("loathed", "lothed"),
    ("sarah", "sara"),
    ("strewed", "strawed"),
    ("cumin", "cummin"),
    ("fullness", "fulness"),
    ("pretense", "pretence"),
    ("sponge", "spunge"),
    ("succor", "succour"), ("succored", "succoured"),
    ("traveling", "travelling"), ("traveler", "traveller"),
    ("vapor", "vapour"), ("vapors", "vapours"),
    ("woolen", "woollen"),
    ("aeneas", "eneas"),
    ("always", "alway"),
    ("appareled", "apparelled"),
    ("assuaged", "asswaged"), ("assuage", "asswage"),
    ("behooved", "behoved"),
    ("chestnut", "chesnut"),
    ("cuckoo", "cuckow"),
    ("endeavored", "endeavoured"), ("endeavoring", "endeavouring"), ("endeavors", "endeavours"), ("endeavor", "endeavour"),
    ("fulfill", "fulfil"),
    ("grizzled", "grisled"),
    ("hymenaeus", "hymeneus"),
    ("lunatic", "lunatick"),
    ("nicolaitanes", "nicolaitans"),
    ("niter", "nitre"),
    ("osprey", "ospray"),
    ("paid", "payed"),
    ("publicly", "publickly"),
    ("raze", "rase"),
    ("revelings", "revellings"),
    ("selvage", "selvedge"),
    ("seraphim", "seraphims"),
    ("sergeants", "serjeants"),
    ("sismai", "sisamai"),
    ("theater", "theatre"),
    ("unblamable", "unblameable"),
    ("unmovable", "unmoveable"),
    ("zacchaeus", "zaccheus"),
    ("zedec", "zedek"),
];

/// Applies `SPELLING_VARIANTS` word-by-word on top of `mechanical_key`'s
/// own case+punctuation normalization -- `DeviationClass::
/// MechanicalCaseSpellingAndPunct`'s own doc comment.
fn spelling_key(s: &str) -> String {
    mechanical_key(s)
        .split(' ')
        .map(|w| SPELLING_VARIANTS.iter().find(|&&(american, _)| american == w).map(|&(_, british)| british).unwrap_or(w))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Runs KRETZ-ACCEPT-1 over the whole corpus: per verse, the excised
/// fragments (in `ExcisedFragment.order`) concatenate (single-space joined)
/// and must equal `canonical`'s own text for that verse -- exactly, or
/// under the disclosed mechanical equivalence above. `canonical` is keyed
/// `(book_index, chapter, verse)` -- the caller's own real, RESTORED
/// (KJV-CASE + KJV-CASE-2) canonical text (this function is pure; it takes
/// that map, never derives it).
pub fn check_conservation(fragments: &[ExcisedFragment], canonical: &BTreeMap<(u8, u16, u16), String>) -> ConservationReport {
    let mut by_verse: BTreeMap<(u8, u16, u16), Vec<&ExcisedFragment>> = BTreeMap::new();
    for f in fragments {
        by_verse.entry((f.book_index, f.chapter, f.verse)).or_default().push(f);
    }
    for v in by_verse.values_mut() {
        v.sort_by_key(|f| f.order);
    }

    let mut report = ConservationReport::default();
    for (&key, canon_text) in canonical {
        let Some(frags) = by_verse.get(&key) else {
            report.uncovered.push(key);
            continue;
        };
        let concatenated = frags.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join(" ");
        report.checked += 1;
        let class = if concatenated == *canon_text {
            report.exact += 1;
            DeviationClass::Exact
        } else if mechanical_key(&concatenated) == mechanical_key(canon_text) {
            report.mechanical += 1;
            DeviationClass::MechanicalCaseAndPunct
        } else if spelling_key(&concatenated) == spelling_key(canon_text) {
            report.mechanical_spelling += 1;
            DeviationClass::MechanicalCaseSpellingAndPunct
        } else {
            DeviationClass::Mismatch
        };
        if class == DeviationClass::Mismatch {
            report.mismatches.push(VerseCheck { book_index: key.0, chapter: key.1, verse: key.2, concatenated, canonical: canon_text.clone(), class });
        }
    }
    report
}

// -----------------------------------------------------------------------
// The date mine: verbatim dating-clause extraction over stored prose.
// -----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Calendar {
    Bc,
    Ad,
    Am,
}

#[derive(Debug, Clone)]
pub struct DateClause {
    pub verbatim: String,
    pub calendar: Calendar,
    pub year: u32,
    pub approx: bool,
}

/// Scans `text` (a `KretzUnit.text`) for verbatim dating clauses --
/// PARSING ONLY, never interpretation (the scouting memo's own binding
/// law): B.C./A.D. year markers (both real orderings observed in the real
/// corpus -- "606 B. C." AND "A. D. 70"), an "about " prefix immediately
/// adjacent sets `approx`. Anno Mundi markers are matched defensively
/// (`"Anno Mundi"`/`"A. M."`) though ZERO real instances were found across
/// the full real corpus (disclosed in the batch report's own class table,
/// not silently assumed present). Reign-year formulas ("in the fourth year
/// of Jehoiakim", common in the real corpus) are NOT extracted as rows in
/// v1 -- disclosed, deliberate: the row shape below has no field for them
/// (no numeric year, no calendar), so forcing one would be exactly the
/// "silent" fabrication the scouting memo's own laws forbid; the class
/// itself is counted separately by the caller's own report, never silently
/// dropped without a count. Every returned `verbatim` is a real substring
/// of `text` (asserted by construction: it is sliced directly from it).
pub fn extract_date_clauses(text: &str) -> Vec<DateClause> {
    const MARKERS: &[(&str, Calendar)] = &[("B. C.", Calendar::Bc), ("A. D.", Calendar::Ad), ("B.C.", Calendar::Bc), ("A.D.", Calendar::Ad), ("Anno Mundi", Calendar::Am), ("A. M.", Calendar::Am)];

    let mut occurrences: Vec<(usize, usize, Calendar)> = Vec::new();
    for &(marker, cal) in MARKERS {
        let mut start = 0usize;
        while let Some(rel) = text[start..].find(marker) {
            let pos = start + rel;
            let end = pos + marker.len();
            let overlaps = occurrences.iter().any(|&(s, e, _)| pos < e && s < end);
            if !overlaps {
                occurrences.push((pos, end, cal));
            }
            start = pos + marker.len().max(1);
        }
    }
    occurrences.sort_by_key(|&(s, ..)| s);

    let mut out = Vec::new();
    for (start, end, cal) in occurrences {
        // Backward: a digit run immediately before the marker (one
        // optional space between) -- the dominant real convention for
        // both B.C. and A.D. ("606 B. C.", "70 A. D.").
        let before = &text[..start];
        let before_trimmed = before.trim_end_matches(' ');
        let digits_end = before_trimmed.len();
        let digits_start = before_trimmed.len() - before_trimmed.chars().rev().take_while(|c| c.is_ascii_digit()).map(char::len_utf8).sum::<usize>();
        if digits_start < digits_end && before_trimmed.len() != before.len() {
            let year_str = &before_trimmed[digits_start..digits_end];
            if let Ok(year) = year_str.parse::<u32>() {
                // `ends_with` is char-boundary-safe on any prefix slice
                // (unlike a fixed BYTE offset back from `digits_start`,
                // which could land mid-character on non-ASCII prose) --
                // `before_trimmed[..digits_start]` is itself always a valid
                // boundary (it is exactly where the trailing digit run,
                // summed by `char::len_utf8`, begins).
                let prefix = &before_trimmed[..digits_start];
                let approx = prefix.ends_with("about ");
                let clause_start = if approx { find_about_start(before_trimmed, digits_start) } else { digits_start };
                out.push(DateClause { verbatim: text[clause_start..end].to_string(), calendar: cal, year, approx });
                continue;
            }
        }
        // Forward: a digit run immediately after the marker's own trailing
        // space ("A. D. 70") -- the secondary real convention, A.D. only
        // in the observed corpus, matched generally here regardless.
        let after = &text[end..];
        let after_trimmed = after.trim_start_matches(' ');
        if after_trimmed.len() != after.len() || after.starts_with(' ') {
            let digits: String = after_trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                if let Ok(year) = digits.parse::<u32>() {
                    let clause_end = end + (after.len() - after_trimmed.len()) + digits.len();
                    out.push(DateClause { verbatim: text[start..clause_end].to_string(), calendar: cal, year, approx: false });
                }
            }
        }
    }
    out
}

fn find_about_start(s: &str, digits_start: usize) -> usize {
    let window = &s[..digits_start];
    window.rfind("about ").unwrap_or(digits_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real byte-verbatim excerpts from the vendored pages (verified against
    // `data/raw/kretzmann/*` directly -- decision 9's own "paragraphs
    // verbatim from source" discipline, proven at the unit level).

    const GEN_1_EXCERPT: &str = r#"<h3 id="the-creation-of-the-world">The Creation of the World.</h3>
<h4 id="the-creation-of-chaos-and-light">The Creation of Chaos and Light</h4>
<p><strong><sup>1</sup>In the beginning God created the heaven and the earth.</strong> In the beginning, cp. John 1, 1. <strong><sup>2</sup>And the earth was without form and void.</strong> The material substance. <strong>And darkness was upon the face of the deep.</strong> There was, as yet, no elemental light. <strong>And the Spirit of God moved upon the face of the waters.</strong> The third person. <strong><sup>3</sup>And God said, Let there be light; and there was light.</strong> God spoke.</p>"#;

    #[test]
    fn gen_1_2_splits_into_three_fragments_and_the_prose_between_attaches_to_each() {
        let parsed = parse_chapter(&wrap_article(GEN_1_EXCERPT), 0, 1, 31).unwrap();
        // Units: v1, v2(a), v2(b), v2(c), v3 -- five Verse-kind units.
        assert_eq!(parsed.units.len(), 5, "units: {:#?}", parsed.units.iter().map(|u| (u.verse_from, u.verse_to, &u.text)).collect::<Vec<_>>());
        assert_eq!((parsed.units[1].verse_from, parsed.units[1].verse_to), (2, 2));
        assert_eq!((parsed.units[2].verse_from, parsed.units[2].verse_to), (2, 2));
        assert_eq!((parsed.units[3].verse_from, parsed.units[3].verse_to), (2, 2));
        assert_eq!(parsed.units[1].text, "The material substance.");
        assert_eq!(parsed.units[2].text, "There was, as yet, no elemental light.");
        assert_eq!(parsed.units[3].text, "The third person.");
        assert_eq!(parsed.units[0].heading.as_deref(), Some("The Creation of the World.: The Creation of Chaos and Light"));

        let v2_fragments: Vec<&str> = parsed.fragments.iter().filter(|f| f.verse == 2).map(|f| f.text.as_str()).collect();
        assert_eq!(v2_fragments, vec!["And the earth was without form and void.", "And darkness was upon the face of the deep.", "And the Spirit of God moved upon the face of the waters."]);
    }

    fn wrap_article(inner: &str) -> String {
        format!(r#"<html><body><article data-pagefind-body>{inner}</article></body></html>"#)
    }

    const PSA_110_EXCERPT: &str = r#"<h3 id="a">A Psalm of Christ.</h3>
<p><strong>A psalm of David,</strong> altogether prophetic. <strong><sup>1</sup>The Lord said unto my Lord,</strong> literally. <strong>Sit Thou at My right hand,</strong> emblem. <strong>until I make Thine enemies Thy footstool.</strong> The Messiah.</p>"#;

    #[test]
    fn psa_110_1_leading_superscription_folds_into_verse_1_and_matches_canonical_under_the_mechanical_class() {
        let parsed = parse_chapter(&wrap_article(PSA_110_EXCERPT), 18, 110, 7).unwrap();
        // The unnumbered superscription lemma folds into verse 1 -- ALL
        // four fragments target verse 1 (the same locus our own canonical
        // layer folds the superscription into).
        let v1: Vec<&ExcisedFragment> = parsed.fragments.iter().filter(|f| f.verse == 1).collect();
        assert_eq!(v1.len(), 4, "fragments: {:#?}", parsed.fragments.iter().map(|f| (f.verse, &f.text)).collect::<Vec<_>>());

        let mut canonical = BTreeMap::new();
        canonical.insert((18u8, 110u16, 1u16), "A Psalm of David. The LORD said unto my Lord, Sit thou at my right hand, until I make thine enemies thy footstool.".to_string());
        let report = check_conservation(&parsed.fragments, &canonical);
        assert_eq!(report.mismatches.len(), 0, "mismatches: {:#?}", report.mismatches);
        assert_eq!(report.mechanical, 1, "PSA 110:1 must pass under the disclosed mechanical (case+punct) equivalence, not exact");
    }

    #[test]
    fn conservation_law_flags_a_genuine_content_deviation_as_a_mismatch() {
        let mut fragments = vec![ExcisedFragment { book_index: 0, chapter: 1, verse: 1, order: 0, text: "In the beginning God made the heaven and the earth.".to_string() }];
        let mut canonical = BTreeMap::new();
        canonical.insert((0u8, 1u16, 1u16), "In the beginning God created the heaven and the earth.".to_string());
        let report = check_conservation(&fragments, &canonical);
        assert_eq!(report.mismatches.len(), 1, "'made' vs 'created' is real word content, not case/punctuation");
        assert_eq!(report.mismatches[0].class, DeviationClass::Mismatch);

        fragments[0].text = "In the beginning God created the heaven and the earth.".to_string();
        let report2 = check_conservation(&fragments, &canonical);
        assert_eq!(report2.exact, 1);
        assert_eq!(report2.mismatches.len(), 0);
    }

    #[test]
    fn uncovered_verse_is_disclosed_not_silently_dropped() {
        let fragments: Vec<ExcisedFragment> = vec![];
        let mut canonical = BTreeMap::new();
        canonical.insert((0u8, 1u16, 1u16), "In the beginning.".to_string());
        let report = check_conservation(&fragments, &canonical);
        assert_eq!(report.uncovered, vec![(0u8, 1u16, 1u16)]);
        assert_eq!(report.checked, 0);
    }

    const JHN_3_TYPE_B_EXCERPT: &str = r#"<h3 id="a">The Visit of Nicodemus. John 3, 1-21.</h3>
<h4 id="b">The call by night:</h4>
<p class="bible" data-pagefind-weight="0.5"><sup id="v1">1</sup>There was a man of the Pharisees, named Nicodemus, a ruler of the Jews. <sup id="v2">2</sup>The same came to Jesus by night. <sup id="v3">3</sup>Jesus answered and said unto him, Verily, verily.</p>
<p>Here is an incident from the happenings of this Passover week.</p>
<p>Note: the Holy Ghost does His work.</p>
<h4 id="c">The witness from above:</h4>"#;

    #[test]
    fn type_b_quote_block_becomes_one_unit_spanning_its_own_verse_range_with_prose_from_both_following_paragraphs() {
        let parsed = parse_chapter(&wrap_article(JHN_3_TYPE_B_EXCERPT), 42, 3, 36).unwrap();
        assert_eq!(parsed.units.len(), 1, "units: {:#?}", parsed.units.iter().map(|u| (u.verse_from, u.verse_to, &u.text)).collect::<Vec<_>>());
        assert_eq!((parsed.units[0].verse_from, parsed.units[0].verse_to), (1, 3));
        assert_eq!(parsed.units[0].text, "Here is an incident from the happenings of this Passover week. Note: the Holy Ghost does His work.");
        assert_eq!(parsed.units[0].kind, UnitKind::Verse);

        let by_verse: Vec<(u16, &str)> = parsed.fragments.iter().map(|f| (f.verse, f.text.as_str())).collect();
        assert_eq!(by_verse, vec![(1, "There was a man of the Pharisees, named Nicodemus, a ruler of the Jews."), (2, "The same came to Jesus by night."), (3, "Jesus answered and said unto him, Verily, verily.")]);
    }

    const PSA_1_INTRO_EXCERPT: &str = r#"<h3 id="a">The Difference Between the Righteous and the Ungodly.</h3>
<p>All men are sinners: all have sinned and come short of the glory of God.</p>
<p><strong><sup>1</sup>Blessed is the man,</strong> literally, blessednesses. <strong>that walketh not in the counsel of the ungodly,</strong> making the plan.</p>"#;

    #[test]
    fn pericope_intro_prose_maps_to_the_sections_own_covered_verse_range() {
        let parsed = parse_chapter(&wrap_article(PSA_1_INTRO_EXCERPT), 18, 1, 6).unwrap();
        let intro = parsed.units.iter().find(|u| u.kind == UnitKind::PericopeIntro).expect("a pericope-intro unit must exist");
        assert_eq!(intro.text, "All men are sinners: all have sinned and come short of the glory of God.");
        assert_eq!((intro.verse_from, intro.verse_to), (1, 1), "the section's own real range (this fixture's lemmas only reach verse 1)");
    }

    const CHAPTER_INTRO_EXCERPT: &str = r#"<p>General orientation before any heading appears at all.</p>
<h3 id="a">First Pericope.</h3>
<p><strong><sup>1</sup>Text.</strong> Commentary.</p>"#;

    #[test]
    fn chapter_intro_prose_before_any_heading_maps_to_the_true_full_chapter_range() {
        let parsed = parse_chapter(&wrap_article(CHAPTER_INTRO_EXCERPT), 0, 5, 27).unwrap();
        let intro = parsed.units.iter().find(|u| u.kind == UnitKind::ChapterIntro).expect("a chapter-intro unit must exist");
        assert_eq!(intro.text, "General orientation before any heading appears at all.");
        assert_eq!((intro.verse_from, intro.verse_to), (1, 27), "the TRUE chapter range (27), not merely what this fixture's own single lemma covers");
    }

    #[test]
    fn footnote_reference_resolves_to_verbatim_bracketed_text_in_place() {
        let html = wrap_article(
            r##"<h3 id="a">H</h3>
<p><strong><sup>1</sup>Text.</strong> Commentary with a note.<sup><a href="#user-content-fn-1" id="user-content-fnref-1" data-footnote-ref aria-describedby="footnote-label">1</a></sup></p>
<section data-footnotes class="footnotes"><h2 class="sr-only" id="footnote-label">Footnotes</h2>
<ol><li id="user-content-fn-1"><p>A real footnote body. <a href="#user-content-fnref-1" data-footnote-backref="" aria-label="Back to reference 1" class="data-footnote-backref">&#8617;</a></p></li></ol>
</section>"##,
        );
        let parsed = parse_chapter(&html, 0, 1, 1).unwrap();
        assert_eq!(parsed.stats.footnotes, 1);
        assert_eq!(parsed.units[0].text, "Commentary with a note. [Footnote 1: A real footnote body.]");
    }

    #[test]
    fn entity_decoding_covers_the_observed_real_set_plus_numeric_escapes() {
        assert_eq!(decode_entities("Godâ€™s"), "Godâ€™s", "not a real entity -- mojibake bytes pass through untouched (never this parser's own concern; the real files are proper UTF-8)");
        assert_eq!(decode_entities("God&rsquo;s &ldquo;Good exceedingly.&rdquo; &ndash; &#39;quoted&#39;"), "God\u{2019}s \u{201C}Good exceedingly.\u{201D} \u{2013} 'quoted'");
    }

    #[test]
    fn date_clause_extraction_covers_both_real_orderings_and_the_about_approximation() {
        let clauses = extract_date_clauses("The city fell about 606 B. C. and the temple, dedicated A. D. 70, was later destroyed.");
        assert_eq!(clauses.len(), 2, "{clauses:#?}");
        assert_eq!(clauses[0].verbatim, "about 606 B. C.");
        assert_eq!(clauses[0].calendar, Calendar::Bc);
        assert_eq!(clauses[0].year, 606);
        assert!(clauses[0].approx);
        assert_eq!(clauses[1].verbatim, "A. D. 70");
        assert_eq!(clauses[1].calendar, Calendar::Ad);
        assert_eq!(clauses[1].year, 70);
        assert!(!clauses[1].approx);
    }

    #[test]
    fn every_date_clause_verbatim_is_a_real_substring_of_its_own_source_text() {
        let text = "Some events: 536 B. C., about 444 B. C., and A. D. 30 all matter.";
        for c in extract_date_clauses(text) {
            assert!(text.contains(&c.verbatim), "clause '{}' must be a literal substring of its source text", c.verbatim);
        }
    }

    #[test]
    fn read_all_books_table_has_66_entries_summing_to_1189_chapters_in_canonical_order() {
        assert_eq!(BOOKS.len(), 66);
        let total: u32 = BOOKS.iter().map(|b| b.chapters as u32).sum();
        assert_eq!(total, 1189);
        for (i, b) in BOOKS.iter().enumerate() {
            assert_eq!(b.book_index as usize, i);
        }
        assert_eq!(BOOKS[0].slug, "genesis");
        assert_eq!(BOOKS[65].slug, "revelation");
    }
}
