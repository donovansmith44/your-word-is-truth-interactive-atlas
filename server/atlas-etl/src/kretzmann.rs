//! Parses the vendored commentary pages into verse-anchored units. The source renders two DIFFERENT templates for
//! one idea: an interleaved, sub-verse bold lemma followed by prose, and a whole-verse quote block followed by
//! flowing prose. LEMMA-EXCISION is binding -- a unit's text is the commentator's own prose, never a byte of the
//! quoted verse, which survives only as excised fragments for the conservation check.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};

// The SAME 66-book canonical order and 0-based index the canon table uses, verified against the source's own
// listing chapter count for chapter count, so this pure parser needs no cross-crate canon dependency.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    /// A lemma- or quote-derived unit: it comments on the verse range its own excised text covers, one verse for an
    /// interleaved fragment and a possibly wider range for a quote block.
    Verse,
    /// Prose before the chapter's own first heading: maps to the whole chapter's verse range.
    ChapterIntro,
    /// Prose after a heading but before that section's first lemma or quote: maps to the section's own range.
    PericopeIntro,
}

#[derive(Debug, Clone)]
pub struct KretzUnit {
    /// Stable within one chapter's parse, in document order and 0-based: the ordinal is a fact about THIS parse
    /// rather than about graph construction, so it is computed here and not left to the adapter.
    pub id: String,
    pub book_index: u8,
    pub chapter: u16,
    pub verse_from: u16,
    pub verse_to: u16,
    pub kind: UnitKind,
    pub heading: Option<String>,
    /// The commentator's own prose, LEMMA-EXCISED: never a byte of the quoted verse text.
    pub text: String,
}

/// One excised lemma or quote fragment, kept ONLY for the conservation check and never stored on the graph.
#[derive(Debug, Clone)]
pub struct ExcisedFragment {
    pub book_index: u8,
    pub chapter: u16,
    pub verse: u16,
    /// Document order within one verse -- several fragments per verse are legitimate -- and the concatenation
    /// order the conservation check uses.
    pub order: u32,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct ChapterStats {
    pub footnotes: usize,
    /// A footnote reference that landed INSIDE an excised span rather than in stored prose. Never observed, but
    /// counted rather than assumed impossible: the footnote's text is excised from the comparison target either way.
    pub footnotes_in_lemma: usize,
    /// Fragments where the over-excision guard found a shorter reconciling run than the whole candidate, so real
    /// non-verse content bolded in the same span was recovered to prose instead of destroyed. Counted per occurrence.
    pub over_excisions: usize,
    /// Mid-sentence verse boundaries recognized from the source's own inline "v. N" citation text rather than from
    /// a marker tag. Counted per occurrence.
    pub inline_verse_markers: usize,
    /// One line per disclosed structural anomaly -- a malformed marker, a leading unnumbered lemma with nothing to
    /// fold into, an intro unit whose section has no unit to derive a range from -- named by chapter, never silent.
    pub disclosures: Vec<String>,
}

pub struct ParsedChapter {
    pub book_index: u8,
    pub chapter: u16,
    pub units: Vec<KretzUnit>,
    /// Overall document order, not per verse: the conservation check groups by verse itself.
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
    pub over_excisions: usize,
    pub inline_verse_markers: usize,
    pub disclosures: Vec<String>,
}

pub struct KretzmannCorpus {
    pub chapters: Vec<ParsedChapter>,
    pub stats: CorpusStats,
}

/// The real verse count of one chapter, scanned straight off the canonical verse map -- verse 1, 2, 3 until the
/// first miss, since a complete source has no internal gaps -- rather than from a second hand-maintained table.
fn real_verse_count(kjv_verses: &HashMap<String, String>, book_code: &str, chapter: u16) -> u16 {
    let mut v = 1u16;
    while kjv_verses.contains_key(&format!("{book_code}.{chapter}.{}", v + 1)) {
        v += 1;
    }
    v
}

/// The one filesystem-touching entry point; every other function here is pure `&str`-in / data-out. The canonical
/// verse map is the over-excision guard's source and grounds each chapter's real verse count; the guard compares
/// word content only, so unrestored text is sufficient.
pub fn read_all(root: &Path, kjv_verses: &HashMap<String, String>) -> Result<KretzmannCorpus> {
    let mut chapters = Vec::with_capacity(1189);
    let mut stats = CorpusStats::default();
    for book in BOOKS {
        let code = atlas_core::canon::BOOKS[book.book_index as usize].code;
        for chapter in 1..=book.chapters {
            let path = root.join(book.slug).join(format!("{chapter}.html"));
            let html = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            let chapter_verse_count = real_verse_count(kjv_verses, code, chapter);
            let parsed = parse_chapter(&html, book.book_index, chapter, chapter_verse_count, kjv_verses)
                .with_context(|| format!("parsing {} (book_index {}, chapter {chapter})", path.display(), book.book_index))?;
            stats.pages += 1;
            stats.units += parsed.units.len();
            stats.fragments += parsed.fragments.len();
            stats.footnotes += parsed.stats.footnotes;
            stats.footnotes_in_lemma += parsed.stats.footnotes_in_lemma;
            stats.over_excisions += parsed.stats.over_excisions;
            stats.inline_verse_markers += parsed.stats.inline_verse_markers;
            for d in &parsed.stats.disclosures {
                stats.disclosures.push(format!("{}/{}: {}", book.slug, chapter, d));
            }
            chapters.push(parsed);
        }
    }
    Ok(KretzmannCorpus { chapters, stats })
}

/// Restricts to the page's own article shell, so chrome outside it can never leak a spurious marker match.
fn article_slice(html: &str) -> Result<&str> {
    let start = html.find("<article").context("no <article> tag found -- not a real chapter page")?;
    let gt = html[start..].find('>').map(|p| start + p + 1).context("malformed <article> opening tag")?;
    let end = html[gt..].find("</article>").map(|p| gt + p).context("no closing </article> tag found")?;
    Ok(&html[gt..end])
}

/// Splits the body into its footnote-definition section, if any, and the remaining main content.
fn split_off_footnotes(body: &str) -> (&str, Option<&str>) {
    match body.find(r#"<section data-footnotes"#) {
        None => (body, None),
        Some(start) => {
            let end = body[start..].find("</section>").map(|p| start + p + "</section>".len()).unwrap_or(body.len());
            (&body[..start], Some(&body[start..end]))
        }
    }
}

/// The footnote definitions as `id -> cleaned text`: the backref link removed, every other tag stripped, entities
/// decoded, whitespace collapsed.
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
        // The backref link, a bare arrow inside its own anchor, is the only sub-element besides the footnote's own
        // prose, so it is excised the way any structural, non-prose marker is.
        let no_backref = strip_between(li_body, "<a href=\"#user-content-fnref-", "</a>");
        let text = collapse_ws(&decode_entities(&strip_tags(&no_backref)));
        out.insert(fn_id, text);
        search_from = li_start + li_close_rel + "</li>".len();
    }
    out
}

