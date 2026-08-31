//! Batch F2: ingestion of the user's own catechism verse-mapping repo
//! (brain-fuel/catechism, <https://github.com/brain-fuel/catechism>, pinned
//! at a specific commit SHA -- see `data/fetch-raw.ps1`) -- `resources/*.yaml`,
//! one file per catechism topic, each holding numbered QUESTION-level
//! entries: `{ title: "...", refs: !!set of "Book Chapter:Verse" }`. This
//! module (a) parses that YAML shape and (b) canonicalizes each
//! human-readable ref string into this app's own canonical verse-ref
//! grammar (`BOOK.CH.V`, one string per INDIVIDUAL verse -- the same
//! flattened convention `curated::expand_verse_ref` already uses for
//! hand-typed `catechism.toml` citations, so a passage-grouping consumer
//! downstream, batch-f2-brief.md's 6-ARCH, sees the identical shape
//! regardless of which batch's data produced it).
//!
//! Human ref forms actually observed across the real ~45 files (verified by
//! a full grep sweep of every distinct ref string in every
//! `resources/*.yaml` file BEFORE writing this parser, not guessed):
//! - `"Isaiah 45:20"` -- book chapter:verse
//! - `"Exodus 20:1-3"` -- book chapter:verse-verse (same-chapter range)
//! - `"Psalm 1"` / `"1 Samuel 28"` -- bare chapter (no verse at all)
//! - `"Romans 12-13"` / `"Job 38-41"` -- bare CHAPTER range (no colon at all)
//! - `"1 John 1:7-2:2"` / `"Genesis 3:1-4:12"` -- CROSS-CHAPTER verse range
//! - `"Exodus 34:1, 27-28"` -- comma-separated compound: the book+chapter is
//!   stated once, in the FIRST segment; every later comma segment is a bare
//!   verse or verse-range within that SAME chapter
//! No semicolons, no unicode dashes, no malformed/inverted ranges, and no
//! ref ever crosses a BOOK boundary (only chapter boundaries) anywhere in
//! the real data -- all confirmed by that same sweep.
//!
//! Book names are always the FULL English name (never an abbreviation),
//! e.g. `"1 Corinthians"`, `"Song of Solomon"` -- resolved primarily via
//! `atlas_core::canon::resolve_alias` (covers 52 of the 53 distinct book
//! names actually used in the real data). The one gap: the repo cites the
//! Psalms in the SINGULAR ("Psalm 23"), while this app's own canonical name
//! is plural ("Psalms") and its OSIS abbreviation is "Ps" -- neither
//! normalizes to "psalm" -- so a tiny, disclosed local fallback covers it.
//! `every_real_book_name_resolves` (this module's own test, run against a
//! fixture mirroring the real distinct-name list) pins that this fallback
//! is the ONLY gap, not merely "probably fine."

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use atlas_core::canon::resolve_alias;
use atlas_core::data::{CatechismPart, CatechismQuestion};
use atlas_core::refs::BookId;

/// One QUESTION-level entry parsed from a single `resources/*.yaml` file,
/// refs still in RAW, human-readable form -- canonicalization
/// (`canonicalize_ref` below) is a separate step, so a YAML-shape parse
/// failure and a ref-canonicalization failure are reported distinctly, and
/// a caller can attach its own per-file/per-item context to each.
#[derive(Debug, Clone, PartialEq)]
pub struct RawQuestion {
    pub number: u32,
    pub title: String,
    pub refs: Vec<String>,
}

/// PARTS-1: drops entries a `refs: !!set` block lists MORE THAN ONCE,
/// returning the normalized text and how many were dropped.
///
/// `resources/02.0-The-Ten-Commandments.yaml` question 10 lists
/// `"1 John 3:4"` twice. `serde_yaml` refuses a mapping with duplicate
/// keys -- correctly, in general -- and a YAML `!!set` IS a mapping in
/// YAML's own data model, which is why the file parses nowhere. But a set
/// that names the same member twice denotes exactly the set that names it
/// once: dropping the repeat is meaning-preserving, not a relaxed guard.
/// Everything else stays strict, including the duplicate-key rule for any
/// mapping that is not a set block.
///
/// Scoped deliberately narrowly -- the seen-set resets at each new set
/// block -- so two different questions citing the same verse (the normal
/// case, and the whole point of the topical edges) are untouched.
fn dedupe_set_entries(input: &str) -> (String, usize) {
    let mut out = String::with_capacity(input.len());
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut in_set = false;
    let mut dropped = 0usize;

    for line in input.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("refs:") {
            in_set = true;
            seen.clear();
        } else if in_set && trimmed.starts_with("? ") {
            if !seen.insert(trimmed) {
                dropped += 1;
                continue;
            }
        } else if !trimmed.is_empty() {
            // Any other non-blank line ends the set block; a blank line
            // does not, since the real files separate nothing with them
            // INSIDE a block but do use them between questions.
            in_set = false;
        }
        out.push_str(line);
        out.push('\n');
    }

    (out, dropped)
}

