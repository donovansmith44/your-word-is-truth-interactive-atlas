//! SVEB-1: the Svebilius Catechism parser.
//!
//! SOURCE: `data/raw/catechism-mapping/catechism-{sha}/svebilius/en/`, the
//! brain-fuel/catechism repo's own `svebilius/` directory -- FETCHED since
//! Batch F2 and deliberately ingested-nothing twice (the F2 brief scoped it
//! out; `data/curated/catechism-mapping.toml`'s own header re-noted it for
//! completeness). This batch ingests it.
//!
//! Olaus Svebilius' catechism, in Bishop Daniel Juslenius' Finnish
//! translation (Skara, 1745), Englished from Tero Kotti's 2007 edition of
//! `Svebilius_Katekismus.pdf`. Two files are read:
//!
//! - `03-yksinkertainen-selitys.md` -- the *Simple Exposition*, the work
//!   this batch is actually after: 314 numbered questions-and-answers whose
//!   answers quote Scripture inline. Parsed into `SvebiliusUnit`s.
//! - `02-lutherin-katekismus.md` -- Luther's OWN Small Catechism as this
//!   edition renders it. NOT a third copy of a text the app already holds
//!   twice (`catechism.toml` and Concord part 7); parsed as a parallel
//!   RENDERING, the same shape the six brain-fuel/bible editions take on
//!   Bible text units.
//!
//! MARKDOWN GRAMMAR (read off the real files, not assumed):
//! - `## HEADING` opens a SECTION. The Exposition has nine, in source
//!   order: Preface, the Law, the Creed, the Lord's Prayer, Baptism,
//!   Confession, the Lord's Supper, the Penitential Psalms, Confession of
//!   Sins.
//! - Within a section, `**N. QUESTION**` opens a unit and `Answer: ...`
//!   carries its answer. `N` RESTARTS at 1 in every section.
//! - The last two sections are not question-and-answer at all: section 8 is
//!   seven psalms quoted whole, section 9 is a single confession formula.
//!   Their blocks are still addressable units, numbered sequentially, with
//!   no question -- disclosed rather than force-fit into a Q&A shape the
//!   source never uses.
//!
//! THE SOURCE'S OWN NUMBERING IS CARRIED FAITHFULLY, GAPS INCLUDED.
//! Section 4 (the Lord's Prayer) holds 53 questions but numbers running to
//! 54: the source skips 53. Renumbering to close that would put every
//! citation of this edition permanently out of step with the printed text,
//! so the gap stands and `SvebiliusStats::numbering_gaps` reports it.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};

use crate::catechism_map::canonicalize_ref;

/// How many of the nine sections are question-and-answer. Sections 8 and 9
/// (the Penitential Psalms, the Confession of Sins) are prose.
pub const QA_SECTIONS: usize = 7;

/// The Exposition's nine sections, in source order. Position in this list
/// IS the `section` number in a `SvebiliusRef` -- 1-based, so the Preface
/// is 1 and Confession of Sins is 9.
pub const SECTIONS: [&str; 9] = [
    "Preface",
    "On the Ten Commandments of God, or the Law",
    "On the Creed and the Gospel",
    "On the Lord's Prayer",
    "On the Sacrament of Baptism",
    "On Confession and Absolution",
    "On the Lord's Supper, or the Sacrament of the Altar",
    "The Seven Penitential Psalms of King David",
    "Confession of Sins",
];