/// Excises every span from a matching start prefix to its closing anchor: narrowly scoped to the backref link,
/// unlike the generic tag strip that follows.
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

/// A sentinel wrapping a footnote's own text inline, in private-use characters that are guaranteed absent from real
/// source prose, so the later resolve pass finds them exactly.
const FN_SENTINEL_OPEN: char = '\u{E000}';
const FN_SENTINEL_CLOSE: char = '\u{E001}';

/// Replaces every inline footnote reference with a sentinel carrying the looked-up definition's cleaned text,
/// resolved to its final form later, once the surrounding text's role -- stored prose or excised lemma -- is known.
/// A reference naming an id with no definition falls back to a placeholder and is counted.
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

/// Resolves each sentinel to its final bracketed form in stored prose, and silently excises it, counted, inside an
/// excised span: a footnote is never verse content, so folding its text into the comparison target would be a bug.
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

/// The recognized document-order markers.
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

/// Finds the first INLINE verse-boundary marker matching the source's own "v. N" mid-sentence citation shape, a real
/// transcription convention where a boundary is rendered as literal text instead of a tag. A candidate matches only
/// when its number is EXACTLY the verse after the one currently open, which is what tells a genuine boundary from an
/// ordinary backward cross-reference, and a non-alphabetic byte must precede the "v" so book abbreviations never fire.
fn find_inline_verse_marker(text: &str, expected_next: Option<u16>) -> Option<(usize, usize, u16)> {
    let expected = expected_next?;
    let mut search_from = 0usize;
    while let Some(rel) = text[search_from..].find("v. ") {
        let pos = search_from + rel;
        let preceded_by_letter = text[..pos].chars().last().is_some_and(|c| c.is_alphabetic());
        if !preceded_by_letter {
            let after = &text[pos + "v. ".len()..];
            let digit_run: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = digit_run.parse::<u16>() {
                if v == expected {
                    return Some((pos, pos + "v. ".len() + digit_run.len(), v));
                }
            }
        }
        search_from = pos + "v. ".len();
    }
    None
}

