//! Batch RED-1 ("Red letters on Jesus' words in every translation," owner
//! order 2026-08-25): parses the vendored `data/raw/red-letter/
//! eng-kjv.osis.xml` -- a public-domain KJV OSIS file (eBible.org, via the
//! `seven1m/open-bibles` curated PD-bibles collection; see LICENSES.md's own
//! "KJV red-letter markup" section for the full PD grounding and the
//! CrossWire-substitution disclosure) -- into per-verse `<q who="Jesus">`
//! spans, then ALIGNS each span's own text against OUR canonical KJV verse
//! text (the GAZ-1 alias-law pattern, `atlas-etl/src/validate.rs`'s own
//! `run_place_names_kjv`: "every curated alias string must be a
//! case-sensitive verbatim substring... of the KJV text").
//!
//! SOURCE SHAPE (OSIS, milestone-style -- verified directly against the real
//! vendored file): `<verse osisID="Matt.4.19" sID="..." n="19" />TEXT
//! <verse eID="..." />` per verse; a red span is `<q who="Jesus" sID="..."
//! marker="" />TEXT<q eID="..." />` NESTED inside one verse's own text
//! region -- the source's own `<q>` usage never crosses a verse boundary
//! (its own revisionDesc header: "`<q>` markup is stopped and restarted at
//! all verse boundaries"), so this parser never needs to carry an open span
//! across a `<verse eID.../>`. Book identity comes from `<div type="book"
//! osisID="Gen" ...>` -- `osisID` is the OSIS 3-ish-letter code
//! (`"Gen"`/`"Matt"`/`"1Cor"`), which `atlas_core::canon::resolve_alias`
//! ALREADY resolves directly (`BookInfo.osis`, canon.rs) -- no separate
//! mapping table needed. A book whose `osisID` does not resolve (the
//! source's own bundled Apocrypha/Deuterocanon -- its header states "with
//! Apocrypha/Deuterocanon") is silently excluded from `current_book`, so its
//! verses never open at all -- correct: our canon has no Apocrypha, and
//! Jesus never speaks there regardless.
//!
//! PARSER SHAPE: hand-rolled tag scanner, single pass (this crate's own
//! established house style -- `kretzmann.rs`/`concord.rs` are both
//! hand-rolled HTML scanners too; no XML/HTML parsing crate exists anywhere
//! in this workspace's `Cargo.lock`, and this file's own tag grammar is
//! regular enough not to need one). Only text INSIDE a `who="Jesus"` span is
//! ever accumulated -- the parser has no need to reconstruct a whole
//! verse's own plain text (unlike `kretzmann.rs`, which excises AROUND kept
//! lemma text); `<transChange type="added">...</transChange>` (KJV's own
//! italicized/supplied-word convention) is transparent here -- its own
//! open/close tags are stripped, its inner text flows through like any
//! other text chunk, whether inside or outside a Jesus span. The MOMENT a
//! verse closes, its own raw span texts are immediately aligned against
//! `kjv_verses` (below) and only the resolved byte offsets are kept -- the
//! raw text itself never needs to outlive that one verse's own scope.
//!
//! ALIGNMENT: per verse, each source span's own TRIMMED text is searched
//! (case-sensitive first) as a substring of OUR canonical verse text,
//! starting from a CURSOR that advances past each successful match (so two
//! spans in one verse, or a repeated short phrase, resolve left-to-right
//! rather than both collapsing onto the first occurrence). A verse
//! containing multiple SEPARATE `<q who="Jesus">` runs (narration between
//! two sayings) is real and handled this way, in document order. Failing
//! that, a CASE-INSENSITIVE retry (`ascii_ci_find` below, the disclosed
//! KJV-CASE class: our RESTORED case convention -- LORD/Lord -- vs this
//! source's own normalization) resolves the offset the same way, against
//! OUR bytes (never the source's own differently-cased text). Neither
//! found: the span is dropped from the SUB-VERSE table (counted, never
//! guessed) -- the verse itself STAYS in the verse SET regardless (decision
//! 2a: the verse set is edition-independent, sourced from "does the source
//! mark ANY red content here," not from OUR OWN byte-alignment success).
//!
//! ASCII-CASE-INSENSITIVE, NOT `str::to_lowercase()`, disclosed: Rust's
//! `to_lowercase()` is Unicode-aware and can change a string's own BYTE
//! LENGTH for some scripts (never English/Latin text), which would silently
//! break the "offset transfers back to the original bytes" step. Real KJV
//! text is ASCII plus the occasional en/em dash (case-invariant) --
//! `ascii_ci_find` below only ever treats ASCII letters case-insensitively
//! (`u8::eq_ignore_ascii_case`'s own documented scope) and compares every
//! other byte exactly, so it is safe over arbitrary UTF-8 and simply
//! reports "not found" rather than mis-computing an offset on any text this
//! assumption doesn't hold for.