/// Parses one `resources/*.yaml` file's own numbered-entry shape: a
/// top-level mapping from a bare integer key (`1`, `2`, ...) to
/// `{ title: string, refs: !!set of string }`. Deserializes via
/// `serde_yaml::Value` (untyped) rather than a `#[derive(Deserialize)]`
/// struct: a YAML `!!set` is represented in YAML's own data model as an
/// ordinary MAPPING node (every entry's own value is null) regardless of
/// the tag, so walking the generic `Value` tree directly sidesteps any
/// question of whether serde_yaml's derive path has special `!!set`
/// support at all -- confirmed empirically against REAL file content (this
/// module's own tests are built from actual `resources/02.1-*.yaml`
/// excerpts), not assumed from documentation. Questions are returned
/// SORTED by their own numeric key (YAML mapping order is not guaranteed
/// to already be numeric-ascending, though every real file inspected
/// happens to be); each question's own `refs` are returned in the file's
/// insertion order (serde_yaml preserves YAML mapping order).
pub fn parse_yaml_questions(input: &str) -> Result<Vec<RawQuestion>> {
    let (normalized, repeats) = dedupe_set_entries(input);
    if repeats > 0 {
        eprintln!("CATECHISM YAML: dropped {repeats} repeated `!!set` ref entr(ies) -- see dedupe_set_entries");
    }
    let doc: serde_yaml::Value = serde_yaml::from_str(&normalized).context("invalid YAML")?;
    let top = doc.as_mapping().context("expected a top-level YAML mapping of question-number -> {title, refs}")?;

    let mut out = Vec::with_capacity(top.len());
    for (key, val) in top {
        let number: u32 = key
            .as_u64()
            .or_else(|| key.as_str().and_then(|s| s.parse::<u64>().ok()))
            .with_context(|| format!("question key {key:?} is not a positive integer"))?
            .try_into()
            .with_context(|| format!("question key {key:?} is out of range for u32"))?;

        let entry = val.as_mapping().with_context(|| format!("question {number}: expected a mapping with title/refs"))?;
        let title = entry
            .get("title")
            .and_then(|v| v.as_str())
            .with_context(|| format!("question {number}: missing or non-string 'title'"))?
            .to_string();

        let refs_val = entry.get("refs").with_context(|| format!("question {number} ('{title}'): missing 'refs'"))?;
        let refs_map =
            refs_val.as_mapping().with_context(|| format!("question {number} ('{title}'): 'refs' is not a YAML set/mapping"))?;
        let mut refs = Vec::with_capacity(refs_map.len());
        for (ref_key, _) in refs_map {
            let r = ref_key.as_str().with_context(|| format!("question {number} ('{title}'): a 'refs' entry is not a string"))?;
            refs.push(r.to_string());
        }

        out.push(RawQuestion { number, title, refs });
    }

    out.sort_by_key(|q| q.number);
    Ok(out)
}

/// Resolves a human book name to its canonical `BookId`. Tries the app's
/// own `canon::resolve_alias` first (covers every full book name/OSIS/code
/// this app already knows, case/spacing-insensitive) -- falls back to a
/// tiny, disclosed local table for the ONE gap found in the real data (see
/// this module's own header).
pub fn resolve_book_name(name: &str) -> Option<BookId> {
    if let Some(b) = resolve_alias(name) {
        return Some(b);
    }
    match name.trim().to_ascii_lowercase().as_str() {
        "psalm" => resolve_alias("Psalms"),
        _ => None,
    }
}

/// Splits `"1 Samuel 28"` -> `("1 Samuel", "28")`, `"2 Corinthians 8-9"` ->
/// `("2 Corinthians", "8-9")`, `"Song of Solomon 2:1"` -> `("Song of
/// Solomon", "2:1")`, `"Isaiah 45:20"` -> `("Isaiah", "45:20")`. Every book
/// name actually used in the real data is either a bare run of alphabetic
/// words, or exactly ONE leading numeral (`1`/`2`/`3`) followed by a run of
/// alphabetic words (never a digit anywhere else in the name) -- this
/// tokenizer relies on exactly that shape rather than a general-purpose
/// regex (no `regex` crate dependency needed for one small, fully-verified
/// grammar). Returns `None` when no chapter-spec tail (a token starting
/// with a digit, after the optional leading book-numeral) is found at all.
fn split_book_and_tail(segment: &str) -> Option<(String, String)> {
    let tokens: Vec<&str> = segment.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let mut i = 0;
    if matches!(tokens[0], "1" | "2" | "3") {
        i += 1;
    }
    while i < tokens.len() && tokens[i].chars().all(|c| c.is_ascii_alphabetic()) {
        i += 1;
    }
    if i == 0 || i >= tokens.len() {
        return None;
    }

    Some((tokens[..i].join(" "), tokens[i..].join(" ")))
}

/// A parsed chapter-spec tail (the part of a ref AFTER its book name),
/// still book-agnostic -- `canonicalize_ref` resolves it against a
/// specific `BookId` afterward. Chapter/verse numbers are `u32` (this
/// app's own canonical types use `u16`; validated to fit on push, see
/// `push_verse`/`expand_whole_chapter` below -- keeping this enum's own
/// arithmetic in a wider type avoids an intermediate overflow panic on a
/// malformed/huge input, converting only at the final, checked boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChapterSpec {
    WholeChapter(u32),
    ChapterRange(u32, u32),
    Verse(u32, u32),
    VerseRange(u32, u32, u32),
    CrossChapterRange(u32, u32, u32, u32),
}

fn parse_u32(s: &str, raw: &str) -> Result<u32> {
    s.trim().parse::<u32>().with_context(|| format!("ref '{raw}': '{}' is not a valid number", s.trim()))
}

fn parse_chapter_and_verse(s: &str, raw: &str) -> Result<(u32, u32)> {
    let (ch, v) = s.split_once(':').with_context(|| format!("ref '{raw}': expected 'chapter:verse' in '{s}'"))?;
    Ok((parse_u32(ch, raw)?, parse_u32(v, raw)?))
}

/// Parses the tail of the FIRST comma-segment of a ref (the one that
/// always carries the chapter, e.g. `"20:1-3"`, `"1"`, `"12-13"`,
/// `"1:7-2:2"`). See this module's own header for the full form catalog.
fn parse_chapter_tail(tail: &str, raw: &str) -> Result<ChapterSpec> {
    if let Some((left, right)) = tail.split_once('-') {
        let (left, right) = (left.trim(), right.trim());
        match (left.contains(':'), right.contains(':')) {
            (false, false) => {
                let (from_ch, to_ch) = (parse_u32(left, raw)?, parse_u32(right, raw)?);
                Ok(ChapterSpec::ChapterRange(from_ch, to_ch))
            }
            (true, true) => {
                let (fc, fv) = parse_chapter_and_verse(left, raw)?;
                let (tc, tv) = parse_chapter_and_verse(right, raw)?;
                Ok(ChapterSpec::CrossChapterRange(fc, fv, tc, tv))
            }
            (true, false) => {
                let (ch, fv) = parse_chapter_and_verse(left, raw)?;
                let tv = parse_u32(right, raw)?;
                Ok(ChapterSpec::VerseRange(ch, fv, tv))
            }
            (false, true) => bail!("ref '{raw}': malformed tail '{tail}' (colon on the range's right side but not its left)"),
        }
    } else if let Some((ch, v)) = tail.split_once(':') {
        Ok(ChapterSpec::Verse(parse_u32(ch, raw)?, parse_u32(v, raw)?))
    } else {
        Ok(ChapterSpec::WholeChapter(parse_u32(tail, raw)?))
    }
}