/// Splits a span's raw inner HTML at each verse-number marker -- a bare superscript, or an inline citation-shaped
/// one -- into consecutive `(verse, text)` fragments, taking whichever candidate starts first in document order. The
/// first fragment carries no number when the span does not open with a marker, which the caller's own state resolves.
fn split_by_verse_markers(raw: &str) -> (Vec<(Option<u16>, String)>, usize, usize) {
    let mut fragments: Vec<(Option<u16>, String)> = Vec::new();
    let mut anomalies = 0usize;
    let mut inline_markers = 0usize;
    let mut cur_verse: Option<u16> = None;
    let mut cur_text = String::new();
    let mut rest = raw;
    loop {
        let sup_rel = rest.find("<sup");
        let inline_hit = find_inline_verse_marker(rest, cur_verse.map(|v| v + 1));
        let use_inline = match (sup_rel, inline_hit) {
            (None, None) => {
                cur_text.push_str(rest);
                break;
            }
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (Some(sr), Some((ir, ..))) => ir < sr,
        };

        if use_inline {
            let (ir, iend, v) = inline_hit.unwrap();
            cur_text.push_str(&rest[..ir]);
            fragments.push((cur_verse, std::mem::take(&mut cur_text)));
            cur_verse = Some(v);
            inline_markers += 1;
            rest = &rest[iend..];
            continue;
        }

        let rel = sup_rel.unwrap();
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
        // The LEADING digit run only: the corpus sub-letters some markers, a finer split within one verse, and the
        // trailing letter carries no locus meaning of its own -- both "5a" and "5b" target verse 5.
        let digit_run: String = inner.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digit_run.is_empty() {
            fragments.push((cur_verse, std::mem::take(&mut cur_text)));
            cur_verse = digit_run.parse::<u16>().ok();
        } else {
            // No leading digit at all, so not a marker this parser recognizes: disclosed and kept as ordinary text
            // rather than dropped.
            anomalies += 1;
            cur_text.push_str(&rest[rel..after]);
        }
        rest = &rest[after..];
    }
    fragments.push((cur_verse, cur_text));
    // An empty leading fragment, which a span opening exactly on a marker always produces, carries no content and is
    // dropped -- unless it is the only fragment, so a span with neither marker nor text stays visible to the caller.
    let len_is_one = fragments.len() == 1;
    fragments.retain(|(_, t)| !t.trim().is_empty() || len_is_one);
    (fragments, anomalies, inline_markers)
}

/// Extracts a complete, bare verse-number marker that trails a gap with only whitespace after it -- the source's own
/// floating-marker-before-a-span quirk -- returning the remaining gap text and that verse number.
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

/// Cleans an EXCISED fragment: the same pipeline as prose, except that a footnote sentinel is silently excised rather
/// than rendered, and the excision is counted.
fn clean_lemma(raw: &str) -> (String, usize) {
    let (resolved, anomalies) = resolve_footnote_sentinels(raw, true);
    (collapse_ws(&decode_entities(&strip_tags(&resolved))), anomalies)
}

/// Combines the active heading and sub-heading into one display string: the more specific one is prefixed by its
/// parent for context, since a unit carries a single heading field rather than a path.
fn compose_heading(h3: &Option<String>, h4: &Option<String>) -> Option<String> {
    match (h3, h4) {
        (Some(a), Some(b)) => Some(format!("{a}: {b}")),
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (None, None) => None,
    }
}