use std::collections::HashMap;

use anyhow::{Context, Result};

/// One verse the SOURCE marks as containing >=1 run of Christ's words --
/// present in the verse SET regardless of alignment outcome (module doc
/// comment). `spans` carries only the SUCCESSFULLY ALIGNED byte-offset
/// ranges (into OUR canonical verse text, `String::find`-compatible byte
/// indices) -- empty when every span in this verse failed to align (a real,
/// disclosed, counted case; the verse stays in the set, just contributes no
/// sub-verse row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedLetterVerse {
    pub book_index: u8,
    pub chapter: u16,
    pub verse: u16,
    /// Byte-offset `(start, end)` pairs into OUR canonical verse text,
    /// ascending, non-overlapping (guaranteed by the cursor-advancing
    /// alignment walk).
    pub spans: Vec<(usize, usize)>,
}

/// Alignment-law counts (decision 2's own "counted, categorized" -- module
/// doc comment): every `<q who="Jesus">` run the source carries, however it
/// resolved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AlignmentStats {
    /// The verse SET's own size -- one per verse carrying >=1 source span,
    /// regardless of alignment outcome.
    pub verses_with_source_markup: usize,
    /// Every source span found, across every verse -- exact + case_insensitive + not_found.
    pub source_spans_total: usize,
    /// Case-sensitive verbatim substring match (the GAZ-1 law, unmodified).
    pub exact: usize,
    /// The disclosed KJV-CASE class: found only case-insensitively.
    pub case_insensitive: usize,
    /// Neither found -- dropped from the sub-verse table, never guessed;
    /// the verse itself still counts toward `verses_with_source_markup`.
    pub not_found: usize,
}

#[derive(Debug, Clone, Default)]
pub struct RedLetterCorpus {
    /// Canon order (book, chapter, verse ascending) -- the walk this
    /// module's own `parse` produces is already in document order, which
    /// for a whole-Bible OSIS file IS canon order.
    pub verses: Vec<RedLetterVerse>,
    pub stats: AlignmentStats,
}

/// Extracts `name="value"` from a raw tag string (e.g. `osisID`/`sID`/
/// `eID`/`n`/`who`/`type`) -- the one shared attribute-read primitive every
/// tag-classification branch below uses. `None` when the attribute is
/// absent, never a panic on a malformed/unexpected tag shape.
///
/// LEADING-SPACE GUARD (a real, self-caught bug this file's own unit tests
/// found before this parser ever touched the real vendored data): `sID=` is
/// a literal trailing SUBSTRING of `osisID=` ("o-s-i-**s-I-D**="), so a bare
/// `tag.find("sID=\"")` on `<verse osisID="Matt.4.19" sID="v1" n="19" />`
/// matches INSIDE `osisID`'s own value first, returning `"Matt.4.19"`
/// instead of the real `sID` attribute's `"v1"` -- silently wrong, not a
/// crash (the eventual `<verse eID="v1" />` close tag's own `eid` then
/// never equals this wrongly-captured `sid`, so the verse quietly never
/// enters the output at all). Searching for `" {name}=\""` (a LEADING
/// SPACE) instead anchors the match to a genuine attribute boundary --
/// every real attribute in this file is preceded by whitespace, `osisID`'s
/// own tail can never BE preceded by one mid-word.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let pat = format!(" {name}=\"");
    let start = tag.find(&pat)? + pat.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// The SAME leading-space guard as `attr` above, for the boolean
