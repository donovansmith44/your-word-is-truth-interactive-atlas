//! The per-surface provenance companion index. Every ROW carries a `ProvenanceId` but the served
//! index entries do not -- `EdgeMeta` has no provenance, and widening it would put a string on every
//! one of ~344k `cites` entries -- so one pre-store scan keeps what a frontier surface asks for.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::canon::RowFamily;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::sections::{section_of_cross_ref, Section};

/// One row family's name, spelled as the `Graph` field name verbatim so a reader can check a call
/// site against the struct. Only the families a wire surface cites are named here.
pub mod family {
    pub const CROSS_REFS: &str = "cross_refs";
    pub const CATECHISM: &str = "catechism";
    pub const ATTESTS: &str = "attests";
    pub const MENTIONS: &str = "mentions";
    pub const ANALOGUE: &str = "analogue";
    /// The `cross_refs` rows the Book of Concord's section holds: its citations of Scripture.
    pub const CONCORD_CITATIONS: &str = "concord_citations";
}

/// The key a row's provenance is filed under: its family's name, except that the Book of Concord's
/// citations of Scripture are kept apart from the Bible's cross references, so a verse's cross
/// references are attributed to the sources of those rows alone.
pub fn family_key(row_family: RowFamily, section: Section) -> &'static str {
    match (row_family, section) {
        (RowFamily::CrossRefs, Section::Concord) => family::CONCORD_CITATIONS,
        _ => row_family.name(),
    }
}

#[derive(Debug, Default, Clone)]
pub struct ProvenanceIndex {
    by_family: BTreeMap<&'static str, BTreeSet<String>>,
}

impl ProvenanceIndex {
    /// The ONE scan: called once, before the graph moves into the store.
    pub fn build(g: &Graph) -> ProvenanceIndex {
        let mut by_family: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
        macro_rules! sweep {
            ($name:expr, $field:ident) => {
                by_family.entry($name).or_default().extend(g.$field.iter().map(|r| r.provenance.clone()));
            };
        }
        // Every provenance-bearing family is swept, not merely the cited ones, so a new family
        // cannot silently escape attribution.
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
        for section in [Section::Kjv, Section::Concord] {
            by_family.entry(family_key(RowFamily::CrossRefs, section)).or_default();
        }
        for row in &g.cross_refs {
            by_family.entry(family_key(RowFamily::CrossRefs, section_of_cross_ref(row))).or_default().insert(row.provenance.clone());
        }
        sweep!("quotes", quotes);
        sweep!("confesses", confesses);
        sweep!("corresponds_bible", corresponds_bible);
        sweep!("temporal_adjacency", temporal_adjacency);
        sweep!(family::ANALOGUE, analogue);
        sweep!("occurs", occurs);
        sweep!("parent_of", parent_of);
        sweep!("partners", partners);
        sweep!("participates", participates);
        sweep!("authored", authored);
        sweep!("shown", shown);
        sweep!("map_succession", map_succession);

        ProvenanceIndex { by_family }
    }

    /// The same index from the database's own `SELECT DISTINCT provenance` per family: the sweep
    /// `build` does in memory, done by SQLite.
    pub fn from_families(by_family: BTreeMap<&'static str, BTreeSet<String>>) -> ProvenanceIndex {
        ProvenanceIndex { by_family }
    }

    /// Every distinct provenance id of one row family, sorted -- a SET, never a single id, because a
    /// family can be multi-sourced. An unknown name yields an empty list rather than a panic, and so
    /// does an uninhabited family, so an empty answer means "no rows" or "no such family".
    pub fn by_family(&self, name: &str) -> Vec<String> {
        self.by_family.get(name).map(|s| s.iter().cloned().collect()).unwrap_or_default()
    }

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

    #[test]
    fn the_book_of_concords_citations_are_attributed_apart_from_the_bibles_cross_references() {
        // Arrange
        let mut g = Graph::default();
        let concord = atlas_graph_types::text::TextLocus { at: atlas_graph_types::text::TextRef::Concord(atlas_graph_types::text::ConcordRef { part: 3, article: 1, paragraph: 1 }), span: None };
        let cites = |from: atlas_graph_types::text::TextLocus, provenance: &str| atlas_graph_types::edge::CrossRef {
            from,
            to: locus(),
            to_last: None,
            target_display: "MAT.8.3".to_string(),
            votes: 0,
            provenance: provenance.into(),
        };
        g.cross_refs = vec![cites(locus(), "openbible.info-cross-references"), cites(concord, "concord-citations")];
        // Act
        let ix = ProvenanceIndex::build(&g);
        // Assert
        assert_eq!(
            (ix.by_family(family::CROSS_REFS), ix.by_family(family::CONCORD_CITATIONS)),
            (vec!["openbible.info-cross-references".to_string()], vec!["concord-citations".to_string()])
        );
    }
}