/// `chapter_verse_count` is the TRUE count from the caller's canonical source, never re-derived from what the
/// commentary happens to cover, because a chapter-intro unit's range must be the real one. An empty canonical map is
/// a graceful, total no-op: every fragment stays lemma in full, so a fixture that does not care can pass an empty one.
pub fn parse_chapter(html: &str, book_index: u8, chapter: u16, chapter_verse_count: u16, kjv_verses: &HashMap<String, String>) -> Result<ParsedChapter> {
    let book_code = atlas_core::canon::BOOKS[book_index as usize].code;
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
    let mut over_excisions = 0usize;
    let mut inline_verse_markers = 0usize;
    // A per-verse byte cursor into that verse's OWN canonical text: a verse split across several fragments must
    // reconcile each one from wherever the prior fragment left off, never from the verse's start again.
    let mut verse_cursor: BTreeMap<u16, usize> = BTreeMap::new();

    let mut h3: Option<String> = None;
    let mut h4: Option<String> = None;
    let mut section: usize = 0;
    let mut current_verse: Option<u16> = None;
    let mut pending_unnumbered: Vec<(usize, String)> = Vec::new();
    let mut open_unit: Option<usize> = None;
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
                    // The floating marker belongs to the NEXT span, not to this gap's prose, so it is stashed by
                    // pre-registering the current verse -- the following span's "no leading marker" branch then
                    // resolves it correctly.
                    flush_prose(&mut pending_prose, open_unit, &mut raw_units, &compose_heading(&h3, &h4), section);
                    current_verse = Some(v);
                    open_unit = None;
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
                let (parts, anomalies, inline_hits) = split_by_verse_markers(raw);
                if anomalies > 0 {
                    disclosures.push(format!("chapter {chapter}: {anomalies} non-digit <sup> marker(s) inside a lemma span, kept as text"));
                }
                if inline_hits > 0 {
                    inline_verse_markers += inline_hits;
                    disclosures.push(format!("chapter {chapter}: {inline_hits} inline \"v. N\" verse-boundary marker(s) recognized inside a lemma span (fix round 2)"));
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
                            None => None,
                        }
                    } else {
                        verse_opt
                    };
                    let heading = compose_heading(&h3, &h4);
                    let unit_idx = raw_units.len();
                    // This string IS the excised lemma candidate and feeds the fragments only. The unit's own text
                    // starts empty and is filled later from the COMMENTARY that follows this span, plus whatever the
                    // over-excision guard recovers.
                    raw_units.push(RawUnit { kind: UnitKind::Verse, heading, verse_from: resolved_verse, verse_to: resolved_verse, text: String::new(), section });
                    open_unit = Some(unit_idx);
                    match resolved_verse {
                        None => pending_unnumbered.push((unit_idx, text)),
                        Some(v) => {
                            current_verse = Some(v);
                            // A fresh numbered marker resolves every fragment still waiting on one FIRST, in their
                            // original document order: a leading unnumbered lemma folds into the first numbered verse
                            // that follows and must keep the lower order, since it was read first.
                            for (p_idx, p_text) in pending_unnumbered.drain(..) {
                                raw_units[p_idx].verse_from = Some(v);
                                raw_units[p_idx].verse_to = Some(v);
                                let (lemma_text, prose_tail) = apply_over_excision_guard(book_code, chapter, v, &p_text, kjv_verses, &mut verse_cursor);
                                if !prose_tail.is_empty() {
                                    over_excisions += 1;
                                    disclosures.push(format!("chapter {chapter} verse {v}: over-excision guard recovered {} prose byte(s) from a leading (superscription-class) lemma", prose_tail.len()));
                                    raw_units[p_idx].text = prose_tail;
                                }
                                if !lemma_text.is_empty() {
                                    fragments.push(ExcisedFragment { book_index, chapter, verse: v, order: frag_order, text: lemma_text });
                                    frag_order += 1;
                                }
                            }
                            let (lemma_text, prose_tail) = apply_over_excision_guard(book_code, chapter, v, &text, kjv_verses, &mut verse_cursor);
                            if !prose_tail.is_empty() {
                                over_excisions += 1;
                                disclosures.push(format!("chapter {chapter} verse {v}: over-excision guard recovered {} prose byte(s) from a lemma span", prose_tail.len()));
                                raw_units[unit_idx].text = prose_tail;
                            }
                            if !lemma_text.is_empty() {
                                fragments.push(ExcisedFragment { book_index, chapter, verse: v, order: frag_order, text: lemma_text });
                                frag_order += 1;
                            }
                        }
                    }
                }
            }
            Segment::Quote(raw) => {
                flush_prose(&mut pending_prose, open_unit, &mut raw_units, &compose_heading(&h3, &h4), section);
                let (parts, anomalies, inline_hits) = split_by_verse_markers(raw);
                if anomalies > 0 {
                    disclosures.push(format!("chapter {chapter}: {anomalies} non-digit <sup> marker(s) inside a quote block, kept as text"));
                }
                if inline_hits > 0 {
                    inline_verse_markers += inline_hits;
                    disclosures.push(format!("chapter {chapter}: {inline_hits} inline \"v. N\" verse-boundary marker(s) recognized inside a quote block (fix round 2)"));
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
                    let (lemma_text, prose_tail) = apply_over_excision_guard(book_code, chapter, v, &text, kjv_verses, &mut verse_cursor);
                    if !prose_tail.is_empty() {
                        over_excisions += 1;
                        disclosures.push(format!("chapter {chapter} verse {v}: over-excision guard recovered {} prose byte(s) from a quote-block fragment", prose_tail.len()));
                        if !raw_units[unit_idx].text.is_empty() {
                            raw_units[unit_idx].text.push(' ');
                        }
                        raw_units[unit_idx].text.push_str(&prose_tail);
                    }
                    if !lemma_text.is_empty() {
                        fragments.push(ExcisedFragment { book_index, chapter, verse: v, order: frag_order, text: lemma_text });
                        frag_order += 1;
                    }
                }
                if let (Some(a), Some(b)) = (min_v, max_v) {
                    raw_units[unit_idx].verse_from = Some(a);
                    raw_units[unit_idx].verse_to = Some(b);
                    open_unit = Some(unit_idx);
                } else {
                    // A quote block with no resolvable verse is disclosed per fragment above; the empty shell unit is
                    // dropped rather than emitted without a range.
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

    // Each intro unit's range is its SECTION's min..max verse among that section's own verse-kind units, while a
    // chapter intro instead takes the true full-chapter range.
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
                _ => continue,
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
        stats: ChapterStats { footnotes: footnote_count, footnotes_in_lemma, over_excisions, inline_verse_markers, disclosures },
    })
}

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

