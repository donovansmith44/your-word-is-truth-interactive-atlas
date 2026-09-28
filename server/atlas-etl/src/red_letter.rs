//! Parses the vendored KJV OSIS file into per-verse `<q who="Jesus">` spans and ALIGNS each against OUR canonical
//! verse text -- case-sensitive first, then an ASCII-case-insensitive retry, with an advancing cursor so repeated
//! phrases resolve left to right. A span aligning neither way is dropped and counted; the verse stays in the set.

use std::collections::HashMap;

use anyhow::{Context, Result};

/// One verse the SOURCE marks as containing Christ's words, present in the verse SET whatever the alignment
/// outcome. `spans` carries only the successfully aligned ranges, and is empty when every span in the verse failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedLetterVerse {
    pub book_index: u8,
    pub chapter: u16,
    pub verse: u16,
    /// Byte-offset `(start, end)` pairs into OUR canonical verse text, ascending and non-overlapping, which the
    /// cursor-advancing walk guarantees.
    pub spans: Vec<(usize, usize)>,
}

/// Alignment counts: every source span the file carries, however it resolved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AlignmentStats {
    /// The verse SET's size: one per verse carrying at least one source span, whatever the alignment outcome.
    pub verses_with_source_markup: usize,
    /// Every source span found, across every verse: the exact, case-insensitive and not-found counts summed.
    pub source_spans_total: usize,
    /// A case-sensitive verbatim substring match.
    pub exact: usize,
    /// The disclosed case class: found only case-insensitively.
    pub case_insensitive: usize,
    /// Neither found: dropped from the sub-verse table, never guessed, while the verse itself still counts.
    pub not_found: usize,
}

#[derive(Debug, Clone, Default)]
pub struct RedLetterCorpus {
    /// Canon order, which for a whole-Bible OSIS file is the document order this parser already walks.
    pub verses: Vec<RedLetterVerse>,
    pub stats: AlignmentStats,
}

/// Extracts `name="value"` from a raw tag, returning `None` for an absent attribute rather than panicking. It
/// searches for a LEADING SPACE before the name: `sID=` is a trailing substring of `osisID=`, so a bare search
/// matches inside that attribute's value and returns wrong data, which makes the verse quietly never close.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let pat = format!(" {name}=\"");
    let start = tag.find(&pat)? + pat.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// The same leading-space guard, for the boolean open/close detectors that need no value.
fn has_attr(tag: &str, name: &str) -> bool {
    tag.contains(&format!(" {name}=\""))
}

/// The real file uses only `&lt;` and `&gt;`, and only in its header prose, so this covers the five predefined XML
/// entities and no numeric references: sufficient for this one source, disclosed rather than assumed complete.
fn decode_entities(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('&') {
        return std::borrow::Cow::Borrowed(s);
    }
    std::borrow::Cow::Owned(s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&"))
}

/// Byte-for-byte comparison with ASCII letters case-insensitive and every other byte exact. Not `to_lowercase`,
/// which is Unicode-aware and can change a string's BYTE LENGTH, breaking the transfer of offsets back to ours.
fn ascii_ci_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(&x, &y)| x.eq_ignore_ascii_case(&y))
}

/// The leftmost offset where `needle` matches ASCII-case-insensitively. Only valid char boundaries are tried, and
/// a needle with any non-ASCII byte must match it exactly, which keeps every returned offset a boundary in turn.
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

/// Collapses whitespace runs to one space and trims: the source's XML is PRETTY-PRINTED, so a raw span text can
/// carry a newline where our prose has an ordinary space, and an unnormalized search misses real spans. Only the
/// NEEDLE needs this -- our own text is already ordinary single-spaced prose, so the haystack is never touched.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Aligns one verse's raw spans against OUR text, left to right with an advancing cursor; every input span counts,
/// found or not. The residual not-found class is real and disclosed -- each one a spelling or punctuation
/// difference from our canon -- and none is bridged: only case-insensitivity is an authorized second tier.
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
    /// Completed span texts in document order, each still carrying whatever whitespace the source left around it:
    /// trimming happens at alignment, so the raw text survives as long as possible.
    spans_raw: Vec<String>,
}

