//! The KJV adapter's fidelity boundary law: every source verse becomes exactly one TextUnit
//! rendering, and the full reading-order walk reproduces the source text byte for byte. A failure
//! refuses construction rather than logging, and the expected side shares no parser with the graph.

use std::collections::BTreeSet;

use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::NodeKind;
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::text::{Rendering, TranslationId};

use crate::kjv_adapter::{self, KJV_TRANSLATION};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FidelityViolation(pub String);

impl std::fmt::Display for FidelityViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KJV adapter fidelity violation: {}", self.0)
    }
}
impl std::error::Error for FidelityViolation {}

/// A single expected verse, independently re-derived from the raw source bytes.
struct ExpectedVerse {
    book_index: u8,
    chapter: u16,
    verse: u16,
    text: String,
}

/// An independent KJV-JSON reader for this law's own "expected" side: the extraction walk and this
/// dataset's book-name quirks are written from scratch, sharing no code with the adapter's parser.
/// `serde_json` and `canon::resolve_alias` are still shared, so a bug inside either would hide.
mod independent_reader {
    use super::ExpectedVerse;

    /// Resolves this dataset's own book-name spellings from scratch: a direct canon-table match
    /// first, then a retry with a trailing " of John" stripped and a leading Roman numeral (I/II/III)
    /// rewritten to its digit.
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

    /// Verses in book/chapter/verse order, one per source JSON entry, cross-referencing nothing.
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
        // Sorted explicitly rather than assuming the source's book order is canon order, so this
        // reader stays correct against a file with reordered books.
        out.sort_by_key(|(book_index, v)| (*book_index, v.chapter, v.verse));
        Ok(out.into_iter().map(|(_, v)| v).collect())
    }
}

/// Re-derives "expected" from the source on every check rather than trusting what the builder
/// recorded: that is what makes this a proof that source and built graph agree. A real `brainfuel`
/// corpus means the same case restoration the build applied is applied to `expected` first.
pub fn check_kjv_fidelity(source_kjv_json: &str, built: &Graph, brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>) -> Result<(), FidelityViolation> {
    let mut expected = independent_reader::read(source_kjv_json).map_err(|e| FidelityViolation(format!("source failed to parse: {e}")))?;

    if let Some(corpus) = brainfuel {
        let kjv_by_dot_ref = atlas_etl::brainfuel::king_james_by_dot_ref(corpus);
        for v in &mut expected {
            let dot_ref = kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse);
            if let Some(theirs) = kjv_by_dot_ref.get(&dot_ref) {
                // The graph's own renderings went through this same restoration, so both classes
                // must be applied here too, or the law reports a false violation at every restored
                // position.
                use atlas_etl::brainfuel::RestorationOutcome;
                match atlas_etl::brainfuel::classify_and_restore(&dot_ref, &v.text, theirs) {
                    RestorationOutcome::WholeVerse(restored) | RestorationOutcome::Superscription(restored) => v.text = restored,
                    RestorationOutcome::Excluded | RestorationOutcome::MirrorCase | RestorationOutcome::Residue => {}
                }
            }
        }
    }

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
        if *rendering != Rendering::whole(v.text.clone()) {
            return Err(FidelityViolation(format!(
                "TextUnit {} rendering does not match its source text byte-for-byte",
                kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse)
            )));
        }
    }
    // Bijection is two-sided, but only the BIBLE half of `NodeKind::TextUnit` is this law's business:
    // Concord paragraphs share the kind and differ only by their raw id's corpus prefix, so a real
    // Concord corpus riding alongside is not a fidelity violation.
    for id in built.nodes.keys() {
        if id.kind == NodeKind::TextUnit && kjv_adapter::decode_text_unit(id).is_some() && !expected_ids.contains(id) {
            return Err(FidelityViolation(format!("built graph has a TextUnit node {id:?} with no corresponding source verse")));
        }
    }

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
                renderings.get(&TranslationId(KJV_TRANSLATION.to_string())).map(Rendering::text).unwrap_or_default()
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

    #[test]
    fn a_concord_text_unit_riding_alongside_is_not_a_kjv_fidelity_violation() {
        let (mut graph, ..) = build_graph_from_sources(GOOD_KJV, NO_XREFS, &crate::event_world::empty_atlas()).unwrap();
        let concord_id = atlas_graph_types::id::AnyNodeId { kind: NodeKind::TextUnit, raw: "concord/7.2.1".to_string() };
        let mut renderings = atlas_graph_types::text::LayerMap::new();
        renderings.insert(TranslationId("bente-dau".to_string()), Rendering::whole("We should fear, love, and trust in God above all things.".to_string()));
        graph.nodes.insert(
            concord_id.clone(),
            atlas_graph_types::node::Node { id: concord_id, payload: NodePayload::TextUnit { corpus: "concord", renderings }, provenance: "concord".to_string() },
        );
        assert_eq!(check_kjv_fidelity(GOOD_KJV, &graph, None), Ok(()), "a real Concord TextUnit is out of this law's own scope, never a violation");
    }

    #[test]
    fn independent_reader_resolves_the_same_quirky_book_names_the_adapter_does() {
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
                let rendering = renderings.get_mut(&TranslationId(KJV_TRANSLATION.to_string())).unwrap();
                *rendering = Rendering::whole(format!("X{}", &rendering.text()[1..]));
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
