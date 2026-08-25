//! The KJV adapter's fidelity boundary law (design doc P3; types doc §5's
//! `BoundaryLaw` trait): BIJECTION (every source verse becomes exactly one
//! TextUnit rendering) and RECONSTRUCTION (the full reading-order walk
//! reproduces the source text byte-for-byte). Fail-loud: `check_kjv_fidelity`
//! is called automatically inside `GraphService::from_sources`/`build` (see
//! that module) and a failure there refuses construction entirely, not
//! merely logs.
//!
//! FIX ROUND 1, I2 (de-circularization): the "expected" side used to come
//! from `kjv_adapter::read_kjv_ordered`, which itself calls
//! `atlas_etl::kjv::parse` -- THE SAME function `build_graph_from_sources`
//! calls to build the graph being checked. Both sides therefore flowed
//! through identical parsing/book-resolution code on identical bytes, so a
//! bug INSIDE that shared code (e.g. `atlas_core::canon::resolve_alias`
//! mis-resolving two distinct raw names to the same index) would corrupt
//! both sides identically and the law would report `Ok(())` regardless.
//! `independent_reader` below re-derives "expected" through CODE THAT NEVER
//! CALLS `atlas_etl::kjv::parse` (or any of its types/helpers) -- see that
//! module's own doc comment for exactly what is and isn't shared with the
//! adapter path, and why.
//!
//! BATCH KJV-CASE: `check_kjv_fidelity` gained a third parameter,
//! `brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>`. The
//! adapter path (`atlas_graph::build::build_graph_from_sources_with_eras_
//! and_brainfuel`) now case-restores the KJV text it builds TextUnit nodes
//! from (`atlas_etl::brainfuel::restore_kjv_case`) whenever a real
//! `brainfuel` corpus is supplied -- so "the graph's own text" and "the
//! raw bytes of `source_kjv_json`" honestly diverge by exactly the
//! restored positions' casing. Passing the SAME corpus through here lets
//! this law apply the IDENTICAL, separately law-tested
//! `atlas_etl::brainfuel::restore_verse_case` transform to its own
//! independently-derived `expected` text before comparing -- keeping I2's
//! independence boundary intact (this still shares NO parsing/book-
//! resolution code with the adapter path; `restore_verse_case` is a pure,
//! generic byte transform, not a parser, and is proven correct on its own
//! terms by its own unit + real-data tests) while keeping the law
//! MEANINGFUL: it now proves the graph matches "kjv.json, case-restored,"
//! which is what "the declared source" honestly means post-restoration,
//! rather than falsely reporting drift at every one of the restored
//! positions. `None` (no real brainfuel) leaves `expected` untouched,
//! exactly the pre-batch behavior -- every existing caller that never
//! threads real brainfuel data through is unaffected.
//!
//! BATCH KJV-CASE-2 (batch-kjv-case2-brief.md): `atlas_etl::brainfuel::
//! restore_kjv_case` (the adapter path's own case-restoration call, in
//! `build.rs`) now also restores superscription-tail-aligned positions,
//! via `atlas_etl::brainfuel::classify_and_restore` -- the ONE mechanism
//! shared by both real call sites (that module's own doc comment). This
//! law's own loop below now calls the SAME function, so its "expected"
//! side stays in lockstep with whatever the adapter path actually did at
//! every position, superscription-tail class included.

use std::collections::BTreeSet;

use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::NodeKind;
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::text::TranslationId;

use crate::kjv_adapter::{self, KJV_TRANSLATION};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FidelityViolation(pub String);

impl std::fmt::Display for FidelityViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KJV adapter fidelity violation: {}", self.0)
    }
}
impl std::error::Error for FidelityViolation {}

/// A single expected verse, as independently re-derived from raw source
/// bytes -- deliberately the SAME shape as `kjv_adapter::KjvVerse` in
/// spirit, but constructed by wholly separate code (see
/// `independent_reader` below).
struct ExpectedVerse {
    book_index: u8,
    chapter: u16,
    verse: u16,
    text: String,
}