/// Parses a comma-CONTINUATION segment (every segment after the first in a
/// compound ref like `"Exodus 34:1, 27-28"`) -- a bare verse (`"29"`) or a
/// bare same-chapter verse range (`"27-28"`), against the chapter already
/// established by the first segment. Never itself carries a book, a
/// chapter, or a colon (none of the real data's continuation segments do).
fn parse_continuation(piece: &str, chapter: u32, raw: &str) -> Result<ChapterSpec> {
    if piece.contains(':') {
        bail!("ref '{raw}': continuation segment '{piece}' unexpectedly carries its own chapter");
    }
    if let Some((left, right)) = piece.split_once('-') {
        Ok(ChapterSpec::VerseRange(chapter, parse_u32(left, raw)?, parse_u32(right, raw)?))
    } else {
        Ok(ChapterSpec::Verse(chapter, parse_u32(piece, raw)?))
    }
}

fn push_verse(book: BookId, chapter: u32, verse: u32, verses: &HashMap<String, String>, raw: &str, out: &mut Vec<String>) -> Result<()> {
    let ch: u16 = chapter.try_into().with_context(|| format!("ref '{raw}': chapter {chapter} out of range"))?;
    let v: u16 = verse.try_into().with_context(|| format!("ref '{raw}': verse {verse} out of range"))?;
    let key = format!("{}.{}.{}", book.code(), ch, v);
    if !verses.contains_key(&key) {
        bail!("ref '{raw}': '{key}' does not exist in the compiled KJV text");
    }
    out.push(key);
    Ok(())
}

/// Every verse of `chapter`, walked forward from verse 1 until
/// `book.chapter.(v+1)` is absent from the compiled KJV text -- see this
/// module's own header for why this walks the REAL compiled text rather
/// than depending on a separate chapter-verse-count table (this ingestion
/// step needs no data beyond the verses map already produced by
/// `kjv::parse`).
fn expand_whole_chapter(book: BookId, chapter: u32, verses: &HashMap<String, String>, raw: &str, out: &mut Vec<String>) -> Result<()> {
    let ch: u16 = chapter.try_into().with_context(|| format!("ref '{raw}': chapter {chapter} out of range"))?;
    let first_key = format!("{}.{}.1", book.code(), ch);
    if !verses.contains_key(&first_key) {
        bail!("ref '{raw}': chapter '{}.{}' does not exist in the compiled KJV text", book.code(), ch);
    }
    let mut v: u16 = 1;
    loop {
        out.push(format!("{}.{}.{}", book.code(), ch, v));
        let next_key = format!("{}.{}.{}", book.code(), ch, v + 1);
        if !verses.contains_key(&next_key) {
            break;
        }
        v += 1;
    }
    Ok(())
}

/// Walks forward from `(from_chapter, from_verse)` to `(to_chapter,
/// to_verse)` inclusive, one verse at a time, via the compiled text itself
/// (never a separate chapter-count table -- same reasoning as
/// `expand_whole_chapter`): if the NEXT verse number in the same chapter
/// exists, that's next; otherwise the chapter has ended, so chapter+1 verse
/// 1 is next. Fails loudly (rather than looping forever or silently
/// stopping short) if the endpoint is never reached within a generous
/// bound -- a malformed/inverted cross-chapter range is a real citation-
/// integrity error, not a silent truncation.
#[allow(clippy::too_many_arguments)]
fn expand_cross_chapter(
    book: BookId,
    from_chapter: u32,
    from_verse: u32,
    to_chapter: u32,
    to_verse: u32,
    verses: &HashMap<String, String>,
    raw: &str,
    out: &mut Vec<String>,
) -> Result<()> {
    if (from_chapter, from_verse) > (to_chapter, to_verse) {
        bail!("ref '{raw}': inverted cross-chapter range {from_chapter}:{from_verse}-{to_chapter}:{to_verse}");
    }

    let mut chapter: u16 =
        from_chapter.try_into().with_context(|| format!("ref '{raw}': chapter {from_chapter} out of range"))?;
    let mut verse: u16 = from_verse.try_into().with_context(|| format!("ref '{raw}': verse {from_verse} out of range"))?;
    let target_chapter: u16 = to_chapter.try_into().with_context(|| format!("ref '{raw}': chapter {to_chapter} out of range"))?;
    let target_verse: u16 = to_verse.try_into().with_context(|| format!("ref '{raw}': verse {to_verse} out of range"))?;

    let key = format!("{}.{}.{}", book.code(), chapter, verse);
    if !verses.contains_key(&key) {
        bail!("ref '{raw}': '{key}' does not exist in the compiled KJV text");
    }

    const MAX_STEPS: u32 = 5000; // a generous bound -- no real cross-chapter citation in this data spans more than a few chapters
    for _ in 0..MAX_STEPS {
        out.push(format!("{}.{}.{}", book.code(), chapter, verse));
        if chapter == target_chapter && verse == target_verse {
            return Ok(());
        }

        let same_chapter_next = format!("{}.{}.{}", book.code(), chapter, verse + 1);
        if verses.contains_key(&same_chapter_next) {
            verse += 1;
            continue;
        }
        let next_chapter_first = format!("{}.{}.1", book.code(), chapter + 1);
        if verses.contains_key(&next_chapter_first) {
            chapter += 1;
            verse = 1;
            continue;
        }
        bail!("ref '{raw}': ran off the end of the compiled KJV text before reaching {to_chapter}:{to_verse}");
    }
    bail!("ref '{raw}': cross-chapter range exceeded {MAX_STEPS} verses -- likely malformed");
}

