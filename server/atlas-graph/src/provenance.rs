//! Batch PROV-1: the per-surface provenance companion index.
//!
//! OWNER ORDER 1 (verbatim): "one thing we definitely need for EVERY PIECE
//! OF DATA is the source from which it came. openbible, etc."
//!
//! WHY THIS EXISTS AT ALL (the constraint, stated plainly rather than
//! worked around silently): every row of the graph carries a
//! `ProvenanceId`, but the SERVED graph does not. `Graph::build_indexes`
//! lowers rows into `BiIndex` entries carrying `(EdgeId, Position,
//! EdgeMeta)`, and `EdgeMeta` is `None | Narrative(..) | Votes(..)` --
//! provenance is not among them. `MemSnapshot`'s own `graph` field is
//! private, and `GraphQuery` (the port every handler goes through) exposes
//! `node`/`derive`/`edge_summary`/`edges`/`reading_window` and nothing
//! else. So a handler holding a snapshot can read a NODE's provenance
//! (`Node::provenance`, already on the wire as `NodeCardOut.provenance`)
//! and CANNOT read any ROW's.
//!
//! Widening `EdgeMeta`/`EdgeEntry` would be a graph-types contract change
//! -- the standing types-first veto -- and this batch carries no such
//! authorization. Widening it would also put a provenance string on every
//! one of ~344k `cites` index entries, against a committed sub-100ms
//! frontier law and a 3s artifact-load ceiling.
//!
//! So this module takes the same shape `service.rs` already uses a dozen
//! times over ("a companion index the generic port does not model,"
//! `bible_position`/`era_ids`/`narrative_legs`/`temporal_neighbors`/...):
//! ONE pre-store scan of the row tables, keeping only what a frontier
//! surface actually asks for, keyed small. Nothing here re-derives a fact
//! -- every value is a `provenance` field copied verbatim off the row that
//! declared it.
//!
//! HONESTY OVER CONVENIENCE. Two kinds of entry lived here, and they are
//! not interchangeable:
//!   * PER-ROW / PER-SUBJECT (`attests_by_event`, `analogue_by_pair`,
//!     `event_mentions_by_event`) -- the exact rows behind one rendered
//!     section of one node's frontier. DB-3 (spec 4) moved these OUT of
//!     this index: `GraphQuery::row_provenance` answers "which row made
//!     this edge" from the port, and `GraphService::{attests_provenance,
//!     event_mentions_provenance, analogue_provenance}` compose it; their
//!     unit laws (the leper lesson included) live in `service.rs` now.
//!
//!     CORRECTED (this batch's own error, kept visible rather than quietly
//!     rewritten): the first version of this paragraph asserted that
//!     `attests` is multi-source in the real corpus. IT IS NOT --
//!     `the_per_family_provenance_map_of_the_real_artifact_is_pinned`
//!     measures it as `{event-witnesses}` alone; ATTEST-1's own
//!     `attestation-corrections` rows land on `mentions` and `analogue`,
//!     not on `attests` (`event_world.rs`'s own two ATTEST-1 loops). The
//!     claim was inferred from reading the adapter instead of measured.
//!
//!     The per-subject shape still stands, on the honest ground rather
//!     than the invented one: `mentions` IS genuinely multi-source (five
//!     kinds, `attestation-corrections` among four Theographic ones), the
//!     EVENT-membership rows mix `theographic` and `curated` per row, and
//!     a family that is single-sourced TODAY is not guaranteed to stay so
//!     -- `attests_for_event` keeps telling the truth either way, where a
//!     family average would start lying the moment a second source
//!     appeared. That is the leper lesson generalized: a hand-authored row
//!     must never be able to wear an imported source's clothes.
//!   * PER-FAMILY (`by_family`) -- the complete distinct provenance SET of
//!     a whole row table, served where no smaller honest key exists
//!     (`cross_refs`, `catechism`). Measured, not assumed: `cross_refs` is
//!     single-sourced (`{openbible.info-cross-references}`) so "every row
//!     of this table says X" and "this row says X" are the same sentence
//!     there; `catechism` is NOT (`{curated-catechism,
//!     concord-sc-overlap}`), and that is exactly why this returns a SET
//!     and never a single id -- the affordance renders every entry, so a
//!     multi-sourced section names all of its sources instead of quietly
//!     picking one, and `the_per_family_provenance_map_of_the_real_artifact_
//!     is_pinned` pins both answers.
//!
//! FIX ROUND 1 (review L-4): `is_single_sourced(&self, name) -> bool` used
//! to live here, and this paragraph used to present it as the thing "a
//! caller asks instead of assuming." No caller asked -- its only callers
//! were this file's own unit tests -- and the composability bar this batch
//! quotes is "reusable, but never speculative," so it is GONE rather than
//! narrated. It was also the wrong shape for the design: the whole point of
//! `by_family` returning a SET is that a caller never has to decide whether
//! collapsing is safe, and a predicate whose only use is to authorize the
//! collapse is an invitation to the leper failure mode. The fact it stated
//! is still checked, just where facts belong -- as set EQUALITIES in
//! `the_per_family_provenance_map_of_the_real_artifact_is_pinned` and in
//! this module's own tests.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::graph::Graph;