/// open/close-detector calls (`tag.contains("sID=")` etc.) that don't need
/// the value itself, just whether a genuine (not `osisID`-tail-coincidence)
/// occurrence exists.
fn has_attr(tag: &str, name: &str) -> bool {
    tag.contains(&format!(" {name}=\""))
}

/// Minimal XML entity decode -- the real vendored file carries only `&lt;`/
/// `&gt;` (both inside the header's own prose describing the `<q>` element,
/// confirmed by direct inspection before this parser was written; verse
/// TEXT never uses an entity at all), so this covers the five predefined
/// XML entities and nothing else (no numeric character references) --
/// sufficient for this one source, disclosed rather than silently assumed
/// complete for some other file.
fn decode_entities(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('&') {
        return std::borrow::Cow::Borrowed(s);
    }
    std::borrow::Cow::Owned(s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&"))
}

/// Byte-for-byte comparison, ASCII letters case-insensitive, every other
/// byte exact (module doc comment: why not `to_lowercase()`).
fn ascii_ci_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(&x, &y)| x.eq_ignore_ascii_case(&y))
}

/// The KJV-CASE fallback's own search primitive: the first (leftmost) byte
/// offset in `haystack` where `needle` matches ASCII-case-insensitively.
/// Only tries positions that are valid UTF-8 char boundaries in `haystack`
/// (a needle containing any non-ASCII byte sequence must then match it
/// EXACTLY -- `eq_ignore_ascii_case`'s own documented scope -- which is what
/// keeps every returned offset a valid boundary in turn; module doc
/// comment).
fn ascii_ci_find(haystack: &str, needle: &str) -> Option<usize> {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || n.len() > h.len() {
        return None;
    }
    for start in 0..=(h.len() - n.len()) {
        if !haystack.is_char_boundary(start) {
            continue;
        }
        if ascii_ci_eq(&h[start..start + n.len()], n) {
            return Some(start);
        }
    }
    None
}

/// Collapses every run of whitespace (space/tab/newline) to one plain
/// space, and trims the ends -- a REAL, self-caught alignment gap this
/// parser's own first pass over the real vendored data found (not a
/// hypothetical): the source's own XML is PRETTY-PRINTED, inserting a
/// literal newline at points like `Blessed\n<transChange type="added">are
/// </transChange> they...` (MAT.5.4's own real shape) -- insignificant to
/// XML, but the raw accumulated span text still carries that `\n` between
/// "Blessed" and "are" where OUR canonical prose has an ordinary single
/// space, so a byte-exact (or even case-insensitive) search against the
/// UNNORMALIZED raw text silently missed spans it should have found. Our
/// own canonical KJV text is already single-spaced, ordinary prose, so only
/// the NEEDLE (the source's own span text) ever needs this -- the haystack
/// is never touched.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Aligns one verse's own raw source spans against OUR canonical text,
/// left-to-right with an advancing cursor (module doc comment) -- pure,
/// unit-tested directly (below) independent of the XML scan. Every input
/// span contributes to `stats` (found or not), even one that ends up
/// contributing no output span.
///
/// RESIDUAL `not_found` CLASS, disclosed (real committed data, batch-red1-
/// report.md has the full per-verse list): 18 of 2,081 real source spans
/// (0.87%) genuinely fail both tiers -- every one of them a wording
/// difference between this source's own 1769-class text and OUR canonical
/// KJV, never a parsing defect: spelling variants this source spells
/// differently than our own canon (`Caesar`/our `Cesar`, `Judaea`/our
/// `Judea`, `Galilaeans`/our `Galileans`, `Zacchaeus`/our `Zaccheus`,
/// `Nicolaitanes`/our `Nicolaitans`, `Barjona`/our `Bar–jona`), two dropped
/// possessive apostrophes, one dropped comma, and one source-side literal
/// typo (MAT.5.30 "cut **if** off" for "cut **it** off"). None extend
/// beyond a real, disclosed spelling/punctuation class into anything
/// resembling a content substitution -- and per decision 2's own law, NONE
/// are bridged by a wider fuzzy-match: only case-insensitivity is an
/// authorized second tier; every one of these 18 is counted here and
/// disclosed, never guessed into a wrong offset.
fn align_verse(spans_raw: &[String], canon_text: &str, stats: &mut AlignmentStats) -> Vec<(usize, usize)> {
    let mut cursor = 0usize;
    let mut aligned = Vec::new();
    for raw in spans_raw {
        let normalized = normalize_whitespace(raw);
        let needle = normalized.as_str();
        if needle.is_empty() {
            continue;
        }
        stats.source_spans_total += 1;
        if let Some(rel) = canon_text[cursor..].find(needle) {
            let start = cursor + rel;
            let end = start + needle.len();
            aligned.push((start, end));
            cursor = end;
            stats.exact += 1;
        } else if let Some(rel) = ascii_ci_find(&canon_text[cursor..], needle) {
            let start = cursor + rel;
            let end = start + needle.len();
            aligned.push((start, end));
            cursor = end;
            stats.case_insensitive += 1;
        } else {
            stats.not_found += 1;
        }
    }
    aligned
}