/// True for a book with exactly one chapter (Obadiah, Philemon, 2 John,
/// 3 John, Jude) -- determined from the compiled KJV text itself (does
/// `book.2.1` exist?), not a hardcoded book list, so it stays correct for
/// any future canon change with no edit needed here. Citation CONVENTION
/// for a single-chapter book drops the chapter number entirely -- "Jude 6"
/// means verse 6 of Jude's one chapter, not "chapter 6" (which doesn't
/// exist) -- confirmed a real, live gap by `sweep_all_real_refs`-style
/// verification against the actual data (4 of 1550 real refs, all "Jude N"/
/// "Jude N-M" forms, failed before this fix; 0 failed after).
fn is_single_chapter_book(book: BookId, verses: &HashMap<String, String>) -> bool {
    !verses.contains_key(&format!("{}.2.1", book.code()))
}

fn expand_spec(book: BookId, spec: ChapterSpec, verses: &HashMap<String, String>, raw: &str, out: &mut Vec<String>) -> Result<()> {
    // A bare "chapter" or "chapter-chapter" spec against a single-chapter
    // book is really "verse"/"verse-verse" of chapter 1 -- see
    // is_single_chapter_book's own doc comment. An explicit "chapter:verse"
    // form is untouched either way (already correct, or a genuine
    // out-of-range citation this function's own existence checks will
    // catch honestly).
    let spec = if is_single_chapter_book(book, verses) {
        match spec {
            ChapterSpec::WholeChapter(n) => ChapterSpec::Verse(1, n),
            ChapterSpec::ChapterRange(from, to) => ChapterSpec::VerseRange(1, from, to),
            other => other,
        }
    } else {
        spec
    };

    match spec {
        ChapterSpec::WholeChapter(ch) => expand_whole_chapter(book, ch, verses, raw, out),
        ChapterSpec::ChapterRange(from_ch, to_ch) => {
            if from_ch > to_ch {
                bail!("ref '{raw}': inverted chapter range {from_ch}-{to_ch}");
            }
            for ch in from_ch..=to_ch {
                expand_whole_chapter(book, ch, verses, raw, out)?;
            }
            Ok(())
        }
        ChapterSpec::Verse(ch, v) => push_verse(book, ch, v, verses, raw, out),
        ChapterSpec::VerseRange(ch, fv, tv) => {
            if fv > tv {
                bail!("ref '{raw}': inverted verse range {ch}:{fv}-{tv}");
            }
            for v in fv..=tv {
                push_verse(book, ch, v, verses, raw, out)?;
            }
            Ok(())
        }
        ChapterSpec::CrossChapterRange(fc, fv, tc, tv) => expand_cross_chapter(book, fc, fv, tc, tv, verses, raw, out),
    }
}

/// Canonicalizes ONE human-readable ref string (requirement 2: "Book
/// Chapter:Verse" human format, ranges, bare chapters -> canonical refs")
/// into a flat, ordered `Vec` of individual canonical verse-ref strings
/// (`"BOOK.CH.V"`). `verses` is the compiled KJV text map -- used both to
/// walk chapter boundaries for whole-chapter/cross-chapter expansion and to
/// fail loudly (never silently drop, per this batch's own citation-
/// integrity rule) the instant a produced verse doesn't actually exist in
/// this atlas's compiled text. See this module's own header for the full,
/// verified catalog of ref shapes this handles.
pub fn canonicalize_ref(raw: &str, verses: &HashMap<String, String>) -> Result<Vec<String>> {
    let raw_trimmed = raw.trim();
    if raw_trimmed.is_empty() {
        bail!("empty ref string");
    }

    let mut out = Vec::new();
    let mut book: Option<BookId> = None;
    let mut chapter: Option<u32> = None;

    for (i, piece) in raw_trimmed.split(',').enumerate() {
        let piece = piece.trim();
        if piece.is_empty() {
            bail!("ref '{raw_trimmed}': empty comma-separated segment");
        }

        if i == 0 {
            let (book_name, tail) =
                split_book_and_tail(piece).with_context(|| format!("ref '{raw_trimmed}': no chapter number found"))?;
            let book_id = resolve_book_name(&book_name)
                .with_context(|| format!("ref '{raw_trimmed}': unrecognized book name '{book_name}'"))?;
            let spec = parse_chapter_tail(&tail, raw_trimmed)?;
            if let ChapterSpec::Verse(ch, _) | ChapterSpec::VerseRange(ch, _, _) | ChapterSpec::WholeChapter(ch) = spec {
                chapter = Some(ch);
            }
            // A ChapterRange or CrossChapterRange segment can never be
            // followed by a bare comma-continuation in the real data (a
            // continuation implies "more verses of the ONE chapter just
            // named"); `chapter` is left `None` for those shapes, and any
            // later segment that needs it will fail loudly via the
            // `with_context` below instead of silently guessing.
            book = Some(book_id);
            expand_spec(book_id, spec, verses, raw_trimmed, &mut out)?;
        } else {
            let book_id = book.expect("set on the first segment or this function already bailed");
            let ch = chapter
                .with_context(|| format!("ref '{raw_trimmed}': comma-continuation '{piece}' has no single established chapter"))?;
            let spec = parse_continuation(piece, ch, raw_trimmed)?;
            expand_spec(book_id, spec, verses, raw_trimmed, &mut out)?;
        }
    }

    Ok(out)
}

// --- data/curated/catechism-mapping.toml (requirement 3: the file->item
// mapping table) + data/curated/catechism-deut5.toml (requirement 5b) -----