/// Decodes the entity set observed across the real corpus plus generic numeric escapes.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviationClass {
    /// Exact byte match against the canonical (restored) verse text.
    Exact,
    /// A DISCLOSED EQUIVALENCE: the concatenation differs from canonical only by case and punctuation. It covers both
    /// observed classes -- the reverential case convention, and the boundary punctuation the digital edition
    /// introduces by rendering each fragment as its own sentence. Mechanical and symmetric: the underlying word
    /// sequence must still match exactly, so it can never mask a content difference.
    MechanicalCaseAndPunct,
    /// A THIRD disclosed class, found by mining the real corpus: the digital edition systematically modernizes
    /// archaic spelling. The variant table is curated and auditable -- never a fuzzy or edit-distance guess, which
    /// could silently equate two different words -- and applies on top of case and punctuation normalization, so this
    /// class still requires the same word sequence, position for position.
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
    /// A canonical verse with no excised fragments at all: lawful, since the commentary summarizes some spans, and so
    /// asserted and disclosed rather than treated as an error.
    pub uncovered: Vec<(u8, u16, u16)>,
}

/// The mechanical comparison key: lowercased, then every punctuation, quote and dash character replaced with a SPACE
/// rather than deleted -- deleting a hyphen would merge two words into one that could never equal the canonical pair
/// -- then whitespace-collapsed. Applied identically to both sides, so a genuine word difference still fails.
fn mechanical_key(s: &str) -> String {
    let spaced: String = s.chars().map(|c| if is_word_separator(c) { ' ' } else { c }).collect();
    collapse_ws(&spaced.to_lowercase())
}

/// The shared word-boundary predicate, defined ONCE and used by both the comparison key and the guard's own
/// tokenizer, so the two can never drift apart on what counts as a word.
fn is_word_separator(c: char) -> bool {
    c.is_whitespace() || c.is_ascii_punctuation() || matches!(c, '\u{2013}' | '\u{2014}' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{2026}')
}

/// Curated (modernized -> canonical) spelling pairs, built exclusively from real corpus mismatches occurring at least
/// twice, each manually confirmed to be the SAME word differently spelled. Word-choice pairs, a meaning change, and a
/// genuine canon-internal name-form variance were deliberately excluded and stay disclosed mismatches: collapsing any
/// of them here could mask a real difference.
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

/// Applies the spelling table word by word on top of the case-and-punctuation key.
fn spelling_key(s: &str) -> String {
    mechanical_key(s).split(' ').map(spelling_normalize_word).collect::<Vec<_>>().join(" ")
}

/// One already-lowercased word through the spelling table, unchanged when absent: factored out so the guard's own
/// tokenizer uses the IDENTICAL per-word normalization rather than a second, driftable copy.
fn spelling_normalize_word(w: &str) -> &str {
    SPELLING_VARIANTS.iter().find(|&&(american, _)| american == w).map(|&(_, british)| british).unwrap_or(w)
}

// THE OVER-EXCISION GUARD: a bolded run occasionally carries the commentator's own prose in the SAME span as genuine
// verse text, and the two real instances refute any prefix-only or suffix-only split -- one is prose then verse, the
// other verse, then an aside, then verse again. So a fragment's words are reconciled against the verse's remaining
// canonical words by recursive LONGEST-COMMON-BLOCK matching, never a plain longest common subsequence: an LCS may
// match a coincidental word repeat inside a prose block and steal a position from the genuine clause, which really did
// tear two real spans apart. Anchoring on the single longest contiguous run first leaves nothing for such a repeat to
// steal. Whatever stays unmatched returns to stored prose.

/// Below this many words, an unmatched run is treated as RETAINED lemma rather than recovered prose: a single
/// substituted or dropped word would otherwise be ripped out of the verse text as if it were commentary.
const MIN_PROSE_RUN_WORDS: usize = 3;

/// Tokenizes into `(normalized word, start, end)` triples under the identical normalization the comparison keys use,
/// but retaining each token's byte span into the ORIGINAL text -- which is what lets verbatim, boundary-safe runs be
/// sliced back out once the alignment has classified each word.
fn tokenize_words_with_spans(s: &str) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut word_start: Option<usize> = None;
    let mut word_end = 0usize;
    for (i, c) in s.char_indices() {
        let end = i + c.len_utf8();
        if is_word_separator(c) {
            if let Some(start) = word_start.take() {
                out.push((spelling_normalize_word(&s[start..word_end].to_lowercase()).to_string(), start, word_end));
            }
        } else {
            if word_start.is_none() {
                word_start = Some(i);
            }
            word_end = end;
        }
    }
    if let Some(start) = word_start {
        out.push((spelling_normalize_word(&s[start..word_end].to_lowercase()).to_string(), start, word_end));
    }
    out
}