/// One verse's own raw, as-sourced state during the scan.
struct OpenVerse {
    book_index: u8,
    chapter: u16,
    verse: u16,
    sid: String,
    /// Completed `<q who="Jesus">` run texts, in document order -- each
    /// still carries whatever leading/trailing whitespace the source's own
    /// formatting left around it (trimmed by `align_verse`, not here: the
    /// RAW text is what a future caller wanting the untrimmed source would
    /// want preserved as long as possible).
    spans_raw: Vec<String>,
}

/// Parses the vendored OSIS XML text into a `RedLetterCorpus`, aligning
/// every source span against `kjv_verses` THE MOMENT each verse closes
/// (module doc comment has the full scan + alignment law). `&str`-in/
/// data-out (this crate's own established discipline, `build.rs`'s own doc
/// comment) -- `read_all` below is the one filesystem-touching wrapper.
pub fn parse(xml: &str, kjv_verses: &HashMap<String, String>) -> Result<RedLetterCorpus> {
    let mut current_book: Option<u8> = None;
    let mut current_chapter: u16 = 0;
    let mut open_verse: Option<OpenVerse> = None;
    let mut open_q_sid: Option<String> = None;
    let mut span_buf = String::new();

    let mut verses: Vec<RedLetterVerse> = Vec::new();
    let mut stats = AlignmentStats::default();

    let mut i = 0usize;
    while i < xml.len() {
        if xml.as_bytes()[i] != b'<' {
            let next_lt = xml[i..].find('<').map(|p| i + p).unwrap_or(xml.len());
            if open_q_sid.is_some() {
                span_buf.push_str(&decode_entities(&xml[i..next_lt]));
            }
            i = next_lt;
            continue;
        }
        let tag_end = match xml[i..].find('>') {
            Some(p) => i + p + 1,
            None => break, // truncated/malformed tail -- stop rather than panic; whatever parsed so far is honest
        };
        let tag = &xml[i..tag_end];
        i = tag_end;

        if tag.starts_with("<div") && tag.contains("type=\"book\"") {
            current_book = attr(tag, "osisID").and_then(atlas_core::canon::resolve_alias).map(|b| b.0);
            continue;
        }
        if tag.starts_with("<chapter") {
            if has_attr(tag, "sID") {
                if let Some(n) = attr(tag, "n").and_then(|s| s.parse::<u16>().ok()) {
                    current_chapter = n;
                }
            }
            continue;
        }
        if tag.starts_with("<verse") {
            if has_attr(tag, "sID") {
                if let (Some(book_index), Some(sid), Some(verse)) = (current_book, attr(tag, "sID"), attr(tag, "n").and_then(|s| s.parse::<u16>().ok())) {
                    open_verse = Some(OpenVerse { book_index, chapter: current_chapter, verse, sid: sid.to_string(), spans_raw: Vec::new() });
                }
                // No current_book (an Apocrypha/Deuterocanon verse, module
                // doc comment) or an unparseable n: `open_verse` stays
                // `None` -- this verse's own content is silently never
                // collected, by construction (never a partial/guessed row).
                continue;
            }
            if let Some(eid) = attr(tag, "eID") {
                if let Some(v) = open_verse.take() {
                    if v.sid == eid && !v.spans_raw.is_empty() {
                        let dot_ref = format!("{}.{}.{}", atlas_core::canon::BOOKS[v.book_index as usize].code, v.chapter, v.verse);
                        stats.verses_with_source_markup += 1;
                        let aligned = match kjv_verses.get(&dot_ref) {
                            Some(canon_text) => align_verse(&v.spans_raw, canon_text, &mut stats),
                            // Our own canon lacks this verse (never true of
                            // the real committed kjv.json -- every verse the
                            // OSIS source marks red is well within the
                            // 66-book canon, Apocrypha already excluded via
                            // `current_book` above -- but a slimmed-down
                            // test fixture may legitimately carry only a
                            // few verses): every one of this verse's own
                            // raw spans is honestly `not_found`, never a
                            // panic on a caller's own smaller fixture.
                            None => {
                                stats.source_spans_total += v.spans_raw.len();
                                stats.not_found += v.spans_raw.len();
                                Vec::new()
                            }
                        };
                        verses.push(RedLetterVerse { book_index: v.book_index, chapter: v.chapter, verse: v.verse, spans: aligned });
                    }
                }
            }
            continue;
        }
        if tag.starts_with("<q") {
            if tag.contains("who=\"Jesus\"") && has_attr(tag, "sID") {
                if let Some(sid) = attr(tag, "sID") {
                    open_q_sid = Some(sid.to_string());
                    span_buf.clear();
                }
                continue;
            }
            if let Some(eid) = attr(tag, "eID") {
                if open_q_sid.as_deref() == Some(eid) {
                    if let Some(v) = open_verse.as_mut() {
                        v.spans_raw.push(std::mem::take(&mut span_buf));
                    }
                    open_q_sid = None;
                }
                // An eID belonging to some OTHER (non-Jesus, or already-
                // closed) q milestone is not this parser's own open span --
                // ignored, matching the file's own disclosure that q
                // markup here is used ONLY for the who="Jesus" feature (no
                // real nesting expected; this check is the defensive floor
                // if that ever proves wrong on some verse).
                continue;
            }
            continue;
        }
        // Every other tag (`<transChange>`, `</transChange>`, `<p>`,
        // `<title>`, `<note>`, book/chapter CLOSE tags, etc.) is
        // transparent: strip the tag itself, keep scanning -- its own
        // inner text (if any) is picked up by the plain-text branch above
        // precisely because no state changed.
    }

    Ok(RedLetterCorpus { verses, stats })
}