/// FIX ROUND 1, I2: an independent, minimal KJV-JSON reader for the
/// fidelity law's OWN "expected" side -- never calls
/// `atlas_etl::kjv::parse`, never imports its `RawKjv`/`RawBook`/
/// `RawChapter`/`RawVerse` types, never calls its `normalize_book_name`.
///
/// What IS still shared with the adapter path, disclosed exactly (per the
/// fix-round instruction to state this precisely):
/// - `serde_json` itself (a mature, independently-tested third-party
///   library doing raw byte-level JSON decoding -- not app logic; using a
///   different JSON library would not make this check more independent
///   in any way that matters, only more code to maintain).
/// - `atlas_core::canon::resolve_alias` (the canonical book-name-or-code
///   lookup table) -- a foundational, general-purpose data table this
///   whole codebase shares for "what book is this," not parsing logic
///   specific to the raw KJV.json shape or to `atlas_etl::kjv`'s own
///   adapter. A bug INSIDE `resolve_alias` itself (e.g. two distinct
///   names mapping to the same index) would still not be caught by this
///   law -- that residual gap is real and disclosed, not hidden -- but
///   `resolve_alias` is exercised by dozens of other call sites across
///   this workspace (ETL validation, every ref-parsing endpoint), so a
///   bug there is a much shallower, much more heavily-tested surface than
///   the KJV-JSON-specific canon-walk this law exists to check.
///
/// What is INDEPENDENT (the actual point of this module): the JSON
/// EXTRACTION walk (raw `serde_json::Value` field access -- `.get("books")`,
/// `.as_array()`, `.as_str()`, `.as_u64()` -- never `#[derive(Deserialize)]`
/// onto a shape mirroring `atlas_etl::kjv`'s own structs) and the
/// dataset's own book-name-quirk normalization (Roman-numeral prefixes,
/// the "of John" suffix), which is hand-written here from scratch --
/// same PROBLEM as `atlas_etl::kjv::normalize_book_name` solves, wholly
/// separate CODE, so a bug in one is not mechanically replicated in the
/// other. This module has NO dependency on `atlas_etl::kjv` at all (not
/// even via `use` -- checked: the fidelity module now has no such
/// import).
mod independent_reader {
    use super::ExpectedVerse;

    /// Resolves this dataset's own book-name spelling to a canonical
    /// index, written from scratch (NOT calling
    /// `atlas_etl::kjv::normalize_book_name`): try a direct canon-table
    /// match first (covers the overwhelming majority of names verbatim,
    /// e.g. "Genesis", "Song of Solomon"); failing that, strip a trailing
    /// " of John" and/or a leading Roman-numeral word (I/II/III) rewritten
    /// to its Arabic digit, and retry.
    fn resolve_book_index(raw_name: &str) -> Option<u8> {
        let name = raw_name.trim();
        if let Some(id) = atlas_core::canon::resolve_alias(name) {
            return Some(id.0);
        }

        let without_suffix = name.strip_suffix(" of John").unwrap_or(name);
        if without_suffix != name {
            if let Some(id) = atlas_core::canon::resolve_alias(without_suffix) {
                return Some(id.0);
            }
        }

        if let Some((first, rest)) = without_suffix.split_once(' ') {
            let arabic = match first {
                "I" => Some('1'),
                "II" => Some('2'),
                "III" => Some('3'),
                _ => None,
            };
            if let Some(digit) = arabic {
                let candidate = format!("{digit} {rest}");
                if let Some(id) = atlas_core::canon::resolve_alias(&candidate) {
                    return Some(id.0);
                }
            }
        }

        None
    }

