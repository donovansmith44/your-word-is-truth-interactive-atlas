//! The KJV source adapter: turns the raw scrollmapper `kjv.json` text into
//! one TextUnit node per verse, in canon reading order.
//!
//! Controller ruling 2 ("find its parser in server/atlas-etl -- reuse it as
//! the adapter's reader"): `read_kjv_ordered` calls `atlas_etl::kjv::parse`
//! verbatim -- the SAME raw-JSON parser `atlas-etl`'s own ETL binary calls --
//! and only adds the walk into the graph's typed node/id shapes on top. No
//! parsing logic is forked or duplicated.
//!
//! Node identity: a TextUnit's `AnyNodeId.raw` is `"bible/{book}.{chapter}.{verse}"`
//! where `book` is the 0-based index into `atlas_core::canon::BOOKS` (the
//! SAME index `atlas_core::refs::BookId` already uses everywhere else in
//! this codebase) -- an adapter choice, not a graph-types shape: `VerseRef.book`
//! is `u8` with no pinned numbering scheme, so reusing atlas-core's existing
//! canonical index keeps one book-numbering convention across the whole
//! workspace instead of inventing a second one. This format is also exactly
//! what `graph_types::graph::Graph::build_indexes`'s own (private) `text_node`
//! helper produces from a `TextRef::Bible(VerseRef)` -- required for the
//! cites BiIndex (built from `TextLocus` endpoints) to resolve to the SAME
//! node ids this adapter inserts into `graph.nodes`.

use std::collections::HashMap;

use atlas_core::data::Canon;
use atlas_graph_types::id::{AnyNodeId, NodeKind};
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{LayerMap, TranslationId};

/// The Bible corpus id (matches `graph_types::text::BibleTag::ID`) and the
/// one translation layer this batch compiles (matches
/// `atlas_core::translation::DEFAULT_TRANSLATION`).
pub const BIBLE_CORPUS: &str = "bible";
pub const KJV_TRANSLATION: &str = "kjv";

/// One parsed KJV verse, already resolved to its canon position.
#[derive(Debug, Clone)]
pub struct KjvVerse {
    pub book_index: u8,
    pub chapter: u16,
    pub verse: u16,
    pub text: String,
}

/// Reuses `atlas_etl::kjv::parse` (see module doc comment) to read
/// `kjv_json`, then re-walks the resulting `Canon` in book/chapter/verse
/// order -- which IS the Bible corpus's reading order (design doc §4: canon
/// sequence at every granularity) -- to produce one `KjvVerse` per verse,
/// already in that order. A verse present in `canon.chapters` counts but
/// absent from the parsed verse map is skipped, never fabricated (mirrors
/// `atlas_server::handlers::chapter`'s own `if let Some(text) = ...`
/// out-of-canon tolerance).
///
/// `book_index` is resolved per book via `atlas_core::canon::resolve_alias`
/// on `book.code` -- the TRUE global index into `atlas_core::canon::BOOKS`
/// -- deliberately NOT `canon.books`'s own local position (`.enumerate()`).
/// `atlas_etl::kjv::parse` returns only the books actually present in the
/// source, sorted by their true index but re-densified to that subset's own
/// 0.., so the two coincide ONLY when every one of the 66 canonical books is
/// present (true for the real committed `kjv.json`, which is why that
/// coincidence stayed hidden until a partial fixture caught it in this
/// crate's own tests). Resolving explicitly keeps `book_index` a stable,
/// source-independent identity, which every downstream node id and the
/// fidelity law's own byte comparison depend on.
pub fn read_kjv_ordered(kjv_json: &str) -> anyhow::Result<(Canon, Vec<KjvVerse>)> {
    let (canon, verses) = atlas_etl::kjv::parse(kjv_json)?;
    let ordered = ordered_verses_from_canon(&canon, &verses)?;
    Ok((canon, ordered))
}