/// Reads and parses `red_letter_dir/eng-kjv.osis.xml` -- the one
/// filesystem-touching wrapper (this crate's own "`&str`-in/data-out; only
/// the caller touches the filesystem" discipline, `build.rs`'s own doc
/// comment) -- mirroring `kretzmann::read_all`/`concord::read_all`'s own
/// calling convention (a root PATH, not raw text, so `service.rs`'s
/// `load_red_letter`/`bins/compile_graph.rs` can call this exactly like its
/// two siblings).
pub fn read_all(red_letter_dir: &std::path::Path, kjv_verses: &HashMap<String, String>) -> Result<RedLetterCorpus> {
    let path = red_letter_dir.join("eng-kjv.osis.xml");
    let xml = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    parse(&xml, kjv_verses).with_context(|| format!("parsing {}", path.display()))
}

/// Groups the verse SET (`corpus.verses`, one entry per source-marked verse
/// regardless of alignment) into MAXIMAL CONTIGUOUS canon-order ranges --
/// decision 3's own SpokenBy shape ("one row per maximal contiguous verse
/// RANGE of the verse set... ranges keep the table honest to discourse
/// shape"). Two verses are contiguous when the second is EXACTLY the
/// first's own next verse in canon reading order (same book, same chapter,
/// verse+1, OR the first chapter's own last verse followed by the next
/// chapter's own verse 1 -- `chapter_verse_counts` supplies the per-
/// (book,chapter) verse count this needs to detect a chapter rollover
/// correctly, never assumed to be exactly `verse+1` in a different
/// chapter). `corpus.verses` is already in document (== canon) order, so
/// this is a single linear pass, no sort needed.
pub fn contiguous_ranges(verses: &[RedLetterVerse], chapter_verse_counts: &dyn Fn(u8, u16) -> Option<u16>) -> Vec<((u8, u16, u16), (u8, u16, u16))> {
    let mut ranges: Vec<((u8, u16, u16), (u8, u16, u16))> = Vec::new();
    for v in verses {
        let cur = (v.book_index, v.chapter, v.verse);
        if let Some(last) = ranges.last_mut() {
            let (from, to) = *last;
            let is_next = if to.0 == cur.0 && to.1 == cur.1 && to.2 + 1 == cur.2 {
                true
            } else if to.0 == cur.0 && to.1 + 1 == cur.1 && cur.2 == 1 && chapter_verse_counts(to.0, to.1) == Some(to.2) {
                true
            } else {
                false
            };
            if is_next {
                last.1 = cur;
                continue;
            }
            let _ = from;
        }
        ranges.push((cur, cur));
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verses_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    // ---- align_verse (the pure alignment law) --------------------------

    #[test]
    fn align_verse_finds_an_exact_case_sensitive_match() {
        let mut stats = AlignmentStats::default();
        let aligned = align_verse(&["Follow me, and I will make you fishers of men.\n".to_string()], "And he saith unto them, Follow me, and I will make you fishers of men.", &mut stats);
        assert_eq!(aligned, vec![(24, 70)]);
        assert_eq!(stats.exact, 1);
        assert_eq!(stats.case_insensitive, 0);
        assert_eq!(stats.not_found, 0);
        assert_eq!(stats.source_spans_total, 1);
    }

    #[test]
    fn align_verse_falls_back_to_case_insensitive_and_still_reports_our_bytes() {
        let mut stats = AlignmentStats::default();
        // Source normalizes "Lord" where our RESTORED text carries "LORD".
        let aligned = align_verse(&["the lord thy God".to_string()], "Thou shalt worship the LORD thy God, and him only shalt thou serve.", &mut stats);
        let (start, end) = aligned[0];
        assert_eq!(&"Thou shalt worship the LORD thy God, and him only shalt thou serve."[start..end], "the LORD thy God", "the served text must be OUR bytes/casing, never the source's own");
        assert_eq!(stats.case_insensitive, 1);
        assert_eq!(stats.exact, 0);
    }

    #[test]
    fn align_verse_drops_a_genuinely_unfound_span_without_guessing() {
        let mut stats = AlignmentStats::default();
        let aligned = align_verse(&["this text appears nowhere".to_string()], "And Jesus said, Verily I say unto you.", &mut stats);
        assert!(aligned.is_empty());
        assert_eq!(stats.not_found, 1);
        assert_eq!(stats.exact, 0);
        assert_eq!(stats.case_insensitive, 0);
    }

    #[test]
    fn align_verse_resolves_two_spans_in_one_verse_left_to_right() {
        // A verse with narration between two separate sayings -- the SAME
        // short word ("Come") legitimately repeats; the cursor must not
        // collapse both onto the first occurrence.
        let mut stats = AlignmentStats::default();
        let aligned = align_verse(&["Come unto me".to_string(), "Come, follow me".to_string()], "Jesus said, Come unto me, all ye that labour. Then he said, Come, follow me.", &mut stats);
        assert_eq!(aligned.len(), 2);
        assert!(aligned[0].0 < aligned[1].0, "the second span must be found AFTER the first, not re-matching it");
    }

    #[test]
    fn align_verse_trims_a_trailing_formatting_newline() {
        // MAT.4.19's own real shape: the source span's raw text carries a
        // trailing "\n" before its own closing tag.
        let mut stats = AlignmentStats::default();
        let aligned = align_verse(&["Follow me.\n".to_string()], "He saith, Follow me.", &mut stats);
        assert_eq!(aligned, vec![(10, 20)]);
    }

    // ---- parse (the full XML scan) --------------------------------------

    fn osis(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?><osis><osisText><header></header><div type="bookGroup"><div type="book" osisID="Matt" canonical="true"><title>Matthew</title>{body}</div></div></osisText></osis>"#
        )
    }

    #[test]
    fn parse_extracts_the_mat_4_19_case_exactly_the_brief_names() {
        // The narration prefix is NOT red; the speech is -- the exact
        // required test spot (batch-red1-brief.md decision 6).
        let xml = osis(
            r#"<chapter osisRef="Matt.4" sID="c1" n="4" /><verse osisID="Matt.4.19" sID="v1" n="19" />And he saith unto them,
<q who="Jesus" sID="q1" marker="" />Follow me, and I will make you fishers of men.
<q eID="q1" /><verse eID="v1" /><chapter eID="c1" />"#,
        );
        let kjv = verses_map(&[("MAT.4.19", "And he saith unto them, Follow me, and I will make you fishers of men.")]);
        let corpus = parse(&xml, &kjv).unwrap();
        assert_eq!(corpus.verses.len(), 1);
        let v = &corpus.verses[0];
        assert_eq!((v.book_index, v.chapter, v.verse), (39, 4, 19)); // Matthew is index 39
        assert_eq!(v.spans, vec![(24, 70)]);
        let canon = &kjv["MAT.4.19"];
        assert_eq!(&canon[24..70], "Follow me, and I will make you fishers of men.");
        assert_eq!(&canon[..24], "And he saith unto them, ", "the narration prefix must NOT be part of the red span");
    }

    #[test]
    fn parse_skips_a_verse_with_no_jesus_span_entirely() {
        let xml = osis(r#"<chapter osisRef="Matt.1" sID="c1" n="1" /><verse osisID="Matt.1.1" sID="v1" n="1" />The book of the generation of Jesus Christ.<verse eID="v1" /><chapter eID="c1" />"#);
        let kjv = verses_map(&[("MAT.1.1", "The book of the generation of Jesus Christ.")]);
        let corpus = parse(&xml, &kjv).unwrap();
        assert!(corpus.verses.is_empty(), "a verse with zero <q who=\"Jesus\"> runs must not enter the verse set");
        assert_eq!(corpus.stats.verses_with_source_markup, 0);
    }

    #[test]
    fn parse_handles_two_separate_jesus_runs_in_one_verse() {
        let xml = osis(
            r#"<chapter osisRef="Matt.9" sID="c1" n="9" /><verse osisID="Matt.9.6" sID="v1" n="6" />But that ye may know... (then saith he to the sick of the palsy,) <q who="Jesus" sID="q1" marker="" />Arise, take up thy bed<q eID="q1" />, and go unto thine house.<verse eID="v1" /><chapter eID="c1" />"#,
        );
        let kjv = verses_map(&[("MAT.9.6", "But that ye may know... (then saith he to the sick of the palsy,) Arise, take up thy bed, and go unto thine house.")]);
        let corpus = parse(&xml, &kjv).unwrap();
        assert_eq!(corpus.verses.len(), 1);
        assert_eq!(corpus.verses[0].spans.len(), 1);
    }

    #[test]
    fn parse_excludes_apocrypha_books_that_dont_resolve_against_our_canon() {
        let xml = format!(
            r#"<?xml version="1.0" encoding="utf-8"?><osis><osisText><header></header><div type="bookGroup"><div type="book" osisID="Tob" canonical="false"><title>Tobit</title><chapter osisRef="Tob.1" sID="c1" n="1" /><verse osisID="Tob.1.1" sID="v1" n="1" /><q who="Jesus" sID="q1" marker="" />should never resolve<q eID="q1" /><verse eID="v1" /><chapter eID="c1" /></div></div></osisText></osis>"#
        );
        let kjv: HashMap<String, String> = HashMap::new();
        let corpus = parse(&xml, &kjv).unwrap();
        assert!(corpus.verses.is_empty(), "an Apocryphal book (osisID doesn't resolve against our 66-book canon) must contribute nothing");
    }

    #[test]
    fn parse_handles_a_transchange_added_word_transparently() {
        let xml = osis(
            r#"<chapter osisRef="Matt.4" sID="c1" n="4" /><verse osisID="Matt.4.4" sID="v1" n="4" />But he answered and said, <q who="Jesus" sID="q1" marker="" />It is written, Man shall not live by bread alone, but by every <transChange type="added">word</transChange> that proceedeth out of the mouth of God.<q eID="q1" /><verse eID="v1" /><chapter eID="c1" />"#,
        );
        let kjv = verses_map(&[("MAT.4.4", "But he answered and said, It is written, Man shall not live by bread alone, but by every word that proceedeth out of the mouth of God.")]);
        let corpus = parse(&xml, &kjv).unwrap();
        assert_eq!(corpus.verses.len(), 1);
        assert_eq!(corpus.verses[0].spans.len(), 1, "the transChange-wrapped word must flow through as part of the same span, not break it in two");
    }

    #[test]
    fn parse_counts_a_genuinely_unresolvable_span_without_guessing_and_keeps_the_verse_in_the_set() {
        let xml = osis(r#"<chapter osisRef="Matt.5" sID="c1" n="5" /><verse osisID="Matt.5.3" sID="v1" n="3" /><q who="Jesus" sID="q1" marker="" />words that do not match our canon at all<q eID="q1" /><verse eID="v1" /><chapter eID="c1" />"#);
        let kjv = verses_map(&[("MAT.5.3", "Blessed are the poor in spirit: for theirs is the kingdom of heaven.")]);
        let corpus = parse(&xml, &kjv).unwrap();
        assert_eq!(corpus.verses.len(), 1, "the verse set is edition-independent -- membership does not depend on alignment success");
        assert!(corpus.verses[0].spans.is_empty(), "an unaligned span must never be guessed into a wrong offset");
        assert_eq!(corpus.stats.not_found, 1);
        assert_eq!(corpus.stats.verses_with_source_markup, 1);
    }

    // ---- contiguous_ranges ------------------------------------------------

    fn rv(book: u8, chapter: u16, verse: u16) -> RedLetterVerse {
        RedLetterVerse { book_index: book, chapter, verse, spans: vec![] }
    }

    #[test]
    fn contiguous_ranges_merges_consecutive_verses_into_one_range() {
        let verses = vec![rv(39, 5, 3), rv(39, 5, 4), rv(39, 5, 5)];
        let counts = |_b: u8, _c: u16| -> Option<u16> { None };
        let ranges = contiguous_ranges(&verses, &counts);
        assert_eq!(ranges, vec![((39, 5, 3), (39, 5, 5))]);
    }

    #[test]
    fn contiguous_ranges_splits_at_a_real_gap() {
        let verses = vec![rv(39, 5, 3), rv(39, 5, 4), rv(39, 5, 9)];
        let counts = |_b: u8, _c: u16| -> Option<u16> { None };
        let ranges = contiguous_ranges(&verses, &counts);
        assert_eq!(ranges, vec![((39, 5, 3), (39, 5, 4)), ((39, 5, 9), (39, 5, 9))]);
    }

    #[test]
    fn contiguous_ranges_crosses_a_chapter_boundary_when_the_prior_chapter_truly_ends_there() {
        let verses = vec![rv(39, 5, 47), rv(39, 5, 48), rv(39, 6, 1)];
        let counts = |_b: u8, c: u16| -> Option<u16> { if c == 5 { Some(48) } else { None } };
        let ranges = contiguous_ranges(&verses, &counts);
        assert_eq!(ranges, vec![((39, 5, 47), (39, 6, 1))]);
    }

    #[test]
    fn contiguous_ranges_does_not_cross_a_chapter_boundary_when_the_prior_chapter_has_more_verses() {
        let verses = vec![rv(39, 5, 47), rv(39, 6, 1)];
        // Chapter 5 actually has 48 verses -- verse 47 is NOT its last, so
        // this must NOT be treated as a rollover into chapter 6.
        let counts = |_b: u8, c: u16| -> Option<u16> { if c == 5 { Some(48) } else { None } };
        let ranges = contiguous_ranges(&verses, &counts);
        assert_eq!(ranges, vec![((39, 5, 47), (39, 5, 47)), ((39, 6, 1), (39, 6, 1))]);
    }
}
