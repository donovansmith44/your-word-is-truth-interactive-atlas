//! LEX-1 (spec §7): the lexicon adapter -- `LexiconEntry` nodes and
//! `Occurs` rows from `atlas_etl::lexicon::read_all`'s pre-parsed corpus.
//!
//! One node per Strong's entry (spec §7.2, the payload field for field, as
//! published; `id.raw` = the Strong's id, e.g. `"G3056"`). One `Occurs` row
//! per ALIGNED token (spec §7.3): `entry --occurs-in--> verse`, the locus
//! being the verse plus a one-token span on the `greek_textus_receptus` /
//! `hebrew_masoretic` layer (the CoNLL-U token id). Rows are emitted in
//! the corpus's own canonical reading order (book, chapter, verse, token),
//! which is exactly what makes a verse's `words` page list its tokens in
//! order and an entry's `occurs-in` page a concordance in canon order --
//! "by construction and nothing else".
//!
//! Skipped and COUNTED, never silently dropped: unmatched tokens (no
//! `Strong=`, ~4.7 % by upstream design -- they live in the `token`
//! inventory only), tokens whose Strong's id has no lexicon entry (none in
//! the vendored corpus; the count is printed so a future upstream gap
//! shows), and tokens on a verse the KJV canon does not carry (none
//! either -- both texts share the 31,102-verse Protestant canon, and the
//! `kjv_adapter` has already built every verse node when this pass runs).
//! Absent `ctx.lexicon` (every fixture that does not supply the corpus) is
//! a true no-op, the SAME "absent == honestly empty, not a placeholder"
//! treatment `ctx.red_letter`/`ctx.kretzmann`/`ctx.concord` already get.
//!
//! Provenance (spec §7.1, §7.3): the entry nodes are `stepbible-tbesg`
//! (Strong's PD text + TBESG glosses + MACULA domains, one registry row
//! saying what the id draws on); the NT rows `stepbible-tagnt`, the OT
//! rows `stepbible-tahot`. All three are registered in
//! `data/curated/sources.toml` (`provenance_registry_real_data.rs`
//! reconciles every id the graph carries against `sources.json`).

use atlas_etl::lexicon::{LexiconCorpus, LAYER_GREEK, LAYER_HEBREW};
use atlas_graph_types::edge::Occurs;
use atlas_graph_types::id::{AnyNodeId, LexiconEntryId, NodeKind};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{TextLocus, TextRef, TokenSpan, TranslationId, VerseRef};

use crate::kjv_adapter::verse_node_id;
use crate::pipeline::BuildCtx;

/// The entry nodes' provenance id (registered in `sources.toml`).
pub const PROVENANCE_ENTRY: &str = "stepbible-tbesg";
/// The NT (Greek) `Occurs` rows' provenance id.
pub const PROVENANCE_OCCURS_NT: &str = "stepbible-tagnt";
/// The OT (Hebrew) `Occurs` rows' provenance id.
pub const PROVENANCE_OCCURS_OT: &str = "stepbible-tahot";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LexiconAdapterStats {
    /// `LexiconEntry` nodes authored.
    pub entries: usize,
    /// `Occurs` rows authored (one per aligned token that resolved).
    pub occurs: usize,
    /// Tokens with no `Strong=` (upstream `Align=unmatched`): inventory only.
    pub tokens_unmatched: usize,
    /// Aligned tokens whose Strong's id has no lexicon entry (skipped, no edge).
    pub tokens_without_entry: usize,
    /// Aligned tokens on a verse the graph has no node for (skipped, no edge).
    pub tokens_off_canon: usize,
}

/// `n:LexiconEntry:<strong>` -- the one spelling of an entry's node id.
pub fn entry_node_id(strong: &str) -> AnyNodeId {
    AnyNodeId { kind: NodeKind::LexiconEntry, raw: strong.to_string() }
}

/// The NORMALIZE-stage pass: nodes first, then rows in corpus order.
pub fn normalize(ctx: &mut BuildCtx) -> LexiconAdapterStats {
    let mut stats = LexiconAdapterStats::default();
    let Some(corpus) = ctx.lexicon else {
        return stats;
    };
    author_entries(ctx, corpus, &mut stats);
    author_occurs(ctx, corpus, &mut stats);
    ctx.stats.lexicon = stats.clone();
    stats
}

fn author_entries(ctx: &mut BuildCtx, corpus: &LexiconCorpus, stats: &mut LexiconAdapterStats) {
    for e in &corpus.entries {
        let id = entry_node_id(&e.strong);
        let node = Node {
            id: id.clone(),
            payload: NodePayload::LexiconEntry {
                strong: e.strong.clone(),
                lang: e.lang.clone(),
                lemma: e.lemma.clone(),
                translit: e.translit.clone(),
                pos: e.pos.clone(),
                glosses: e.glosses.clone(),
                senses: e.senses.clone(),
                domains: e.domains.clone(),
                root: e.root.clone(),
            },
            provenance: ProvenanceId::from(PROVENANCE_ENTRY),
        };
        let fresh = ctx.graph.nodes.insert(id, node).is_none();
        debug_assert!(fresh, "the reader guarantees unique Strong's ids: {}", e.strong);
        stats.entries += 1;
    }
}

fn author_occurs(ctx: &mut BuildCtx, corpus: &LexiconCorpus, stats: &mut LexiconAdapterStats) {
    let nt = ProvenanceId::from(PROVENANCE_OCCURS_NT);
    let ot = ProvenanceId::from(PROVENANCE_OCCURS_OT);
    let greek = TranslationId(LAYER_GREEK.to_string());
    let hebrew = TranslationId(LAYER_HEBREW.to_string());
    for t in &corpus.tokens {
        let Some(strong) = t.strong.as_deref() else {
            stats.tokens_unmatched += 1;
            continue;
        };
        if !ctx.graph.nodes.contains_key(&entry_node_id(strong)) {
            stats.tokens_without_entry += 1;
            continue;
        }
        if !ctx.graph.nodes.contains_key(&verse_node_id(t.book.0, t.chapter, t.verse)) {
            stats.tokens_off_canon += 1;
            continue;
        }
        let (layer, provenance) = if t.layer == LAYER_GREEK { (&greek, &nt) } else { (&hebrew, &ot) };
        let span = TokenSpan::new(layer.clone(), t.ord, t.ord).expect("start == end is never inverted");
        ctx.graph.occurs.push(Occurs {
            entry: LexiconEntryId::new(strong.to_string()),
            locus: TextLocus { at: TextRef::Bible(VerseRef { book: t.book.0, chapter: t.chapter, verse: t.verse }), span: Some(span) },
            provenance: provenance.clone(),
        });
        stats.occurs += 1;
    }
}