/// The walk `read_kjv_ordered` does over an already-parsed `(Canon,
/// verses)` pair -- factored out so a caller that already HAS a `Canon` +
/// verse map (e.g. `atlas_core::data::AtlasData`'s own `demo_fixture()`,
/// used by atlas-server's test fixtures) can build the SAME ordered walk
/// directly, without a round trip through raw KJV JSON text. This is what
/// keeps a test's `AtlasData` (places/headings) and `GraphState` (text)
/// fixtures from silently drifting apart -- both derive from ONE canon +
/// verses value instead of two independently hand-authored ones.
pub fn ordered_verses_from_canon(canon: &Canon, verses: &HashMap<String, String>) -> anyhow::Result<Vec<KjvVerse>> {
    let mut ordered = Vec::with_capacity(verses.len());
    for book in &canon.books {
        let book_index = atlas_core::canon::resolve_alias(&book.code).map(|id| id.0).ok_or_else(|| {
            anyhow::anyhow!("book code '{}' does not resolve to a canonical book (unreachable: the canon was already validated)", book.code)
        })?;
        for (chapter_index, &verse_count) in book.chapters.iter().enumerate() {
            let chapter = (chapter_index + 1) as u16;
            for v in 1..=verse_count {
                let key = format!("{}.{}.{}", book.code, chapter, v);
                if let Some(text) = verses.get(&key) {
                    ordered.push(KjvVerse { book_index, chapter, verse: v, text: text.clone() });
                }
            }
        }
    }
    Ok(ordered)
}

/// The TextUnit node id for one KJV verse position -- see module doc
/// comment for the format/numbering convention.
pub fn verse_node_id(book_index: u8, chapter: u16, verse: u16) -> AnyNodeId {
    AnyNodeId { kind: NodeKind::TextUnit, raw: format!("bible/{book_index}.{chapter}.{verse}") }
}

/// The inverse of `verse_node_id` -- `None` for anything not shaped like one
/// of this adapter's own ids (a different NodeKind, or a malformed raw
/// string), never a panic.
pub fn decode_text_unit(id: &AnyNodeId) -> Option<(u8, u16, u16)> {
    if id.kind != NodeKind::TextUnit {
        return None;
    }
    let rest = id.raw.strip_prefix("bible/")?;
    let mut parts = rest.split('.');
    let book: u8 = parts.next()?.parse().ok()?;
    let chapter: u16 = parts.next()?.parse().ok()?;
    let verse: u16 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None; // trailing garbage -- not one of this adapter's ids
    }
    Some((book, chapter, verse))
}

/// The human-legible dot-ref citation for a verse position (e.g. `"JHN.3.16"`),
/// via `atlas_core::canon::BOOKS`'s own code table -- the wire/citation form;
/// `verse_node_id`'s numeric form stays the internal identity (names are
/// refs, not identity -- design doc §9b).
pub fn dot_ref(book: u8, chapter: u16, verse: u16) -> String {
    let code = atlas_core::canon::BOOKS.get(book as usize).map(|b| b.code).unwrap_or("???");
    format!("{code}.{chapter}.{verse}")
}

/// Pipeline-facing NORMALIZE entry point (`pipeline::NormalizePass`):
/// walks `ctx.kjv_canon`/`ctx.kjv_verses` into one TextUnit node per verse
/// plus the bible reading spine -- mirrors exactly what
/// `build::build_graph_from_canon_and_verses`'s own opening block did
/// before the pipeline restructuring (module doc comment above), just
/// reading/writing through `BuildCtx` instead of loose parameters.
pub fn normalize(ctx: &mut crate::pipeline::BuildCtx) -> anyhow::Result<()> {
    let ordered = ordered_verses_from_canon(ctx.kjv_canon, ctx.kjv_verses)?;
    let mut order = Vec::with_capacity(ordered.len());
    for v in &ordered {
        let node = verse_node(v);
        let id = node.id.clone();
        ctx.graph.nodes.insert(id.clone(), node);
        order.push(id);
    }
    ctx.stats.kjv_verses = ordered.len();
    ctx.graph.reading.insert(BIBLE_CORPUS, atlas_graph_types::graph::ReadingSpine { order });
    Ok(())
}

/// The TextUnit node for one parsed verse -- one node per skeleton position,
/// the canonical KJV layer as its only rendering today (sweep F1: ALL layer
/// renderings live on the node; N translations join as more layers later
/// with no shape change).
pub fn verse_node(v: &KjvVerse) -> Node {
    let mut renderings: LayerMap = LayerMap::new();
    renderings.insert(TranslationId(KJV_TRANSLATION.to_string()), v.text.clone());
    Node {
        id: verse_node_id(v.book_index, v.chapter, v.verse),
        payload: NodePayload::TextUnit { corpus: BIBLE_CORPUS, renderings },
        provenance: "kjv".to_string(),
    }
}