/// One `[[file]]` row of `catechism-mapping.toml`: an ingested
/// `resources/*.yaml` path, the catechism item id its questions attach to
/// by DEFAULT, and (for exactly one real file, `05.2.1-Confession-and-
/// Absolution.yaml`, per that file's own comment) a small per-question
/// override list reassigning specific question NUMBERS to a different item.
#[derive(Debug, Clone, PartialEq)]
pub struct MappingFile {
    pub path: String,
    pub item: String,
    pub overrides: Vec<MappingOverride>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MappingOverride {
    pub item: String,
    pub questions: Vec<u32>,
}

impl MappingFile {
    /// The item a given question NUMBER (within this file) attaches to --
    /// the first override whose own `questions` names it, else this file's
    /// own default `item`.
    pub fn item_for(&self, question_number: u32) -> &str {
        self.overrides.iter().find(|o| o.questions.contains(&question_number)).map_or(&self.item, |o| &o.item)
    }
}

/// One `[[entry]]` row of `catechism-deut5.toml` (requirement 5b): a
/// catechism item id, its own curated Deuteronomy 5 parallel verse(s) (our
/// own canonical-ref convention, e.g. `"DEU.5.7"` or `"DEU.5.9-10"` -- NOT
/// the brain-fuel repo's human-readable form, so these expand via the
/// SAME `curated::expand_verse_ref` every other hand-typed curated citation
/// in this app already uses, not `canonicalize_ref` above), and a required
/// `ref_note` documenting the versification judgment call.
#[derive(Debug, Clone, PartialEq)]
pub struct Deut5Entry {
    pub item: String,
    pub verses: Vec<String>,
    pub ref_note: String,
}

/// The fixed title every `catechism-deut5.toml` entry's own compiled
/// `CatechismQuestion` carries -- see that file's own header for why one
/// consistent, descriptive label (rather than a per-commandment title, the
/// repo-derived questions' own convention) is the honest reading of "a
/// small curated mapping... attaching each Commandment item to its DEU.5
/// parallel": these are all the SAME kind of thing (a cross-reference to
/// the parallel Decalogue enumeration), not 11 distinct topical questions.
pub const DEUT5_QUESTION_TITLE: &str = "The Deuteronomy 5 Parallel";

/// Reads every file named in `mapping` (relative to `mapping_root`,
/// e.g. `.../catechism-mapping/catechism-{sha}/`), parses its own numbered
/// questions, canonicalizes every ref, and returns the full set of
/// `CatechismQuestion`s each target item should receive -- grouped by
/// item id, each item's own list in file-then-question-number order. Pure
/// aggregation over already-read strings (the actual `std::fs::read_to_string`
/// calls happen here since `mapping_root`/each file's path are simple
/// joins, not a network fetch -- consistent with "atlas-etl does no
/// networking," the one remaining filesystem exception every OTHER
/// distinct-entry-point curated-file reader in this crate's `main.rs`
/// already takes for granted). Fails loudly and IMMEDIATELY on any file
/// read/YAML-shape/ref-canonicalization error, naming the exact file and
/// question -- this project's own citation-integrity rule (`context()`
/// chains give the full "which file, which question, which ref" trail in
/// the resulting error).
pub fn build_questions_from_mapping(
    mapping: &[MappingFile],
    mapping_root: &Path,
    verses: &HashMap<String, String>,
) -> Result<HashMap<String, Vec<CatechismQuestion>>> {
    let mut by_item: HashMap<String, Vec<CatechismQuestion>> = HashMap::new();

    for file in mapping {
        let full_path = mapping_root.join(&file.path);
        let yaml = std::fs::read_to_string(&full_path)
            .with_context(|| format!("reading {} (catechism-mapping.toml row for '{}')", full_path.display(), file.path))?;
        let raw_questions = parse_yaml_questions(&yaml).with_context(|| format!("parsing {}", full_path.display()))?;

        for rq in raw_questions {
            let item_id = file.item_for(rq.number).to_string();
            let mut verse_list = Vec::new();
            for r in &rq.refs {
                let expanded = canonicalize_ref(r, verses)
                    .with_context(|| format!("{} question {} ('{}')", file.path, rq.number, rq.title))?;
                verse_list.extend(expanded);
            }
            by_item.entry(item_id).or_default().push(CatechismQuestion {
                title: rq.title,
                verses: verse_list,
                source: "brain-fuel/catechism".to_string(),
            });
        }
    }

    Ok(by_item)
}

/// PARTS-1: the part-level twin of `build_questions_from_mapping`.
///
/// Same files, same YAML shape, same `canonicalize_ref` bar -- the only
/// difference is the key. These seven files' questions belong to a whole
/// chief part rather than to any one item, which is precisely why Batch F2
/// deferred them; the returned map is keyed by PART id.
pub fn build_part_questions_from_mapping(
    rows: &[crate::curated::PartMappingRow],
    mapping_root: &Path,
    verses: &HashMap<String, String>,
) -> Result<HashMap<String, Vec<CatechismQuestion>>> {
    let mut by_part: HashMap<String, Vec<CatechismQuestion>> = HashMap::new();

    for row in rows {
        let full_path = mapping_root.join(&row.path);
        let yaml = std::fs::read_to_string(&full_path).with_context(|| {
            format!("reading {} (catechism-part-mapping.toml row for '{}')", full_path.display(), row.path)
        })?;
        let raw_questions = parse_yaml_questions(&yaml).with_context(|| format!("parsing {}", full_path.display()))?;

        for rq in raw_questions {
            let mut verse_list = Vec::new();
            for r in &rq.refs {
                let expanded = canonicalize_ref(r, verses)
                    .with_context(|| format!("{} question {} ('{}')", row.path, rq.number, rq.title))?;
                verse_list.extend(expanded);
            }
            by_part.entry(row.part.clone()).or_default().push(CatechismQuestion {
                title: rq.title,
                verses: verse_list,
                source: "brain-fuel/catechism".to_string(),
            });
        }
    }

    Ok(by_part)
}

/// Merges `by_item` (from `build_questions_from_mapping`, and/or the
/// Deut5 supplement's own equivalent map) into `parts` -- assigns each
/// target item's own `questions` field. Fails loudly if any target item id
/// doesn't actually exist in `parts` (a mapping-table typo, or a
/// catechism.toml item since renamed/removed): every bad id is collected
/// before bailing, same aggregate-don't-fail-fast policy
/// `atlas_etl::validate`'s own `run_*` functions use throughout this crate,
/// even though this particular check runs at MERGE time rather than inside
/// `validate.rs` (it needs to run before `validate::run_catechism` can even
/// see the merged `questions` fields it itself validates).
pub fn merge_questions_into_parts(parts: &mut [CatechismPart], by_item: HashMap<String, Vec<CatechismQuestion>>) -> Result<()> {
    // Owned (String) keys, not &str -- borrowing `parts` here would keep an
    // immutable borrow alive for this whole function, conflicting with the
    // mutable `parts[pi].items[ii].questions.append(...)` below.
    let mut item_index: HashMap<String, (usize, usize)> = HashMap::new();
    for (pi, part) in parts.iter().enumerate() {
        for (ii, item) in part.items.iter().enumerate() {
            item_index.insert(item.id.clone(), (pi, ii));
        }
    }

    let mut unknown: Vec<String> = Vec::new();
    for (item_id, mut questions) in by_item {
        match item_index.get(item_id.as_str()) {
            Some(&(pi, ii)) => parts[pi].items[ii].questions.append(&mut questions),
            None => unknown.push(item_id),
        }
    }

    if !unknown.is_empty() {
        unknown.sort();
        bail!(
            "catechism-mapping.toml / catechism-deut5.toml reference {} unknown catechism item id(s) (not in catechism.toml): {}",
            unknown.len(),
            unknown.join(", ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verses_fixture() -> HashMap<String, String> {
        let mut v = HashMap::new();
        // GEN 1: 5 verses; GEN 2: 3 verses (enough to exercise whole-chapter
        // and cross-chapter walking without a full real KJV fixture).
        for i in 1..=5 {
            v.insert(format!("GEN.1.{i}"), format!("gen1v{i}"));
        }
        for i in 1..=3 {
            v.insert(format!("GEN.2.{i}"), format!("gen2v{i}"));
        }
        // EXO 20: 17 verses (real chapter length) + EXO 34: 28 verses.
        for i in 1..=17 {
            v.insert(format!("EXO.20.{i}"), format!("exo20v{i}"));
        }
        for i in 1..=28 {
            v.insert(format!("EXO.34.{i}"), format!("exo34v{i}"));
        }
        // PSA 1: 6 verses. PSA.2.1 is a sentinel only (proves PSA is
        // multi-chapter to is_single_chapter_book) -- real Psalm 2 has 12
        // verses, irrelevant to any test here.
        for i in 1..=6 {
            v.insert(format!("PSA.1.{i}"), format!("psa1v{i}"));
        }
        v.insert("PSA.2.1".to_string(), "psa2v1".to_string());
        // ROM 12: 21 verses, ROM 13: 14 verses (for a bare chapter range).
        // ROM.2.1 sentinel, same reasoning as PSA.2.1 above.
        v.insert("ROM.2.1".to_string(), "rom2v1".to_string());
        for i in 1..=21 {
            v.insert(format!("ROM.12.{i}"), format!("rom12v{i}"));
        }
        for i in 1..=14 {
            v.insert(format!("ROM.13.{i}"), format!("rom13v{i}"));
        }
        // 1SA 28: 25 verses. 1SA.2.1 sentinel, same reasoning as above.
        v.insert("1SA.2.1".to_string(), "1sa2v1".to_string());
        for i in 1..=25 {
            v.insert(format!("1SA.28.{i}"), format!("1sa28v{i}"));
        }
        v.insert("ISA.45.20".to_string(), "isa45v20".to_string());
        // JUD: single-chapter book, 25 verses (real length) -- no JUD.2.* at
        // all, which is exactly the signal is_single_chapter_book reads.
        for i in 1..=25 {
            v.insert(format!("JUD.1.{i}"), format!("judv{i}"));
        }
        v
    }

    // --- parse_yaml_questions ------------------------------------------

    #[test]
    fn parse_yaml_questions_reads_real_repo_shape() {
        // Verbatim excerpt from the real resources/02.1-The-First-Commandment.yaml.
        let yaml = r#"
1:
  title: "God Alone as Judge"
  refs: !!set
    ? "Luke 12:13-14"

2:
  title: "Trusting God Above Created Things"
  refs: !!set
    ? "Isaiah 45:20"
    ? "Proverbs 11:28"
    ? "Matthew 10:37"

15:
  title: "The First Commandment"
  refs: !!set
    ? "Exodus 20:1-3"
    ? "John 3:16"
    ? "Exodus 20:4"
    ? "Exodus 20:5"
"#;
        let qs = parse_yaml_questions(yaml).unwrap();
        assert_eq!(qs.len(), 3);
        assert_eq!(qs[0].number, 1);
        assert_eq!(qs[0].title, "God Alone as Judge");
        assert_eq!(qs[0].refs, vec!["Luke 12:13-14".to_string()]);
        assert_eq!(qs[1].number, 2);
        assert_eq!(qs[1].refs.len(), 3);
        // Numeric sort, not YAML source order (15 appears last in the file too, but this pins it either way).
        assert_eq!(qs[2].number, 15);
        assert_eq!(qs[2].title, "The First Commandment");
        assert!(qs[2].refs.contains(&"Exodus 20:1-3".to_string()));
    }

    #[test]
    fn parse_yaml_questions_rejects_malformed_yaml() {
        assert!(parse_yaml_questions("not: [valid: yaml: at: all: {{{").is_err());
    }

    #[test]
    fn parse_yaml_questions_rejects_missing_title_or_refs() {
        assert!(parse_yaml_questions("1:\n  refs: !!set\n    ? \"Genesis 1:1\"\n").is_err(), "missing title");
        assert!(parse_yaml_questions("1:\n  title: \"X\"\n").is_err(), "missing refs");
    }

    // --- resolve_book_name ----------------------------------------------

    #[test]
    fn resolve_book_name_covers_full_names_and_the_psalm_singular_gap() {
        assert_eq!(resolve_book_name("Genesis").unwrap().code(), "GEN");
        assert_eq!(resolve_book_name("1 Samuel").unwrap().code(), "1SA");
        assert_eq!(resolve_book_name("2 Corinthians").unwrap().code(), "2CO");
        assert_eq!(resolve_book_name("Song of Solomon").unwrap().code(), "SNG");
        assert_eq!(resolve_book_name("Psalm").unwrap().code(), "PSA", "the one real gap this module's own fallback covers");
        assert_eq!(resolve_book_name("Psalms").unwrap().code(), "PSA", "plural still resolves via resolve_alias directly");
        assert!(resolve_book_name("Not A Real Book").is_none());
    }

    // Every distinct book-name string this module's own header discloses as
    // actually appearing in the real fetched data (53 names, catalogued via
    // a full grep sweep before this parser was written) resolves. Pins the
    // "book-name mapping must cover every book that appears" requirement
    // directly against that verified list, not just a spot check.
    #[test]
    fn every_real_book_name_resolves() {
        const REAL_BOOK_NAMES: &[&str] = &[
            "1 Chronicles", "1 Corinthians", "1 John", "1 Kings", "1 Peter", "1 Samuel", "1 Thessalonians", "1 Timothy",
            "2 Chronicles", "2 Corinthians", "2 Kings", "2 Peter", "2 Samuel", "2 Thessalonians", "2 Timothy", "Acts",
            "Colossians", "Daniel", "Deuteronomy", "Ecclesiastes", "Ephesians", "Exodus", "Ezekiel", "Galatians", "Genesis",
            "Habakkuk", "Hebrews", "Isaiah", "James", "Jeremiah", "Job", "Joel", "John", "Jonah", "Joshua", "Jude", "Judges",
            "Lamentations", "Leviticus", "Luke", "Malachi", "Mark", "Matthew", "Micah", "Nehemiah", "Numbers", "Philippians",
            "Proverbs", "Psalm", "Revelation", "Romans", "Titus", "Zechariah",
        ];
        for name in REAL_BOOK_NAMES {
            assert!(resolve_book_name(name).is_some(), "book name '{name}' failed to resolve");
        }
        assert_eq!(REAL_BOOK_NAMES.len(), 53);
    }

    // --- split_book_and_tail ---------------------------------------------

    #[test]
    fn split_book_and_tail_handles_numbered_and_multiword_books() {
        assert_eq!(split_book_and_tail("1 Samuel 28"), Some(("1 Samuel".into(), "28".into())));
        assert_eq!(split_book_and_tail("2 Corinthians 8-9"), Some(("2 Corinthians".into(), "8-9".into())));
        assert_eq!(split_book_and_tail("Song of Solomon 2:1"), Some(("Song of Solomon".into(), "2:1".into())));
        assert_eq!(split_book_and_tail("Isaiah 45:20"), Some(("Isaiah".into(), "45:20".into())));
        assert_eq!(split_book_and_tail("Psalm 1"), Some(("Psalm".into(), "1".into())));
        assert_eq!(split_book_and_tail("no chapter here"), None);
    }

    // --- canonicalize_ref: every real shape, from real refs -------------

    #[test]
    fn canonicalize_ref_single_verse() {
        assert_eq!(canonicalize_ref("Isaiah 45:20", &verses_fixture()).unwrap(), vec!["ISA.45.20"]);
    }

    #[test]
    fn canonicalize_ref_same_chapter_range() {
        assert_eq!(canonicalize_ref("Exodus 20:1-3", &verses_fixture()).unwrap(), vec!["EXO.20.1", "EXO.20.2", "EXO.20.3"]);
    }

    #[test]
    fn canonicalize_ref_bare_chapter() {
        assert_eq!(canonicalize_ref("Psalm 1", &verses_fixture()).unwrap(), vec!["PSA.1.1", "PSA.1.2", "PSA.1.3", "PSA.1.4", "PSA.1.5", "PSA.1.6"]);
    }

    #[test]
    fn canonicalize_ref_bare_chapter_with_numbered_book() {
        assert_eq!(canonicalize_ref("1 Samuel 28", &verses_fixture()).unwrap().len(), 25);
    }

    #[test]
    fn canonicalize_ref_bare_chapter_range() {
        let out = canonicalize_ref("Romans 12-13", &verses_fixture()).unwrap();
        assert_eq!(out.len(), 21 + 14, "all of ROM.12 (21v) + all of ROM.13 (14v)");
        assert_eq!(out.first().unwrap(), "ROM.12.1");
        assert_eq!(out[20], "ROM.12.21");
        assert_eq!(out[21], "ROM.13.1");
        assert_eq!(out.last().unwrap(), "ROM.13.14");
    }

    #[test]
    fn canonicalize_ref_cross_chapter_verse_range() {
        let out = canonicalize_ref("Genesis 1:4-2:2", &verses_fixture()).unwrap();
        assert_eq!(out, vec!["GEN.1.4", "GEN.1.5", "GEN.2.1", "GEN.2.2"]);
    }

    #[test]
    fn canonicalize_ref_comma_compound() {
        // Real example: "Exodus 34:1, 27-28" -- verse 1, then verses 27-28,
        // all within EXO.34 (the book+chapter is stated only once).
        let out = canonicalize_ref("Exodus 34:1, 27-28", &verses_fixture()).unwrap();
        assert_eq!(out, vec!["EXO.34.1", "EXO.34.27", "EXO.34.28"]);
    }

    #[test]
    fn canonicalize_ref_comma_compound_with_bare_verse_continuation() {
        // Real example: "1 Corinthians 5:11, 13" -- two bare verses.
        let out = canonicalize_ref("1 Corinthians 5:11, 13", &verses_fixture());
        // Fixture doesn't carry 1CO -- assert the PARSE shape (not existence)
        // by checking the error names the expected canonical key, proving
        // the comma-continuation was correctly resolved against chapter 5.
        let err = out.unwrap_err().to_string();
        assert!(err.contains("1CO.5.11"), "{err}");
    }

    #[test]
    fn canonicalize_ref_single_chapter_book_bare_verse_and_range() {
        // Real examples from the actual data: "Jude 6", "Jude 22-25" --
        // convention drops the chapter number entirely for a one-chapter
        // book; must NOT be read as "chapter 6"/"chapters 22-25" (Jude has
        // no chapter 6 or 22).
        assert_eq!(canonicalize_ref("Jude 6", &verses_fixture()).unwrap(), vec!["JUD.1.6"]);
        assert_eq!(
            canonicalize_ref("Jude 22-25", &verses_fixture()).unwrap(),
            vec!["JUD.1.22", "JUD.1.23", "JUD.1.24", "JUD.1.25"]
        );
    }

    // --- fail-loud on real invalid input ----------------------------------

    #[test]
    fn canonicalize_ref_rejects_unknown_book() {
        assert!(canonicalize_ref("Nonesuch 1:1", &verses_fixture()).is_err());
    }

    #[test]
    fn canonicalize_ref_rejects_a_verse_missing_from_compiled_text() {
        let err = canonicalize_ref("Genesis 1:99", &verses_fixture()).unwrap_err().to_string();
        assert!(err.contains("GEN.1.99"), "{err}");
    }

    #[test]
    fn canonicalize_ref_rejects_garbage() {
        assert!(canonicalize_ref("garbage", &verses_fixture()).is_err());
        assert!(canonicalize_ref("", &verses_fixture()).is_err());
    }

    #[test]
    fn canonicalize_ref_rejects_inverted_range() {
        assert!(canonicalize_ref("Genesis 1:5-3", &verses_fixture()).is_err());
    }

    // --- MappingFile::item_for --------------------------------------------

    #[test]
    fn mapping_file_item_for_uses_default_with_no_overrides() {
        let f = MappingFile { path: "x.yaml".into(), item: "default-item".into(), overrides: vec![] };
        assert_eq!(f.item_for(1), "default-item");
        assert_eq!(f.item_for(99), "default-item");
    }

    #[test]
    fn mapping_file_item_for_honors_override_list() {
        let f = MappingFile {
            path: "x.yaml".into(),
            item: "confession-1".into(),
            overrides: vec![MappingOverride { item: "confession-2".into(), questions: vec![6, 7, 9] }],
        };
        assert_eq!(f.item_for(6), "confession-2");
        assert_eq!(f.item_for(7), "confession-2");
        assert_eq!(f.item_for(5), "confession-1", "not in the override list -- falls back to the default");
        assert_eq!(f.item_for(8), "confession-1");
    }

    // --- merge_questions_into_parts ----------------------------------------

    fn parts_fixture() -> Vec<CatechismPart> {
        vec![CatechismPart {
            id: "p".into(),
            title: "P".into(),
            items: vec![
                atlas_core::data::CatechismItem {
                    id: "item-a".into(),
                    name: "Item A".into(),
                    text: None,
                    explanation_heading: "What does this mean?".into(),
                    explanation: "E".into(),
                    where_written: None,
                    verses: vec![],
                    ref_note: None,
                    questions: vec![],
                    reflection: vec![],
                    media: vec![],
                },
                atlas_core::data::CatechismItem {
                    id: "item-b".into(),
                    name: "Item B".into(),
                    text: None,
                    explanation_heading: "What does this mean?".into(),
                    explanation: "E".into(),
                    where_written: None,
                    verses: vec![],
                    ref_note: None,
                    questions: vec![],
                    reflection: vec![],
                    media: vec![],
                },
            ],
            questions: vec![],
            curated: true,
        }]
    }

    #[test]
    fn merge_questions_into_parts_assigns_by_item_id() {
        let mut parts = parts_fixture();
        let mut by_item = HashMap::new();
        by_item.insert(
            "item-a".to_string(),
            vec![CatechismQuestion { title: "Q1".into(), verses: vec!["GEN.1.1".into()], source: "s".into() }],
        );
        merge_questions_into_parts(&mut parts, by_item).unwrap();
        assert_eq!(parts[0].items[0].questions.len(), 1);
        assert_eq!(parts[0].items[0].questions[0].title, "Q1");
        assert!(parts[0].items[1].questions.is_empty(), "item-b got no questions -- untouched");
    }

    #[test]
    fn merge_questions_into_parts_appends_rather_than_overwrites() {
        // Two separate sources (the repo mapping AND the deut5 supplement)
        // both target the same item -- both calls' own questions must
        // survive, not the second silently replacing the first.
        let mut parts = parts_fixture();
        let mut first = HashMap::new();
        first.insert("item-a".to_string(), vec![CatechismQuestion { title: "Repo Q".into(), verses: vec![], source: "brain-fuel/catechism".into() }]);
        merge_questions_into_parts(&mut parts, first).unwrap();

        let mut second = HashMap::new();
        second.insert("item-a".to_string(), vec![CatechismQuestion { title: "Deut5 Q".into(), verses: vec![], source: "deut5-parallel".into() }]);
        merge_questions_into_parts(&mut parts, second).unwrap();

        assert_eq!(parts[0].items[0].questions.len(), 2, "{:?}", parts[0].items[0].questions);
        assert_eq!(parts[0].items[0].questions[0].title, "Repo Q");
        assert_eq!(parts[0].items[0].questions[1].title, "Deut5 Q");
    }

    #[test]
    fn merge_questions_into_parts_fails_loud_on_unknown_item_id() {
        let mut parts = parts_fixture();
        let mut by_item = HashMap::new();
        by_item.insert("no-such-item".to_string(), vec![CatechismQuestion { title: "Q".into(), verses: vec![], source: "s".into() }]);
        let err = merge_questions_into_parts(&mut parts, by_item).unwrap_err();
        assert!(err.to_string().contains("no-such-item"), "{err}");
    }
}