/// One addressable unit of the Exposition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvebiliusUnit {
    pub section: u8,
    /// The source's own number in a Q&A section; a sequential index in the
    /// two prose sections.
    pub unit: u16,
    /// `None` in the two prose sections (see this module's own header).
    pub question: Option<String>,
    pub answer: String,
    /// Canonical verse refs (`BOOK.CH.V`) quoted in the answer, in the
    /// order they appear. Empty is entirely normal -- plenty of answers
    /// cite nothing.
    pub verses: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct SvebiliusStats {
    pub units: usize,
    pub qa_units: usize,
    pub prose_units: usize,
    pub verse_links: usize,
    /// Human-readable citation strings the ref parser could not resolve,
    /// with the unit they came from. REPORTED, never silently dropped --
    /// the same citation-integrity rule Batch F2 set for the YAML mapping.
    pub unresolved: Vec<String>,
    /// `(section, missing_number)` for every gap in a section's own
    /// numbering. See this module's header: gaps are the source's, and are
    /// disclosed rather than closed.
    pub numbering_gaps: Vec<(u8, u16)>,
    /// All-caps structural dividers skipped ("FIRST CHIEF ARTICLE", ...).
    /// Reported rather than silently dropped, the same way the Concord
    /// parser discloses the site-furniture articles it excludes.
    pub skipped_dividers: Vec<String>,
    /// Structural SUB-headings skipped inside a Q&A section ("The first
    /// table", "The second petition", "THE FIRST ARTICLE OF FAITH, Of God
    /// the Father and of Creation"). Ten of them in the real file.
    pub skipped_subheadings: Vec<String>,
    /// Continuation lines folded into the PRECEDING unit's answer -- extra
    /// Scripture the source prints on its own line. Eleven in the real
    /// file, and they carry real citations, so folding them in rather than
    /// dropping them is worth several edges.
    pub continuations: usize,
}

/// Does this line end a sentence?
///
/// This is the whole test that separates the two kinds of unattached line
/// a Q&A section contains, and on the real file it separates them
/// PERFECTLY -- 21 orphan lines, 11 ending in sentence punctuation (every
/// one a continuation carrying Scripture), 10 not (every one a structural
/// sub-heading). The source is consistent because its sub-headings are
/// labels, not sentences.
fn ends_sentence(line: &str) -> bool {
    matches!(line.trim_end().chars().last(), Some('.') | Some('!') | Some('?') | Some('"'))
}

/// A structural divider line: the source announces each coming chief part
/// with a bare all-caps line ("FIRST CHIEF ARTICLE", "THE THIRD CHIEF
/// PART") sitting at the TAIL of the section before it, six in all.
///
/// These are furniture, not content, and treating them as content is not
/// harmless: each one landed as a question-less unit numbered from a
/// separate prose counter, which COLLIDED with a real question number in
/// the same section (the first one produced a second "Sveb 1.1"). Caught
/// by `svebilius_real_data.rs` against the actual file, not by the unit
/// fixtures -- which is what that test exists for.
fn is_divider(line: &str) -> bool {
    let letters: Vec<char> = line.chars().filter(|c| c.is_alphabetic()).collect();
    letters.len() >= 2
        && letters.iter().all(|c| c.is_uppercase())
        && line.chars().all(|c| c.is_alphabetic() || c == ' ')
}

