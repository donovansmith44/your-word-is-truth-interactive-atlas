//! Ingests the vendored catechism verse-mapping repo: one YAML file per topic, whose question-level refs are
//! human-readable ("Exodus 20:1-3", "Psalm 1", "Romans 12-13", "1 John 1:7-2:2", "Exodus 34:1, 27-28") and
//! never cross a book boundary. Book names are full English; only the singular "Psalm" needs a local fallback.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use atlas_core::canon::resolve_alias;
use atlas_core::data::{CatechismPart, CatechismQuestion};
use atlas_core::refs::BookId;

/// One question-level entry, refs still in RAW human-readable form: canonicalization is a separate step, so a
/// YAML-shape failure and a ref failure are reported distinctly and a caller can attach its own context to each.
#[derive(Debug, Clone, PartialEq)]
pub struct RawQuestion {
    pub number: u32,
    pub title: String,
    pub refs: Vec<String>,
}

/// A YAML `!!set` is an ordinary MAPPING node in YAML's data model whatever its tag, so this walks the untyped
/// value tree rather than relying on a derive's set support. Questions come back SORTED by their numeric key,
/// since mapping order is not guaranteed to be ascending; each question's refs keep the file's own order.
pub fn parse_yaml_questions(input: &str) -> Result<Vec<RawQuestion>> {
    let doc: serde_yaml::Value = serde_yaml::from_str(input).context("invalid YAML")?;
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

/// Tries the atlas's own alias resolution first, which covers every full name, OSIS spelling and code it knows,
/// then a tiny local table for the one gap the real data has.
pub fn resolve_book_name(name: &str) -> Option<BookId> {
    if let Some(b) = resolve_alias(name) {
        return Some(b);
    }
    match name.trim().to_ascii_lowercase().as_str() {
        "psalm" => resolve_alias("Psalms"),
        _ => None,
    }
}

/// Splits a segment into its book name and the chapter-spec tail. Every book name in the real data is a run of
/// alphabetic words, optionally preceded by exactly one numeral, and never has a digit elsewhere, so this
/// tokenizer relies on that shape rather than pulling in a regex dependency for one verified grammar.
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

/// Chapter and verse numbers are wider here than the canonical types they become, and are checked on push:
/// keeping the intermediate arithmetic wide means a malformed, huge input cannot overflow before that check.
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

/// Parses the tail of the FIRST comma-segment of a ref, the one that always carries the chapter.
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

/// Parses a comma-CONTINUATION segment -- a bare verse or same-chapter verse range -- against the chapter the
/// first segment established. A continuation never carries a book, a chapter or a colon.
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

/// Every verse of the chapter, walked forward until the next verse is absent from the compiled text: the text
/// itself is the authority, so no separate chapter-verse-count table is needed.
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

/// Walks forward one verse at a time through the compiled text: if the next verse number in the chapter exists it is
/// next, otherwise the chapter has ended and the next chapter's verse 1 is. Fails loudly if the endpoint is never
/// reached within a generous bound: a malformed or inverted range is a citation-integrity error, not a truncation.
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

    const MAX_STEPS: u32 = 5000;
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

/// True for a book with exactly one chapter, determined by asking the compiled text whether chapter 2 verse 1
/// exists rather than from a hardcoded list. The citation CONVENTION for such a book drops the chapter
/// entirely, so "Jude 6" means verse 6 of its one chapter, never chapter 6.
fn is_single_chapter_book(book: BookId, verses: &HashMap<String, String>) -> bool {
    !verses.contains_key(&format!("{}.2.1", book.code()))
}

fn expand_spec(book: BookId, spec: ChapterSpec, verses: &HashMap<String, String>, raw: &str, out: &mut Vec<String>) -> Result<()> {
    // A bare chapter or chapter-range spec against a single-chapter book is really a verse or verse-range of
    // chapter 1. An explicit chapter:verse form is left untouched either way.
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

/// Canonicalizes ONE human-readable ref into a flat, ordered list of individual canonical verse refs, failing
/// loudly the instant a produced verse does not actually exist in the compiled text rather than dropping it.
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
            // A chapter-range or cross-chapter segment is never followed by a bare continuation in the real
            // data, so `chapter` stays `None` for those shapes and any later segment that needs it fails loudly
            // instead of guessing.
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

/// One row of the mapping table: an ingested file path, the catechism item its questions attach to by default,
/// and a small per-question override list reassigning specific question numbers to another item.
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
    /// The first override whose own question list names this number, else the file's default item.
    pub fn item_for(&self, question_number: u32) -> &str {
        self.overrides.iter().find(|o| o.questions.contains(&question_number)).map_or(&self.item, |o| &o.item)
    }
}

/// One supplement entry: an item id, its curated parallel verses in OUR OWN canonical convention rather than
/// the vendored repo's human-readable form -- so they expand through the curated expansion, not this module's
/// canonicalizer -- and a required note documenting the versification judgment.
#[derive(Debug, Clone, PartialEq)]
pub struct Deut5Entry {
    pub item: String,
    pub verses: Vec<String>,
    pub ref_note: String,
}

/// The fixed title every supplement entry's question carries: these are all the same kind of cross-reference to
/// the parallel enumeration, not distinct topical questions, so one consistent label is the honest reading.
pub const DEUT5_QUESTION_TITLE: &str = "The Deuteronomy 5 Parallel";

/// Reads every file the mapping names, parses its questions and canonicalizes every ref, grouped by item id
/// with each item's list in file-then-question-number order. Fails immediately on any read, shape or ref error,
/// naming the exact file and question.
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

/// Assigns each target item's questions, failing loudly when a target id does not exist in the parts -- a
/// mapping typo, or an item since renamed. Every bad id is collected before bailing, and this runs at merge
/// time because the catechism validation needs the merged questions to look at.
pub fn merge_questions_into_parts(parts: &mut [CatechismPart], by_item: HashMap<String, Vec<CatechismQuestion>>) -> Result<()> {
    // Owned keys rather than borrowed: borrowing `parts` here would hold an immutable borrow across the
    // mutation below.
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
        for i in 1..=5 {
            v.insert(format!("GEN.1.{i}"), format!("gen1v{i}"));
        }
        for i in 1..=3 {
            v.insert(format!("GEN.2.{i}"), format!("gen2v{i}"));
        }
        for i in 1..=17 {
            v.insert(format!("EXO.20.{i}"), format!("exo20v{i}"));
        }
        for i in 1..=28 {
            v.insert(format!("EXO.34.{i}"), format!("exo34v{i}"));
        }
        for i in 1..=6 {
            v.insert(format!("PSA.1.{i}"), format!("psa1v{i}"));
        }
        v.insert("PSA.2.1".to_string(), "psa2v1".to_string());
        v.insert("ROM.2.1".to_string(), "rom2v1".to_string());
        for i in 1..=21 {
            v.insert(format!("ROM.12.{i}"), format!("rom12v{i}"));
        }
        for i in 1..=14 {
            v.insert(format!("ROM.13.{i}"), format!("rom13v{i}"));
        }
        v.insert("1SA.2.1".to_string(), "1sa2v1".to_string());
        for i in 1..=25 {
            v.insert(format!("1SA.28.{i}"), format!("1sa28v{i}"));
        }
        v.insert("ISA.45.20".to_string(), "isa45v20".to_string());
        for i in 1..=25 {
            v.insert(format!("JUD.1.{i}"), format!("judv{i}"));
        }
        v
    }

    #[test]
    fn parse_yaml_questions_reads_real_repo_shape() {
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

    #[test]
    fn split_book_and_tail_handles_numbered_and_multiword_books() {
        assert_eq!(split_book_and_tail("1 Samuel 28"), Some(("1 Samuel".into(), "28".into())));
        assert_eq!(split_book_and_tail("2 Corinthians 8-9"), Some(("2 Corinthians".into(), "8-9".into())));
        assert_eq!(split_book_and_tail("Song of Solomon 2:1"), Some(("Song of Solomon".into(), "2:1".into())));
        assert_eq!(split_book_and_tail("Isaiah 45:20"), Some(("Isaiah".into(), "45:20".into())));
        assert_eq!(split_book_and_tail("Psalm 1"), Some(("Psalm".into(), "1".into())));
        assert_eq!(split_book_and_tail("no chapter here"), None);
    }

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
        let out = canonicalize_ref("Exodus 34:1, 27-28", &verses_fixture()).unwrap();
        assert_eq!(out, vec!["EXO.34.1", "EXO.34.27", "EXO.34.28"]);
    }

    #[test]
    fn canonicalize_ref_comma_compound_with_bare_verse_continuation() {
        let out = canonicalize_ref("1 Corinthians 5:11, 13", &verses_fixture());
        let err = out.unwrap_err().to_string();
        assert!(err.contains("1CO.5.11"), "{err}");
    }

    #[test]
    fn canonicalize_ref_single_chapter_book_bare_verse_and_range() {
        assert_eq!(canonicalize_ref("Jude 6", &verses_fixture()).unwrap(), vec!["JUD.1.6"]);
        assert_eq!(
            canonicalize_ref("Jude 22-25", &verses_fixture()).unwrap(),
            vec!["JUD.1.22", "JUD.1.23", "JUD.1.24", "JUD.1.25"]
        );
    }

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
                },
            ],
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