/// The SINGLE LONGEST contiguous run of words common to both sides -- a longest common substring at word level, not a
/// subsequence. Ties resolve to whichever the scan reaches first, which is immaterial: the caller applies this to
/// strictly shrinking sub-ranges either way.
fn longest_common_block(frag: &[(String, usize, usize)], canon: &[(String, usize, usize)]) -> Option<(usize, usize, usize)> {
    let n = frag.len();
    let m = canon.len();
    let mut same = vec![vec![0usize; m + 1]; n + 1];
    let mut best: (usize, usize, usize) = (0, 0, 0);
    for i in 1..=n {
        for j in 1..=m {
            if frag[i - 1].0 == canon[j - 1].0 {
                same[i][j] = same[i - 1][j - 1] + 1;
                if same[i][j] > best.2 {
                    best = (i - same[i][j], j - same[i][j], same[i][j]);
                }
            }
        }
    }
    if best.2 == 0 {
        None
    } else {
        Some(best)
    }
}

/// Recursively partitions both sides by anchoring on the longest common block, then recursing on the piece strictly
/// before it and the piece strictly after. `frag_base` offsets into the FULL fragment word list, since the slices
/// shrink with each call. Returns the largest canonical byte offset any block consumed, which advances the caller's
/// per-verse cursor.
fn align_recursive(frag: &[(String, usize, usize)], canon: &[(String, usize, usize)], frag_base: usize, is_matched: &mut [bool]) -> usize {
    if frag.is_empty() || canon.is_empty() {
        return 0;
    }
    let Some((fi, ci, len)) = longest_common_block(frag, canon) else {
        return 0;
    };
    for k in 0..len {
        is_matched[frag_base + fi + k] = true;
    }
    let left_end = align_recursive(&frag[..fi], &canon[..ci], frag_base, is_matched);
    let this_end = canon[ci + len - 1].2;
    let right_end = align_recursive(&frag[fi + len..], &canon[ci + len..], frag_base + fi + len, is_matched);
    left_end.max(this_end).max(right_end)
}

/// Reconciles one fragment against its verse's REMAINING canonical text -- the per-verse cursor exists because a verse
/// split across several fragments must resume where the prior one left off -- and returns the retained lemma and the
/// recovered prose, the latter empty in the common fully-matched case. A canonical entry missing for that verse is a
/// graceful no-op: the whole fragment stays lemma, so the guard never requires canonical data to keep working.
fn apply_over_excision_guard(book_code: &str, chapter: u16, v: u16, raw_text: &str, kjv_verses: &HashMap<String, String>, verse_cursor: &mut BTreeMap<u16, usize>) -> (String, String) {
    let Some(canonical) = kjv_verses.get(&format!("{book_code}.{chapter}.{v}")) else {
        return (raw_text.to_string(), String::new());
    };
    let cursor = verse_cursor.get(&v).copied().unwrap_or(0).min(canonical.len());
    let remaining = &canonical[cursor..];

    let frag_words = tokenize_words_with_spans(raw_text);
    if frag_words.is_empty() {
        return (raw_text.to_string(), String::new());
    }
    let canon_words = tokenize_words_with_spans(remaining);
    let mut is_matched = vec![false; frag_words.len()];
    let last_canon_end = align_recursive(&frag_words, &canon_words, 0, &mut is_matched);
    verse_cursor.insert(v, cursor + last_canon_end);

    // Unmatched runs shorter than the threshold are merged back into retained lemma in one pass.
    let mut i = 0;
    while i < is_matched.len() {
        if is_matched[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < is_matched.len() && !is_matched[i] {
            i += 1;
        }
        if i - start < MIN_PROSE_RUN_WORDS {
            for slot in &mut is_matched[start..i] {
                *slot = true;
            }
        }
    }

    if is_matched.iter().all(|&m| m) {
        return (raw_text.to_string(), String::new());
    }

    let mut lemma_parts: Vec<&str> = Vec::new();
    let mut prose_parts: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < frag_words.len() {
        let start = i;
        let matched = is_matched[i];
        while i < frag_words.len() && is_matched[i] == matched {
            i += 1;
        }
        let run_start = frag_words[start].1;
        // A recovered run extends forward to the NEXT run's first word rather than stopping at its own last word, so
        // punctuation between two runs stays attached to the word it follows instead of vanishing into the gap.
        let run_end = if i < frag_words.len() { frag_words[i].1 } else { raw_text.len() };
        let piece = raw_text[run_start..run_end].trim();
        if matched {
            lemma_parts.push(piece);
        } else {
            prose_parts.push(piece);
        }
    }
    (lemma_parts.join(" "), prose_parts.join(" "))
}

/// Per verse, the excised fragments concatenate in order and must equal the canonical text for that verse, exactly or
/// under one of the disclosed mechanical equivalences. Pure: the caller supplies the restored canonical map.
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

/// One piece of the composed reading view, kept as a TYPED segment rather than a flat string with an embedded marker,
/// so stripping the comment blocks is exact filtering and never string scanning a stray byte could defeat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadingViewSegment {
    Verse(String),
    Comment(String),
}