/// One row family's name, spelled as the `Graph` field name verbatim, so
/// a reader can check a call site against the struct. Only the families a
/// wire surface actually cites are named -- `by_family` still holds every
/// family (see [`ProvenanceIndex::by_family`]).
pub mod family {
    pub const CROSS_REFS: &str = "cross_refs";
    pub const CATECHISM: &str = "catechism";
    pub const ATTESTS: &str = "attests";
    pub const MENTIONS: &str = "mentions";
    pub const ANALOGUE: &str = "analogue";
}

#[derive(Debug, Default, Clone)]
pub struct ProvenanceIndex {
    // DB-3: the per-edge lookups this index used to carry
    // (`attests_for_event`, `event_mentions_for_event`, `analogue_for_pair`)
    // moved to `GraphQuery::row_provenance` (spec 4) -- see
    // `GraphService::{attests_provenance, event_mentions_provenance,
    // analogue_provenance}`. `by_family` stays: a per-family DISTINCT
    // aggregate is a section-level scan, not an edge lookup, until DB-4's
    // `SELECT DISTINCT provenance` (plan judgment call 5).
    by_family: BTreeMap<&'static str, BTreeSet<String>>,
}

impl ProvenanceIndex {
    /// The ONE scan. Called once, before the graph moves into the store --
    /// same position and same cost class as every other companion index in
    /// `service.rs`.
    pub fn build(g: &Graph) -> ProvenanceIndex {
        let mut by_family: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
        macro_rules! sweep {
            ($name:expr, $field:ident) => {
                by_family.entry($name).or_default().extend(g.$field.iter().map(|r| r.provenance.clone()));
            };
        }
        // Every provenance-bearing family, not merely the cited ones --
        // `atlas-graph/tests/provenance_registry_real_data.rs` reconciles
        // this list against graph-types' own declaration count, so a new
        // family cannot silently escape attribution.
        by_family.entry("nodes").or_default().extend(g.nodes.values().map(|n| n.provenance.clone()));
        sweep!("contains_bible", contains_bible);
        sweep!("contains_concord", contains_concord);
        sweep!(family::ATTESTS, attests);
        sweep!("succession", succession);
        sweep!("canon_succession", canon_succession);
        sweep!("dated_by", dated_by);
        sweep!("located_at", located_at);
        sweep!("fulfills", fulfills);
        sweep!("typology", typology);
        sweep!("named_after", named_after);
        sweep!(family::CATECHISM, catechism);
        sweep!("comments_on", comments_on);
        sweep!("spoken_by", spoken_by);
        sweep!("spoken_at", spoken_at);
        sweep!(family::MENTIONS, mentions);
        sweep!(family::CROSS_REFS, cross_refs);
        sweep!("quotes", quotes);
        sweep!("confesses", confesses);
        sweep!("corresponds_bible", corresponds_bible);
        sweep!("temporal_adjacency", temporal_adjacency);
        sweep!(family::ANALOGUE, analogue);

        ProvenanceIndex { by_family }
    }

    /// DB-4c: the same index from the database's own `SELECT DISTINCT
    /// provenance` per family (`sqlite::serve::load_provenance_families`) --
    /// the sweep `build` does in memory, done by SQLite.
    pub fn from_families(by_family: BTreeMap<&'static str, BTreeSet<String>>) -> ProvenanceIndex {
        ProvenanceIndex { by_family }
    }

    /// Every distinct provenance id carried by one row family, as a sorted
    /// list. An unknown family name yields an EMPTY list, never a panic --
    /// but note that an inhabited family always has at least one id, so an
    /// empty answer at a call site means "this table has no rows" (an
    /// honest, renderable absence) or "you asked for a family that does
    /// not exist" (a typo the family:: constants above exist to prevent).
    pub fn by_family(&self, name: &str) -> Vec<String> {
        self.by_family.get(name).map(|s| s.iter().cloned().collect()).unwrap_or_default()
    }

    /// Every family name this index actually swept, sorted.
    ///
    /// FIX ROUND 1 (review L-3): this exists for ONE caller and says so --
    /// `the_runtime_index_and_the_test_sweep_name_exactly_the_same_families`
    /// in `atlas-graph/tests/provenance_registry_real_data.rs`. `build`
    /// above and that file's `provenance_by_family` are two hand-written
    /// lists of the same 22 families, and the completeness guard only
    /// checked the test's copy; a family present in the sweep but missing
    /// HERE would make `by_family` return `[]` for it, which reads
    /// everywhere else as the honest "this table has no rows." This
    /// accessor is what lets the two be asserted equal instead of merely
    /// intended equal. It is introspection over what was already built --
    /// it computes nothing and it is not a wire surface.
    pub fn families(&self) -> Vec<&'static str> {
        self.by_family.keys().copied().collect()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn locus() -> atlas_graph_types::text::TextLocus {
        atlas_graph_types::text::TextLocus {
            at: atlas_graph_types::text::TextRef::Bible(atlas_graph_types::text::VerseRef { book: 40, chapter: 8, verse: 3 }),
            span: None,
        }
    }

    #[test]
    fn a_familys_distinct_set_is_exactly_its_rows_and_an_unknown_family_is_empty() {
        let mut g = Graph::default();
        g.cross_refs.push(atlas_graph_types::edge::CrossRef {
            from: locus(),
            to: locus(),
            to_last: None,
            target_display: "MAT.8.3".to_string(),
            votes: 1,
            provenance: "openbible.info-cross-references".into(),
        });
        let ix = ProvenanceIndex::build(&g);
        assert_eq!(ix.by_family(family::CROSS_REFS), vec!["openbible.info-cross-references".to_string()]);
        assert!(ix.by_family("no-such-family").is_empty());
    }
}