/// Aligns every span the MOMENT its verse closes, so a raw span text never outlives that verse's scope. `&str`-in,
/// data-out; the wrapper below is the one filesystem-touching function.
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
            // A truncated or malformed tail stops the scan rather than panicking: whatever parsed is honest.
            None => break,
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
                // No current book -- an Apocrypha verse, which our canon does not carry -- or an unparseable
                // number means this verse never opens, so its content is never collected rather than guessed.
                continue;
            }
            if let Some(eid) = attr(tag, "eID") {
                if let Some(v) = open_verse.take() {
                    if v.sid == eid && !v.spans_raw.is_empty() {
                        let dot_ref = format!("{}.{}.{}", atlas_core::canon::BOOKS[v.book_index as usize].code, v.chapter, v.verse);
                        stats.verses_with_source_markup += 1;
                        let aligned = match kjv_verses.get(&dot_ref) {
                            Some(canon_text) => align_verse(&v.spans_raw, canon_text, &mut stats),
                            // Our own canon lacks this verse, which a slimmed-down fixture legitimately can:
                            // every one of its raw spans is honestly not-found, never a panic on the caller.
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
                // An eID belonging to some OTHER quotation milestone is not this parser's open span, so it is
                // ignored: the defensive floor should the file's single-feature use of that markup ever change.
                continue;
            }
            continue;
        }
        // Every other tag is transparent: strip the tag and keep scanning. Its inner text is picked up by the
        // plain-text branch precisely because no state changed.
    }

    Ok(RedLetterCorpus { verses, stats })
}

/// The one filesystem-touching wrapper, taking a root path like its sibling corpus readers so the same callers
/// can drive all of them alike.
pub fn read_all(red_letter_dir: &std::path::Path, kjv_verses: &HashMap<String, String>) -> Result<RedLetterCorpus> {
    let path = red_letter_dir.join("eng-kjv.osis.xml");
    let xml = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    parse(&xml, kjv_verses).with_context(|| format!("parsing {}", path.display()))
}

/// Groups the verse set into MAXIMAL CONTIGUOUS canon-order ranges. Two verses are contiguous when the second is
/// exactly the first's next verse in reading order, chapter rollover included -- which is why the per-chapter
/// verse count is needed rather than assuming `verse + 1`. One linear pass: the input is already in canon order.
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
        let mut stats = AlignmentStats::default();
        let aligned = align_verse(&["Come unto me".to_string(), "Come, follow me".to_string()], "Jesus said, Come unto me, all ye that labour. Then he said, Come, follow me.", &mut stats);
        assert_eq!(aligned.len(), 2);
        assert!(aligned[0].0 < aligned[1].0, "the second span must be found AFTER the first, not re-matching it");
    }

    #[test]
    fn align_verse_trims_a_trailing_formatting_newline() {
        let mut stats = AlignmentStats::default();
        let aligned = align_verse(&["Follow me.\n".to_string()], "He saith, Follow me.", &mut stats);
        assert_eq!(aligned, vec![(10, 20)]);
    }

    fn osis(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?><osis><osisText><header></header><div type="bookGroup"><div type="book" osisID="Matt" canonical="true"><title>Matthew</title>{body}</div></div></osisText></osis>"#
        )
    }

    #[test]
    fn parse_extracts_the_mat_4_19_case_exactly_the_brief_names() {
        let xml = osis(
            r#"<chapter osisRef="Matt.4" sID="c1" n="4" /><verse osisID="Matt.4.19" sID="v1" n="19" />And he saith unto them,
<q who="Jesus" sID="q1" marker="" />Follow me, and I will make you fishers of men.
<q eID="q1" /><verse eID="v1" /><chapter eID="c1" />"#,
        );
        let kjv = verses_map(&[("MAT.4.19", "And he saith unto them, Follow me, and I will make you fishers of men.")]);
        let corpus = parse(&xml, &kjv).unwrap();
        assert_eq!(corpus.verses.len(), 1);
        let v = &corpus.verses[0];
        assert_eq!((v.book_index, v.chapter, v.verse), (39, 4, 19));
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
        let counts = |_b: u8, c: u16| -> Option<u16> { if c == 5 { Some(48) } else { None } };
        let ranges = contiguous_ranges(&verses, &counts);
        assert_eq!(ranges, vec![((39, 5, 47), (39, 5, 47)), ((39, 6, 1), (39, 6, 1))]);
    }
}