/// For every verse in canonical spine order, that verse's own text followed by the stored prose of every unit whose
/// range covers it, in document order -- the same range-covers-verse question each commentary row encodes. A verse
/// with no covering unit contributes only its own segment, which is lawful. Deliberately the shape a real reading
/// view would use, so the law it serves proves real logic rather than a stand-in.
pub fn compose_reading_view(canonical: &BTreeMap<(u8, u16, u16), String>, corpus: &KretzmannCorpus) -> Vec<ReadingViewSegment> {
    let mut by_chapter: HashMap<(u8, u16), &ParsedChapter> = HashMap::new();
    for chapter in &corpus.chapters {
        by_chapter.insert((chapter.book_index, chapter.chapter), chapter);
    }

    let mut out = Vec::with_capacity(canonical.len() * 2);
    for (&(book_index, chapter, verse), text) in canonical {
        out.push(ReadingViewSegment::Verse(text.clone()));
        if let Some(parsed) = by_chapter.get(&(book_index, chapter)) {
            for unit in &parsed.units {
                if unit.verse_from <= verse && verse <= unit.verse_to {
                    out.push(ReadingViewSegment::Comment(unit.text.clone()));
                }
            }
        }
    }
    out
}

/// Strips every comment segment and byte-concatenates what remains, EXACTLY, with no equivalence tiers: this guards
/// the reading-view construction -- spine coverage, verse-text mutation, compose ordering -- rather than parse fidelity.
pub fn strip_comment_blocks(segments: &[ReadingViewSegment]) -> String {
    segments
        .iter()
        .filter_map(|s| match s {
            ReadingViewSegment::Verse(t) => Some(t.as_str()),
            ReadingViewSegment::Comment(_) => None,
        })
        .collect()
}

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

/// Scans stored prose for verbatim dating clauses: PARSING ONLY, never interpretation. Both real orderings of a
/// B.C./A.D. year marker are matched, an adjacent "about" sets the approximate flag, and Anno Mundi markers are
/// matched defensively though the real corpus has none. Reign-year formulas are deliberately NOT extracted: the row
/// has no field for them, so forcing one would fabricate, and the caller counts that class instead.
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
        // Backward: a digit run immediately before the marker, with one optional space -- the dominant convention for
        // both eras.
        let before = &text[..start];
        let before_trimmed = before.trim_end_matches(' ');
        let digits_end = before_trimmed.len();
        let digits_start = before_trimmed.len() - before_trimmed.chars().rev().take_while(|c| c.is_ascii_digit()).map(char::len_utf8).sum::<usize>();
        if digits_start < digits_end && before_trimmed.len() != before.len() {
            let year_str = &before_trimmed[digits_start..digits_end];
            if let Ok(year) = year_str.parse::<u32>() {
                // `ends_with` is char-boundary-safe on any prefix slice, unlike a fixed byte offset back from the
                // digit run, which could land mid-character on non-ASCII prose.
                let prefix = &before_trimmed[..digits_start];
                let approx = prefix.ends_with("about ");
                let clause_start = if approx { find_about_start(before_trimmed, digits_start) } else { digits_start };
                out.push(DateClause { verbatim: text[clause_start..end].to_string(), calendar: cal, year, approx });
                continue;
            }
        }
        // Forward: a digit run immediately after the marker's own trailing space, the secondary real convention.
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

    const GEN_1_EXCERPT: &str = r#"<h3 id="the-creation-of-the-world">The Creation of the World.</h3>