    /// Walks the raw JSON structure directly via `serde_json::Value` --
    /// no typed `#[derive(Deserialize)]` shape shared with
    /// `atlas_etl::kjv`'s own `RawKjv`/`RawBook`/`RawChapter`/`RawVerse`.
    /// Returns verses in book/chapter/verse (canon reading) order, one
    /// per source JSON verse entry, with no cross-referencing against any
    /// other data (a pure, standalone re-read of the raw bytes).
    pub fn read(source_kjv_json: &str) -> Result<Vec<ExpectedVerse>, String> {
        let root: serde_json::Value = serde_json::from_str(source_kjv_json).map_err(|e| format!("independent reader: invalid JSON: {e}"))?;
        let books = root.get("books").and_then(|b| b.as_array()).ok_or("independent reader: no 'books' array at the JSON root")?;

        let mut out: Vec<(u8, ExpectedVerse)> = Vec::new();
        for book in books {
            let name = book.get("name").and_then(|n| n.as_str()).ok_or("independent reader: a book entry has no 'name' string")?;
            let book_index =
                resolve_book_index(name).ok_or_else(|| format!("independent reader: book name '{name}' does not resolve to any canonical book"))?;
            let chapters = book.get("chapters").and_then(|c| c.as_array()).ok_or_else(|| format!("independent reader: book '{name}' has no 'chapters' array"))?;
            for chapter in chapters {
                let chapter_num = chapter
                    .get("chapter")
                    .and_then(|c| c.as_u64())
                    .ok_or_else(|| format!("independent reader: a chapter of '{name}' has no numeric 'chapter'"))? as u16;
                let verses = chapter.get("verses").and_then(|v| v.as_array()).ok_or_else(|| format!("independent reader: {name} {chapter_num} has no 'verses' array"))?;
                for verse in verses {
                    let verse_num = verse
                        .get("verse")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| format!("independent reader: a verse of {name} {chapter_num} has no numeric 'verse'"))? as u16;
                    let text = verse
                        .get("text")
                        .and_then(|t| t.as_str())
                        .ok_or_else(|| format!("independent reader: {name} {chapter_num}:{verse_num} has no 'text' string"))?;
                    out.push((book_index, ExpectedVerse { book_index, chapter: chapter_num, verse: verse_num, text: text.to_string() }));
                }
            }
        }
        // Canon (book/chapter/verse) order -- the JSON's own book order
        // already happens to match canon order in the real committed
        // source (kjv_adapter's own doc comment notes this), but this
        // reader sorts explicitly rather than assuming it, so it stays
        // correct even against a source file with reordered books.
        out.sort_by_key(|(book_index, v)| (*book_index, v.chapter, v.verse));
        Ok(out.into_iter().map(|(_, v)| v).collect())
    }
}