/// Every Scripture citation in one answer, in source order, NORMALIZED for
/// `canonicalize_ref`.
///
/// DIRECTION MATTERS. The obvious approach -- scan left to right for a
/// capitalized word and try to read a citation out of it -- does not work
/// on running English prose: given "Because I am baptized. Gal. 3:27.", a
/// forward scanner happily consumes "Because I am baptized Gal" as a
/// multi-word book name and then finds a perfectly good "3:27" after it.
/// (It did exactly that before this rewrite.)
///
/// So the anchor is the part that is actually distinctive: the
/// `chapter:verse` pair. From each one, walk BACKWARD over at most three
/// words -- the longest real book name is "Song of Solomon" -- and accept
/// the longest candidate that `resolve_book_name` recognizes as an actual
/// book of the canon. Validating against the real 66-book table is what
/// makes this safe: prose can precede a number, but prose does not resolve
/// to a book.
///
/// NORMALIZATION: abbreviating periods are dropped ("Deut." -> "Deut").
/// `resolve_alias` itself is indifferent to them (`canon::norm` strips
/// non-alphanumerics), but the tokenizer in front of it --
/// `catechism_map::split_book_and_tail` -- requires purely alphabetic
/// book-name tokens, so "Deut." would never reach the resolver.
///
/// Deliberately CONSERVATIVE: an explicit `chapter:verse` is required.
/// Bare-chapter mentions ("as Psalm 51 teaches") are not harvested; in
/// running English they are indistinguishable from a sentence about a
/// psalm, and a false edge is worse than a missed one.
fn citations(answer: &str) -> Vec<String> {
    let b = answer.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < b.len() {
        if !b[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        // Must be the start of a number (not mid-number).
        if i > 0 && b[i - 1].is_ascii_digit() {
            i += 1;
            continue;
        }

        // Read chapter:verse[,v][-v].
        let tail_start = i;
        let mut j = i;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j >= b.len() || b[j] != b':' {
            i = j.max(i + 1);
            continue;
        }
        j += 1;
        let digits_after_colon = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j == digits_after_colon {
            i = j.max(i + 1);
            continue;
        }
        while j < b.len() && (b[j] == b',' || b[j] == b'-') && j + 1 < b.len() && b[j + 1].is_ascii_digit() {
            j += 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
        }
        let tail = &answer[tail_start..j];

        // Walk backward over the preceding words.
        let before = &answer[..tail_start];
        let words: Vec<&str> = before.split_whitespace().collect();
        let mut matched: Option<String> = None;
        for take in (1..=3.min(words.len())).rev() {
            let slice = &words[words.len() - take..];
            // Strip abbreviating periods; a word carrying any other
            // punctuation is sentence text, not part of a name.
            let mut parts: Vec<String> = Vec::new();
            let mut ok = true;
            for (n, w) in slice.iter().enumerate() {
                let cleaned = w.trim_end_matches('.');
                let is_ordinal = n == 0 && matches!(*w, "1" | "2" | "3");
                if is_ordinal {
                    parts.push((*w).to_string());
                } else if !cleaned.is_empty() && cleaned.chars().all(|c| c.is_ascii_alphabetic()) {
                    parts.push(cleaned.to_string());
                } else {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            let candidate = parts.join(" ");
            if crate::catechism_map::resolve_book_name(&candidate).is_some() {
                matched = Some(candidate);
                break;
            }
        }

        if let Some(book) = matched {
            out.push(format!("{book} {tail}"));
        }
        i = j.max(i + 1);
    }

    out
}

/// Parses the Simple Exposition. `verses` is the compiled KJV text map,
/// used by `canonicalize_ref` both to expand ranges and to reject a ref
/// naming a verse this atlas does not actually hold.
pub fn parse_exposition(input: &str, verses: &HashMap<String, String>) -> Result<(Vec<SvebiliusUnit>, SvebiliusStats)> {
    let mut units: Vec<SvebiliusUnit> = Vec::new();
    let mut stats = SvebiliusStats::default();

    let mut section: u8 = 0;
    let mut prose_counter: u16 = 0;
    let mut pending: Option<(u16, String)> = None; // (number, question)

    let flush = |units: &mut Vec<SvebiliusUnit>,
                 stats: &mut SvebiliusStats,
                 section: u8,
                 unit: u16,
                 question: Option<String>,
                 answer: String| {
        if answer.trim().is_empty() {
            return;
        }
        let raws = citations(&answer);
        let mut vs = Vec::new();
        for raw in raws {
            match canonicalize_ref(&raw, verses) {
                Ok(expanded) => vs.extend(expanded),
                Err(_) => stats.unresolved.push(format!("Sveb {section}.{unit}: '{raw}'")),
            }
        }
        stats.verse_links += vs.len();
        if question.is_some() {
            stats.qa_units += 1;
        } else {
            stats.prose_units += 1;
        }
        stats.units += 1;
        units.push(SvebiliusUnit { section, unit, question, answer: answer.trim().to_string(), verses: vs });
    };

    for line in input.lines() {
        let line = line.trim();

        if let Some(heading) = line.strip_prefix("## ") {
            let heading = heading.trim();
            let Some(pos) = SECTIONS.iter().position(|s| *s == heading) else {
                bail!("svebilius: unknown section heading '{heading}' -- SECTIONS is the source's own nine, in order");
            };
            section = (pos + 1) as u8;
            prose_counter = 0;
            pending = None;
            continue;
        }
        if section == 0 {
            continue; // front matter before the first heading
        }

        // `**N. Question text**`
        if let Some(rest) = line.strip_prefix("**") {
            if let Some(end) = rest.find("**") {
                let inner = &rest[..end];
                if let Some(dot) = inner.find(". ") {
                    if let Ok(n) = inner[..dot].parse::<u16>() {
                        pending = Some((n, inner[dot + 2..].trim().to_string()));
                        continue;
                    }
                }
            }
        }

        if line.is_empty() {
            continue;
        }

        if is_divider(line) {
            stats.skipped_dividers.push(line.to_string());
            continue;
        }

        // An answer line: either the answer to a pending question, or a
        // standalone prose block in a non-Q&A section.
        let answer = line.strip_prefix("Answer: ").unwrap_or(line).to_string();
        match pending.take() {
            Some((n, q)) => flush(&mut units, &mut stats, section, n, Some(q), answer),
            // No pending question. In the two PROSE sections that is just
            // the next block. In a Q&A section it is one of two things,
            // and telling them apart matters: a CONTINUATION carrying more
            // Scripture (fold it into the unit above, keeping its edges)
            // or a structural SUB-HEADING (skip it). Minting a fresh unit
            // for either -- which is what this did before -- collided its
            // prose counter with a real question number.
            None if section as usize > QA_SECTIONS => {
                prose_counter += 1;
                flush(&mut units, &mut stats, section, prose_counter, None, answer);
            }
            None if ends_sentence(&answer) => {
                if let Some(last) = units.last_mut() {
                    last.answer.push(' ');
                    last.answer.push_str(answer.trim());
                    for raw in citations(&answer) {
                        match canonicalize_ref(&raw, verses) {
                            Ok(expanded) => {
                                stats.verse_links += expanded.len();
                                last.verses.extend(expanded);
                            }
                            Err(_) => stats
                                .unresolved
                                .push(format!("Sveb {}.{} (continuation): '{raw}'", last.section, last.unit)),
                        }
                    }
                    stats.continuations += 1;
                }
            }
            None => stats.skipped_subheadings.push(answer),
        }
    }

    // Disclose numbering gaps per Q&A section (see the module header).
    for sec in 1..=SECTIONS.len() as u8 {
        let mut nums: Vec<u16> =
            units.iter().filter(|u| u.section == sec && u.question.is_some()).map(|u| u.unit).collect();
        if nums.is_empty() {
            continue;
        }
        nums.sort_unstable();
        let max = *nums.last().unwrap();
        for n in 1..=max {
            if !nums.contains(&n) {
                stats.numbering_gaps.push((sec, n));
            }
        }
    }

    Ok((units, stats))
}

/// One item of Luther's Small Catechism as the Juslenius/Svebilius edition
/// renders it -- a parallel RENDERING, keyed by the `### Heading` the
/// source prints (e.g. "The First Commandment"), never a new copy of the
/// catechism's structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JusleniusRendering {
    pub heading: String,
    pub text: String,
}

/// Parses `02-lutherin-katekismus.md` into `### Heading` -> prose.
pub fn parse_juslenius(input: &str) -> Result<Vec<JusleniusRendering>> {
    let mut out: Vec<JusleniusRendering> = Vec::new();
    let mut heading: Option<String> = None;
    let mut buf: Vec<String> = Vec::new();

    let flush = |out: &mut Vec<JusleniusRendering>, heading: &mut Option<String>, buf: &mut Vec<String>| {
        if let Some(h) = heading.take() {
            let text = buf.join(" ").trim().to_string();
            if !text.is_empty() {
                out.push(JusleniusRendering { heading: h, text });
            }
        }
        buf.clear();
    };

    for line in input.lines() {
        let line = line.trim();
        if let Some(h) = line.strip_prefix("### ") {
            flush(&mut out, &mut heading, &mut buf);
            heading = Some(h.trim().to_string());
            continue;
        }
        if line.starts_with("## ") || line.starts_with("# ") {
            flush(&mut out, &mut heading, &mut buf);
            continue;
        }
        if heading.is_some() && !line.is_empty() {
            // Strip the source's own bold question marker; the question is
            // structure, and `catechism.toml` already carries it as
            // `explanation_heading`.
            let cleaned = line.trim_matches('*').trim();
            let cleaned = cleaned.strip_prefix("Answer: ").unwrap_or(cleaned);
            buf.push(cleaned.to_string());
        }
    }
    flush(&mut out, &mut heading, &mut buf);

    if out.is_empty() {
        bail!("svebilius: parsed no Juslenius renderings -- expected `### Heading` sections");
    }
    Ok(out)
}

/// Reads both files from a vendored `svebilius/en` directory.
pub fn read_all(dir: &std::path::Path, verses: &HashMap<String, String>) -> Result<(Vec<SvebiliusUnit>, SvebiliusStats, Vec<JusleniusRendering>)> {
    let exposition = std::fs::read_to_string(dir.join("03-yksinkertainen-selitys.md"))
        .with_context(|| format!("reading {}", dir.join("03-yksinkertainen-selitys.md").display()))?;
    let luther = std::fs::read_to_string(dir.join("02-lutherin-katekismus.md"))
        .with_context(|| format!("reading {}", dir.join("02-lutherin-katekismus.md").display()))?;

    let (units, stats) = parse_exposition(&exposition, verses)?;
    let renderings = parse_juslenius(&luther)?;
    Ok((units, stats, renderings))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verses() -> HashMap<String, String> {
        let mut m = HashMap::new();
        for (k, v) in [
            ("GAL.3.27", "For as many of you as have been baptized into Christ have put on Christ."),
            ("ACT.4.12", "Neither is there salvation in any other..."),
            ("DEU.6.6", "And these words..."),
            ("DEU.6.7", "And thou shalt teach them diligently..."),
        ] {
            m.insert(k.to_string(), v.to_string());
        }
        m
    }

    #[test]
    fn citation_shapes_the_source_actually_uses() {
        // Abbreviated with a period, with an ordinal, with a comma list,
        // and with no period at all -- all four occur in the real file.
        assert_eq!(citations("Gal. 3:27. As many"), vec!["Gal 3:27"]);
        assert_eq!(citations("1 Pet. 2:2. As newborn"), vec!["1 Pet 2:2"]);
        assert_eq!(citations("Deut. 6:6,7. These words"), vec!["Deut 6:6,7"]);
        assert_eq!(citations("Acts 4:12. Neither"), vec!["Acts 4:12"]);
        // Multi-word names resolve (the backward walk takes up to three
        // words). "Song of Sol." -- an abbreviation the canon table does
        // not carry -- is deliberately NOT asserted here: it does not
        // occur anywhere in the real file, and inventing support for an
        // unattested spelling would be machinery guarding nothing.
        assert_eq!(citations("Song of Solomon 2:1."), vec!["Song of Solomon 2:1"]);
    }

    #[test]
    fn running_prose_before_a_citation_is_not_swallowed_into_the_book_name() {
        // The regression this scanner was rewritten for: a forward scanner
        // read "Because I am baptized Gal" as the book name here.
        assert_eq!(
            citations("Because I am baptized. Gal. 3:27. As many of you."),
            vec!["Gal 3:27"]
        );
    }

    #[test]
    fn ordinary_prose_is_not_mistaken_for_a_citation() {
        assert!(citations("Answer: I am.").is_empty());
        assert!(citations("Because I am baptized").is_empty());
        // A year is not a reference -- no chapter:verse pair.
        assert!(citations("in the year 1530, was offered").is_empty());
        // A number that resolves to no book is not a reference.
        assert!(citations("the third day 3:16 rose").is_empty());
    }

    #[test]
    fn numbering_restarts_per_section_and_gaps_are_reported() {
        let src = concat!(
            "## Preface\n\n**1. Are you a Christian?**\n\nAnswer: I am.\n\n",
            "FIRST CHIEF ARTICLE\n\n",
            "## On the Ten Commandments of God, or the Law\n\n**1. What?**\n\nAnswer: The Law.\n",
        );
        let (units, stats) = parse_exposition(src, &verses()).unwrap();
        // Two real units, not three -- and no second "Sveb 1.1".
        assert_eq!(units.len(), 2, "{units:?}");
        assert_eq!(stats.skipped_dividers, vec!["FIRST CHIEF ARTICLE"]);
        assert!(units.iter().all(|u| u.question.is_some()));
    }

    #[test]
    fn a_prose_section_yields_numbered_units_with_no_question() {
        let src = "## Confession of Sins\n\nI, a poor sinful man, confess.\n";
        let (units, _) = parse_exposition(src, &verses()).unwrap();
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].section, 9);
        assert_eq!(units[0].unit, 1);
        assert_eq!(units[0].question, None);
    }

    #[test]
    fn juslenius_renderings_key_on_their_heading() {
        let src = "# Dr. Martin Luther's CATECHISM\n\n## 1. The Ten Commandments\n\n\
                   ### The First Commandment\n\nI am the Lord thy God.\n\n\
                   **What does this mean?**\n\nAnswer: We should fear and love God above all things.\n";
        let out = parse_juslenius(src).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].heading, "The First Commandment");
        assert!(out[0].text.contains("fear and love God above all things"), "{}", out[0].text);
    }

    #[test]
    fn an_unknown_section_heading_fails_loudly() {
        let err = parse_exposition("## Not A Real Section\n\n**1. Q?**\n\nAnswer: A.\n", &verses()).unwrap_err();
        assert!(err.to_string().contains("unknown section heading"), "{err}");
    }

    #[test]
    fn a_continuation_line_folds_into_the_unit_above_and_keeps_its_edges() {
        let src = concat!(
            "## On the Ten Commandments of God, or the Law\n\n",
            "**1. What does the Law teach?**\n\nAnswer: What we owe God.\n\n",
            // Extra Scripture the source prints on its own line -- it
            // belongs to the answer above, and its citation is a real edge.
            "Gal. 3:27. As many of you as have been baptized.\n\n",
            // A structural sub-heading -- a label, not a sentence.
            "The first table\n\n",
            "**2. Next?**\n\nAnswer: Yes.\n",
        );
        let (units, stats) = parse_exposition(src, &verses()).unwrap();
        assert_eq!(units.len(), 2, "orphans must not mint units: {units:?}");
        assert_eq!(stats.continuations, 1);
        assert_eq!(stats.skipped_subheadings, vec!["The first table"]);
        // The folded line's citation reached the unit above it.
        assert_eq!(units[0].verses, vec!["GAL.3.27"]);
        assert!(units[0].answer.contains("baptized"), "{}", units[0].answer);
    }

    #[test]
    fn an_answer_carries_its_citations_expanded() {
        let src = concat!(
            "## Preface\n\n**2. Why are you called a Christian?**\n\n",
            "Answer: Because I am baptized. Gal. 3:27. As many of you. Acts 4:12. Neither is there.\n",
        );
        let (units, stats) = parse_exposition(src, &verses()).unwrap();
        assert_eq!(units[0].verses, vec!["GAL.3.27", "ACT.4.12"]);
        assert_eq!(stats.verse_links, 2);
        assert!(stats.unresolved.is_empty(), "{:?}", stats.unresolved);
    }

    #[test]
    fn chief_part_dividers_are_skipped_not_turned_into_units() {
        let src = concat!(
            "## Preface\n\n**1. Are you a Christian?**\n\nAnswer: I am.\n\n",
            "FIRST CHIEF ARTICLE\n\n",
            "## On the Ten Commandments of God, or the Law\n\n**1. What?**\n\nAnswer: The Law.\n",
        );
        let (units, stats) = parse_exposition(src, &verses()).unwrap();
        // Two real units, not three -- and no second "Sveb 1.1".
        assert_eq!(units.len(), 2, "{units:?}");
        assert_eq!(stats.skipped_dividers, vec!["FIRST CHIEF ARTICLE"]);
        assert!(units.iter().all(|u| u.question.is_some()));
        assert_eq!(units[1].section, 2, "the heading after a divider must still open its section");
    }
}