<h4 id="the-creation-of-chaos-and-light">The Creation of Chaos and Light</h4>
<p><strong><sup>1</sup>In the beginning God created the heaven and the earth.</strong> In the beginning, cp. John 1, 1. <strong><sup>2</sup>And the earth was without form and void.</strong> The material substance. <strong>And darkness was upon the face of the deep.</strong> There was, as yet, no elemental light. <strong>And the Spirit of God moved upon the face of the waters.</strong> The third person. <strong><sup>3</sup>And God said, Let there be light; and there was light.</strong> God spoke.</p>"#;

    #[test]
    fn gen_1_2_splits_into_three_fragments_and_the_prose_between_attaches_to_each() {
        let parsed = parse_chapter(&wrap_article(GEN_1_EXCERPT), 0, 1, 31, &HashMap::new()).unwrap();
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

    const LEV_21_14_V7_CITATION_EXCERPT: &str = "or an harlot, these shall he not take, v. 7; but he shall take a virgin of his own people to wife,";

    #[test]
    fn find_inline_verse_marker_correctly_ignores_lev_21_14s_own_real_backward_v_7_citation() {
        assert_eq!(
            find_inline_verse_marker(LEV_21_14_V7_CITATION_EXCERPT, Some(15)),
            None,
            "a backward 'v. 7' citation, while verse 14 is open (expecting verse 15 next), must NEVER be treated as a verse boundary"
        );

        let (start, end, verse) = find_inline_verse_marker(LEV_21_14_V7_CITATION_EXCERPT, Some(7)).expect("the identical 'v. 7' text must match when 7 genuinely is the expected next verse");
        assert_eq!(verse, 7);
        assert_eq!(&LEV_21_14_V7_CITATION_EXCERPT[start..end], "v. 7");
    }

    const PSA_110_EXCERPT: &str = r#"<h3 id="a">A Psalm of Christ.</h3>
<p><strong>A psalm of David,</strong> altogether prophetic. <strong><sup>1</sup>The Lord said unto my Lord,</strong> literally. <strong>Sit Thou at My right hand,</strong> emblem. <strong>until I make Thine enemies Thy footstool.</strong> The Messiah.</p>"#;

    #[test]
    fn psa_110_1_leading_superscription_folds_into_verse_1_and_matches_canonical_under_the_mechanical_class() {
        let parsed = parse_chapter(&wrap_article(PSA_110_EXCERPT), 18, 110, 7, &HashMap::new()).unwrap();
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
        let parsed = parse_chapter(&wrap_article(JHN_3_TYPE_B_EXCERPT), 42, 3, 36, &HashMap::new()).unwrap();
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
        let parsed = parse_chapter(&wrap_article(PSA_1_INTRO_EXCERPT), 18, 1, 6, &HashMap::new()).unwrap();
        let intro = parsed.units.iter().find(|u| u.kind == UnitKind::PericopeIntro).expect("a pericope-intro unit must exist");
        assert_eq!(intro.text, "All men are sinners: all have sinned and come short of the glory of God.");
        assert_eq!((intro.verse_from, intro.verse_to), (1, 1), "the section's own real range (this fixture's lemmas only reach verse 1)");
    }

    const CHAPTER_INTRO_EXCERPT: &str = r#"<p>General orientation before any heading appears at all.</p>
<h3 id="a">First Pericope.</h3>
<p><strong><sup>1</sup>Text.</strong> Commentary.</p>"#;

    #[test]
    fn chapter_intro_prose_before_any_heading_maps_to_the_true_full_chapter_range() {
        let parsed = parse_chapter(&wrap_article(CHAPTER_INTRO_EXCERPT), 0, 5, 27, &HashMap::new()).unwrap();
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
        let parsed = parse_chapter(&html, 0, 1, 1, &HashMap::new()).unwrap();
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
    fn kretz_accept_2_strips_to_exactly_the_canonical_concatenation_on_a_synthetic_corpus() {
        let mut canonical = BTreeMap::new();
        canonical.insert((0u8, 1u16, 1u16), "In the beginning God created the heaven and the earth.".to_string());
        canonical.insert((0u8, 1u16, 2u16), "And the earth was without form and void.".to_string());
        canonical.insert((0u8, 2u16, 1u16), "Thus the heavens and the earth were finished.".to_string());

        let corpus = KretzmannCorpus {
            chapters: vec![ParsedChapter {
                book_index: 0,
                chapter: 1,
                units: vec![
                    KretzUnit { id: "kretzmann/0.1.0".to_string(), book_index: 0, chapter: 1, verse_from: 1, verse_to: 2, kind: UnitKind::Verse, heading: None, text: "A comment spanning both verses.".to_string() },
                    KretzUnit { id: "kretzmann/0.1.1".to_string(), book_index: 0, chapter: 1, verse_from: 2, verse_to: 2, kind: UnitKind::Verse, heading: None, text: "A second comment on verse 2 alone.".to_string() },
                ],
                fragments: vec![],
                stats: ChapterStats::default(),
            }],
            stats: CorpusStats::default(),
        };

        let segments = compose_reading_view(&canonical, &corpus);
        assert_eq!(
            segments,
            vec![
                ReadingViewSegment::Verse("In the beginning God created the heaven and the earth.".to_string()),
                ReadingViewSegment::Comment("A comment spanning both verses.".to_string()),
                ReadingViewSegment::Verse("And the earth was without form and void.".to_string()),
                ReadingViewSegment::Comment("A comment spanning both verses.".to_string()),
                ReadingViewSegment::Comment("A second comment on verse 2 alone.".to_string()),
                ReadingViewSegment::Verse("Thus the heavens and the earth were finished.".to_string()),
            ]
        );

        let stripped = strip_comment_blocks(&segments);
        let whole_bible: String = canonical.values().map(|s| s.as_str()).collect();
        assert_eq!(stripped, whole_bible, "stripping every Comment segment must recover EXACTLY the canonical concatenation, no residual");
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