/// Re-derives "expected" via `independent_reader::read` (module doc
/// comment above -- I2: no shared path with the adapter's own
/// `atlas_etl::kjv::parse`), then checks `built` against it. Re-deriving
/// from source on every check (rather than trusting whatever the builder
/// happened to record) is the point: this is the proof that source and
/// built graph agree, not a self-consistency check of the builder alone.
///
/// `brainfuel`: Batch KJV-CASE (module doc comment above) -- when real,
/// applies the SAME case-restoration transform the adapter path applies,
/// to this law's OWN independently-derived `expected` text, before the
/// comparison below runs.
pub fn check_kjv_fidelity(source_kjv_json: &str, built: &Graph, brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>) -> Result<(), FidelityViolation> {
    let mut expected = independent_reader::read(source_kjv_json).map_err(|e| FidelityViolation(format!("source failed to parse: {e}")))?;

    if let Some(corpus) = brainfuel {
        let kjv_by_dot_ref = atlas_etl::brainfuel::king_james_by_dot_ref(corpus);
        for v in &mut expected {
            let dot_ref = kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse);
            if let Some(theirs) = kjv_by_dot_ref.get(&dot_ref) {
                // Batch KJV-CASE-2: `classify_and_restore` is the SAME
                // shared mechanism `atlas_etl::brainfuel::restore_kjv_case`
                // uses to build the graph's own TextUnit renderings (see
                // that function's own doc comment) -- both the whole-verse
                // class (pass 1) AND the superscription-tail class (batch
                // KJV-CASE-2) must apply identically here, or this law
                // would report a false violation at every superscription-
                // restored position (the graph's own text would carry the
                // restored tail while this law's own "expected" stayed
                // unrestored). Excluded/mirror-case/residue positions
                // leave `v.text` untouched, exactly matching what the
                // adapter path also left untouched at those positions.
                use atlas_etl::brainfuel::RestorationOutcome;
                match atlas_etl::brainfuel::classify_and_restore(&dot_ref, &v.text, theirs) {
                    RestorationOutcome::WholeVerse(restored) | RestorationOutcome::Superscription(restored) => v.text = restored,
                    RestorationOutcome::Excluded | RestorationOutcome::MirrorCase | RestorationOutcome::Residue => {}
                }
            }
        }
    }

    // BIJECTION: every source verse <-> exactly one TextUnit node, and its
    // rendering matches the source text verbatim.
    let mut expected_ids = BTreeSet::new();
    for v in &expected {
        let id = kjv_adapter::verse_node_id(v.book_index, v.chapter, v.verse);
        expected_ids.insert(id.clone());
        let node = built
            .nodes
            .get(&id)
            .ok_or_else(|| FidelityViolation(format!("source verse {} has no TextUnit node in the built graph", kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse))))?;
        let rendering = match &node.payload {
            NodePayload::TextUnit { renderings, .. } => renderings.get(&TranslationId(KJV_TRANSLATION.to_string())),
            _ => None,
        }
        .ok_or_else(|| FidelityViolation(format!("TextUnit {} carries no KJV rendering", kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse))))?;
        if rendering != &v.text {
            return Err(FidelityViolation(format!(
                "TextUnit {} rendering does not match its source text byte-for-byte",
                kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse)
            )));
        }
    }
    // ...and no EXTRA *Bible* TextUnit node claims to be a KJV verse the
    // source doesn't have (bijection is two-sided). CORP-2a: `NodeKind::
    // TextUnit` is now shared by TWO corpora (`kjv_adapter`'s own Bible
    // verses AND `concord_adapter`'s own Concord paragraphs, both
    // `AnyNodeId { kind: TextUnit, .. }`, differing only in their `raw`
    // string's own corpus prefix) -- this law is titled and scoped "KJV
    // adapter fidelity" (module doc comment), so it must only ever police
    // the BIBLE half of that shared kind, via `kjv_adapter::decode_text_unit`'s
    // own "bible/" prefix check (the SAME test that already tells a real
    // Bible id apart from a malformed one, reused here rather than a
    // second hand-written prefix check). A real, non-empty `contains_concord`
    // corpus riding alongside is not a fidelity violation of THIS law --
    // it has no boundary claim over Concord at all.
    for id in built.nodes.keys() {
        if id.kind == NodeKind::TextUnit && kjv_adapter::decode_text_unit(id).is_some() && !expected_ids.contains(id) {
            return Err(FidelityViolation(format!("built graph has a TextUnit node {id:?} with no corresponding source verse")));
        }
    }

    // RECONSTRUCTION: the full reading-order walk, in order, reproduces the
    // source text byte-for-byte.
    let spine = built
        .reading
        .get(kjv_adapter::BIBLE_CORPUS)
        .ok_or_else(|| FidelityViolation("built graph has no bible reading spine".to_string()))?;
    if spine.order.len() != expected.len() {
        return Err(FidelityViolation(format!(
            "reading-order spine has {} units but the source has {} verses",
            spine.order.len(),
            expected.len()
        )));
    }
    for (id, v) in spine.order.iter().zip(expected.iter()) {
        let expected_id = kjv_adapter::verse_node_id(v.book_index, v.chapter, v.verse);
        if *id != expected_id {
            return Err(FidelityViolation(format!(
                "reading order diverges from source order at {} (spine has {:?})",
                kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse),
                id
            )));
        }
    }
    let reconstructed: Vec<&str> = spine
        .order
        .iter()
        .map(|id| match &built.nodes[id].payload {
            NodePayload::TextUnit { renderings, .. } => {
                renderings.get(&TranslationId(KJV_TRANSLATION.to_string())).map(String::as_str).unwrap_or_default()
            }
            _ => "",
        })
        .collect();
    let source_texts: Vec<&str> = expected.iter().map(|v| v.text.as_str()).collect();
    if reconstructed != source_texts {
        return Err(FidelityViolation("the reading-order walk does not reproduce the source text byte-for-byte".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::build_graph_from_sources;

    const GOOD_KJV: &str = r#"{
      "translation": "KJV",
      "books": [
        { "name": "Genesis", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "In the beginning God created the heaven and the earth." },
          { "verse": 2, "text": "And the earth was without form, and void." }
        ] } ] },
        { "name": "I Samuel", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "Now there was a certain man of Ramathaimzophim." }
        ] } ] },
        { "name": "Revelation of John", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "The Revelation of Jesus Christ." }
        ] } ] }
      ]
    }"#;
    const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

    #[test]
    fn green_on_a_clean_fixture() {
        let (graph, ..) = build_graph_from_sources(GOOD_KJV, NO_XREFS, &crate::event_world::empty_atlas()).unwrap();
        assert_eq!(check_kjv_fidelity(GOOD_KJV, &graph, None), Ok(()));
    }

    /// CORP-2a: a real, live-caught regression -- before this batch's own
    /// fix, this law scanned EVERY `NodeKind::TextUnit` node for Bible
    /// bijection membership, so a real Concord TextUnit riding alongside
    /// (this test inserts one directly, the same shape `concord_adapter::
    /// normalize` builds) failed as "no corresponding source verse," even
    /// though this law's own scope is titled "KJV adapter fidelity" and
    /// was never meant to police a different corpus at all.
    #[test]
    fn a_concord_text_unit_riding_alongside_is_not_a_kjv_fidelity_violation() {
        let (mut graph, ..) = build_graph_from_sources(GOOD_KJV, NO_XREFS, &crate::event_world::empty_atlas()).unwrap();
        let concord_id = atlas_graph_types::id::AnyNodeId { kind: NodeKind::TextUnit, raw: "concord/7.2.1".to_string() };
        let mut renderings = atlas_graph_types::text::LayerMap::new();
        renderings.insert(TranslationId("bente-dau".to_string()), "We should fear, love, and trust in God above all things.".to_string());
        graph.nodes.insert(
            concord_id.clone(),
            atlas_graph_types::node::Node { id: concord_id, payload: NodePayload::TextUnit { corpus: "concord", renderings }, provenance: "concord".to_string() },
        );
        assert_eq!(check_kjv_fidelity(GOOD_KJV, &graph, None), Ok(()), "a real Concord TextUnit is out of this law's own scope, never a violation");
    }

    #[test]
    fn independent_reader_resolves_the_same_quirky_book_names_the_adapter_does() {
        // Pins the SAME three quirky-name cases atlas_etl::kjv's own test
        // covers (kjv.rs::tests::normalizes_roman_numerals_and_of_john_suffix)
        // -- proving this independent reader solves the identical problem,
        // via separate code, not that it happens to dodge the hard cases.
        let expected = independent_reader::read(GOOD_KJV).unwrap();
        let books: std::collections::BTreeSet<u8> = expected.iter().map(|v| v.book_index).collect();
        let gen_idx = atlas_core::canon::resolve_alias("GEN").unwrap().0;
        let sa1_idx = atlas_core::canon::resolve_alias("1SA").unwrap().0;
        let rev_idx = atlas_core::canon::resolve_alias("REV").unwrap().0;
        assert_eq!(books, [gen_idx, sa1_idx, rev_idx].into_iter().collect(), "Genesis / I Samuel -> 1SA / Revelation of John -> REV must all resolve");
    }

    #[test]
    fn red_when_a_verse_is_dropped_from_the_built_graph() {
        let (mut graph, ..) = build_graph_from_sources(GOOD_KJV, NO_XREFS, &crate::event_world::empty_atlas()).unwrap();
        let dropped_id = kjv_adapter::verse_node_id(0, 1, 2);
        graph.nodes.remove(&dropped_id);
        if let Some(spine) = graph.reading.get_mut(kjv_adapter::BIBLE_CORPUS) {
            spine.order.retain(|id| *id != dropped_id);
        }
        let result = check_kjv_fidelity(GOOD_KJV, &graph, None);
        assert!(result.is_err(), "a graph missing a source verse must fail the bijection check");
    }

    #[test]
    fn red_when_a_rendering_byte_is_mutated() {
        let (mut graph, ..) = build_graph_from_sources(GOOD_KJV, NO_XREFS, &crate::event_world::empty_atlas()).unwrap();
        let id = kjv_adapter::verse_node_id(0, 1, 1);
        if let Some(node) = graph.nodes.get_mut(&id) {
            if let NodePayload::TextUnit { renderings, .. } = &mut node.payload {
                let text = renderings.get_mut(&TranslationId(KJV_TRANSLATION.to_string())).unwrap();
                text.replace_range(0..1, "X"); // "In the..." -> "Xn the..."
            }
        }
        let result = check_kjv_fidelity(GOOD_KJV, &graph, None);
        assert!(result.is_err(), "a mutated byte must fail the bijection/reconstruction check");
    }

    #[test]
    fn green_on_the_real_committed_kjv_source() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist (committed real data)");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt"))
            .expect("data/raw/xrefs/cross_references.txt must exist (committed real data)");
        let (graph, stats, ..) = build_graph_from_sources(&kjv_json, &xrefs_tsv, &crate::event_world::empty_atlas()).expect("the real KJV source must parse");
        assert_eq!(stats.kjv_verses, 31_102, "the real KJV text is 31,102 verses");
        assert_eq!(check_kjv_fidelity(&kjv_json, &graph, None), Ok(()), "the real KJV source must satisfy its own bijection + reconstruction law");
    }

    // -------------------------------------------------------------------
    // Batch KJV-CASE: the boundary law survives (and stays MEANINGFUL)
    // once real case restoration is in play.
    // -------------------------------------------------------------------

    fn real_brainfuel() -> atlas_etl::brainfuel::BrainFuelCorpus {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first")
    }

    #[test]
    fn green_over_real_data_with_case_restoration_threaded_through_both_sides() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist (committed real data)");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt"))
            .expect("data/raw/xrefs/cross_references.txt must exist (committed real data)");
        let brainfuel = real_brainfuel();
        let (graph, ..) = crate::build::build_graph_from_sources_with_eras_and_brainfuel(&kjv_json, &xrefs_tsv, &crate::event_world::empty_atlas(), &[], Some(&brainfuel))
            .expect("the real KJV + brainfuel sources must build");
        assert_eq!(
            check_kjv_fidelity(&kjv_json, &graph, Some(&brainfuel)),
            Ok(()),
            "a case-restored graph must satisfy the boundary law when the SAME brainfuel corpus is threaded through the check -- \
             the law now proves 'matches kjv.json, case-restored', which is what the declared source honestly means post-restoration"
        );
    }

    #[test]
    fn red_when_the_graph_is_case_restored_but_the_check_is_not_told() {
        // Proves the check above is not vacuously true: a graph built WITH
        // restoration, checked WITHOUT it, must genuinely disagree at the
        // restored positions (the checker's own "expected" stays
        // unrestored while the graph's own text does not) -- the law still
        // has teeth, it isn't just "always green once you pass a corpus."
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist (committed real data)");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt"))
            .expect("data/raw/xrefs/cross_references.txt must exist (committed real data)");
        let brainfuel = real_brainfuel();
        let (graph, ..) = crate::build::build_graph_from_sources_with_eras_and_brainfuel(&kjv_json, &xrefs_tsv, &crate::event_world::empty_atlas(), &[], Some(&brainfuel))
            .expect("the real KJV + brainfuel sources must build");
        let result = check_kjv_fidelity(&kjv_json, &graph, None);
        assert!(result.is_err(), "a case-restored graph checked against an UNRESTORED expectation must fail -- the law must not be trivially green");
    }
}
