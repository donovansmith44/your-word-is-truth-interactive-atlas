//! One TextUnit node per KJV verse, in canon reading order. A node's raw id is
//! `bible/{book}.{chapter}.{verse}` with `book` the 0-based index into `atlas_core::canon::BOOKS` --
//! the same spelling `Graph::build_indexes` derives from a `TextRef`, or the cites index misses.

use std::collections::HashMap;

use atlas_core::data::Canon;
use atlas_graph_types::id::{AnyNodeId, NodeKind};
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleTag, Corpus, LayerMap, TranslationId};

pub const BIBLE_CORPUS: &str = BibleTag::ID;
/// The one translation layer compiled here; must stay equal to
/// `atlas_core::translation::DEFAULT_TRANSLATION`.
pub const KJV_TRANSLATION: &str = "kjv";
pub const PROVENANCE: &str = "kjv";

/// One parsed KJV verse, already resolved to its canon position.
#[derive(Debug, Clone)]
pub struct KjvVerse {
    pub book_index: u8,
    pub chapter: u16,
    pub verse: u16,
    pub text: String,
}

/// `book_index` is resolved through `atlas_core::canon::resolve_alias`, never the parsed
/// `canon.books` position: the parser re-densifies to the books actually present, so the two
/// coincide only when all 66 are. A verse absent from the verse map is skipped, never fabricated.
pub fn read_kjv_ordered(kjv_json: &str) -> anyhow::Result<(Canon, Vec<KjvVerse>)> {
    let (canon, verses) = atlas_etl::kjv::parse(kjv_json)?;
    let ordered = ordered_verses_from_canon(&canon, &verses)?;
    Ok((canon, ordered))
}

/// Factored out so a caller that already holds a `Canon` and verse map builds the SAME ordered
/// walk, which is what keeps a fixture's text and its `AtlasData` from drifting apart.
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

pub fn verse_node_id(book_index: u8, chapter: u16, verse: u16) -> AnyNodeId {
    AnyNodeId { kind: NodeKind::TextUnit, raw: format!("bible/{book_index}.{chapter}.{verse}") }
}

/// The inverse of `verse_node_id`: `None` for anything not shaped like one of this adapter's ids,
/// never a panic.
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
        return None;
    }
    Some((book, chapter, verse))
}

/// The citation form a consumer reads; `verse_node_id`'s numeric form stays the internal identity.
pub fn dot_ref(book: u8, chapter: u16, verse: u16) -> String {
    let code = atlas_core::canon::BOOKS.get(book as usize).map(|b| b.code).unwrap_or("???");
    format!("{code}.{chapter}.{verse}")
}

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

pub fn verse_node(v: &KjvVerse) -> Node {
    let mut renderings: LayerMap = LayerMap::new();
    renderings.insert(TranslationId(KJV_TRANSLATION.to_string()), v.text.clone());
    Node {
        id: verse_node_id(v.book_index, v.chapter, v.verse),
        payload: NodePayload::TextUnit { corpus: BIBLE_CORPUS, renderings },
        provenance: PROVENANCE.to_string(),
    }
}
