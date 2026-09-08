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
//! HONESTY OVER CONVENIENCE. Two kinds of entry live here, and they are
//! not interchangeable:
//!   * PER-ROW / PER-SUBJECT (`attests_by_event`, `analogue_by_pair`,
//!     `event_mentions_by_event`) -- the exact rows behind one rendered
//!     section of one node's frontier.
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
//!     picking one. `is_single_sourced` lets a caller ask rather than
//!     assume, and `the_per_family_provenance_map_of_the_real_artifact_
//!     is_pinned` pins both answers.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::edge::MentionedEntity;
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
    by_family: BTreeMap<&'static str, BTreeSet<String>>,
    attests_by_event: BTreeMap<String, BTreeSet<String>>,
    event_mentions_by_event: BTreeMap<String, BTreeSet<String>>,
    analogue_by_pair: BTreeMap<(String, String), String>,
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

        let mut attests_by_event: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for row in &g.attests {
            attests_by_event.entry(row.event.0.clone()).or_default().insert(row.provenance.clone());
        }

        let mut event_mentions_by_event: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for row in &g.mentions {
            if let MentionedEntity::Event(e) = &row.entity {
                event_mentions_by_event.entry(e.0.clone()).or_default().insert(row.provenance.clone());
            }
        }

        let mut analogue_by_pair: BTreeMap<(String, String), String> = BTreeMap::new();
        for row in &g.analogue {
            // `Analogue` is SYMMETRIC (`SymRelationId::Analogue`) and the
            // wire walks it from EITHER end, so both orderings are stored
            // -- the same "both ends populate the same map" shape
            // `BiIndex::build_symmetric` itself uses. Not a duplicate
            // claim: one row, two lookup keys.
            analogue_by_pair.insert((row.a.0.clone(), row.b.0.clone()), row.provenance.clone());
            analogue_by_pair.insert((row.b.0.clone(), row.a.0.clone()), row.provenance.clone());
        }

        ProvenanceIndex { by_family, attests_by_event, event_mentions_by_event, analogue_by_pair }
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

    /// True exactly when every row of `name` carries the SAME provenance
    /// id -- the precondition under which serving the family value for an
    /// individual row is a true statement about that row and not an
    /// average. Callers that serve a family value are expected to say
    /// which side of this they are on; the standing real-data law pins the
    /// answer for the two families the wire actually cites.
    pub fn is_single_sourced(&self, name: &str) -> bool {
        self.by_family.get(name).is_some_and(|s| s.len() == 1)
    }

    /// The distinct provenance of the `Attests` rows for ONE event -- the
    /// "PARALLEL ACCOUNTS" section's own sources. Empty for an event with
    /// no accounts at all (the Espousal of Mary, correctly), which renders
    /// as no affordance rather than as a blank one.
    pub fn attests_for_event(&self, event_id: &str) -> Vec<String> {
        self.attests_by_event.get(event_id).map(|s| s.iter().cloned().collect()).unwrap_or_default()
    }

    /// The distinct provenance of the `Mentions` rows naming ONE event --
    /// the "MENTIONED IN" section's own sources.
    pub fn event_mentions_for_event(&self, event_id: &str) -> Vec<String> {
        self.event_mentions_by_event.get(event_id).map(|s| s.iter().cloned().collect()).unwrap_or_default()
    }

    /// The provenance of the ONE `Analogue` row joining two events --
    /// genuinely per-row, in either direction. `None` when the pair is not
    /// joined (which a caller reaching this from a walked edge cannot
    /// normally see).
    pub fn analogue_for_pair(&self, a: &str, b: &str) -> Option<&str> {
        self.analogue_by_pair.get(&(a.to_string(), b.to_string())).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::edge::{Analogue, Attests, Mentions};
    use atlas_graph_types::id::EventId;

    fn locus() -> atlas_graph_types::text::TextLocus {
        atlas_graph_types::text::TextLocus {
            at: atlas_graph_types::text::TextRef::Bible(atlas_graph_types::text::VerseRef { book: 40, chapter: 8, verse: 3 }),
            span: None,
        }
    }

    fn range() -> atlas_graph_types::text::BibleLocusRange {
        atlas_graph_types::text::BibleLocusRange::new(
            atlas_graph_types::text::BibleLocus::whole(atlas_graph_types::text::VerseRef { book: 40, chapter: 8, verse: 1 }),
            atlas_graph_types::text::BibleLocus::whole(atlas_graph_types::text::VerseRef { book: 40, chapter: 8, verse: 4 }),
        )
        .expect("from <= to")
    }

    /// THE LEPER LESSON, as a unit test over a SYNTHETIC graph: an event
    /// whose accounts come from TWO sources must report both, never one.
    /// Collapsing this to a single value is exactly how a hand-repaired row
    /// ends up wearing an imported source's clothes.
    ///
    /// The fixture is synthetic ON PURPOSE, and the distinction matters:
    /// the REAL `attests` table is single-sourced today
    /// (`the_per_family_provenance_map_of_the_real_artifact_is_pinned`
    /// measures `{event-witnesses}`), so this is the behavior the index
    /// must have IF a second source ever lands there -- not a description
    /// of the corpus. See this module's own header for the correction.
    #[test]
    fn an_events_accounts_report_every_source_behind_them_not_just_one() {
        let mut g = Graph::default();
        g.attests.push(Attests { event: EventId::new("e1"), attestation: range(), provenance: "event-witnesses".into(), justification: Default::default() });
        g.attests.push(Attests { event: EventId::new("e1"), attestation: range(), provenance: "attestation-corrections".into(), justification: Default::default() });
        g.attests.push(Attests { event: EventId::new("e2"), attestation: range(), provenance: "event-witnesses".into(), justification: Default::default() });

        let ix = ProvenanceIndex::build(&g);
        assert_eq!(ix.attests_for_event("e1"), vec!["attestation-corrections".to_string(), "event-witnesses".to_string()]);
        assert_eq!(ix.attests_for_event("e2"), vec!["event-witnesses".to_string()]);
        // An event with no accounts renders no affordance, not a blank one.
        assert!(ix.attests_for_event("e3").is_empty());
        // ...and in THIS fixture the family view is multi-sourced, so a
        // caller may not serve it as though it were one thing.
        assert!(!ix.is_single_sourced(family::ATTESTS));
    }

    #[test]
    fn an_analogue_row_resolves_from_either_end_because_the_relation_is_symmetric() {
        let mut g = Graph::default();
        g.analogue.push(Analogue { a: EventId::new("mat_leper_healed"), b: EventId::new("rob_leper_healed"), provenance: "curated-analogues".into() });
        let ix = ProvenanceIndex::build(&g);
        assert_eq!(ix.analogue_for_pair("mat_leper_healed", "rob_leper_healed"), Some("curated-analogues"));
        assert_eq!(ix.analogue_for_pair("rob_leper_healed", "mat_leper_healed"), Some("curated-analogues"));
        assert_eq!(ix.analogue_for_pair("mat_leper_healed", "nothing"), None);
    }

    #[test]
    fn only_event_mentions_land_in_the_event_mentions_map() {
        let mut g = Graph::default();
        g.mentions.push(Mentions { locus: locus(), entity: MentionedEntity::Event(EventId::new("theo-249")), provenance: "event-mentions".into() });
        g.mentions.push(Mentions {
            locus: locus(),
            entity: MentionedEntity::Person(atlas_graph_types::id::PersonId::new("joseph_1")),
            provenance: "theographic-people".into(),
        });
        let ix = ProvenanceIndex::build(&g);
        assert_eq!(ix.event_mentions_for_event("theo-249"), vec!["event-mentions".to_string()]);
        // The family view still sees BOTH -- the per-event view is a
        // filter, never a redefinition of the family.
        assert_eq!(ix.by_family(family::MENTIONS), vec!["event-mentions".to_string(), "theographic-people".to_string()]);
    }

    #[test]
    fn a_single_sourced_family_is_reported_as_such_and_an_unknown_family_is_empty() {
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
        assert!(ix.is_single_sourced(family::CROSS_REFS));
        assert_eq!(ix.by_family(family::CROSS_REFS), vec!["openbible.info-cross-references".to_string()]);
        assert!(!ix.is_single_sourced("no-such-family"));
        assert!(ix.by_family("no-such-family").is_empty());
    }
}
