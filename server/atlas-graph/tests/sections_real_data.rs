//! DB-2a Task 4: the section map (spec §2.1), proven on the real
//! committed graph -- every node maps to exactly one `Section`, every row
//! of every family maps to a section (with the specific subject-node
//! agreements the placement rule promises for the families it says are
//! "authored by the corpus adapter"), and every `justified-by` index
//! entry lands in its source row's section.
//!
//! Follows `canon_real_data.rs`'s own pattern: the committed
//! `data/compiled/graph.bin` is loaded once per binary, through the SAME
//! decode + `build_indexes` + `add_justified_by` sequence
//! `GraphService::from_artifact` performs.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

use atlas_graph::sections::{
    justified_by_source_family, section_of_contains_bible, section_of_family, section_of_node,
    section_of_justified_by, Section,
};
use atlas_graph_types::canon::RowFamily;
use atlas_graph_types::edge::RelationId;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::text::{TextLocus, TextRef};

fn committed_graph() -> &'static Graph {
    static CACHED: OnceLock<Graph> = OnceLock::new();
    CACHED.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/graph.bin");
        let dump = atlas_graph::artifact::read_file(&path).expect(
            "data/compiled/graph.bin must exist -- run `cargo run -p atlas-graph --bin atlas-graph-compile` from server/ first",
        );
        let (mut graph, _stats, _ews, _chronology) =
            atlas_graph::artifact::to_service_parts(dump).expect("to_service_parts must succeed");
        graph.build_indexes();
        atlas_graph::event_world::add_justified_by(&mut graph);
        graph
    })
}

/// The SAME node-id shape `graph_types::graph::Graph`'s own private
/// `text_node` builds from a locus (documented, not guessed, in
/// `event_world.rs`'s own `bible_locus_node_id` doc comment: this is the
/// stable, public format every text-locus edge endpoint already uses).
fn text_node_id(tl: &TextLocus) -> AnyNodeId {
    let raw = match &tl.at {
        TextRef::Bible(v) => format!("bible/{}.{}.{}", v.book, v.chapter, v.verse),
        TextRef::Concord(c) => format!("concord/{}.{}.{}", c.part, c.article, c.paragraph),
    };
    AnyNodeId { kind: NodeKind::TextUnit, raw }
}

// ------------------------------------------------------------------ (a)

#[test]
fn every_node_maps_to_exactly_one_section_with_the_expected_per_section_counts() {
    let g = committed_graph();

    let mut by_section: BTreeMap<Section, usize> = BTreeMap::new();
    let mut kjv_text_units = 0usize;
    let mut kjv_book_containers = 0usize;
    let mut kjv_chapter_containers = 0usize;

    for (id, node) in &g.nodes {
        let section = section_of_node(node);
        *by_section.entry(section).or_default() += 1;

        if section == Section::Kjv {
            match &node.payload {
                NodePayload::TextUnit { .. } => kjv_text_units += 1,
                NodePayload::Container { .. } => {
                    if id.raw.starts_with("bible-book-") {
                        kjv_book_containers += 1;
                    } else if id.raw.starts_with("bible-chapter-") {
                        kjv_chapter_containers += 1;
                    } else {
                        panic!("a Kjv container must be a book or chapter container: {id:?}");
                    }
                }
                other => panic!("unexpected Kjv node payload for {id:?}: {other:?}"),
            }
        }
    }

    let total: usize = by_section.values().sum();
    println!("DB-2a SECTION MAP: {total} nodes across {} sections", by_section.len());
    for (section, n) in &by_section {
        println!("  {}: {n}", section.name());
    }

    assert_eq!(total, g.nodes.len(), "every node must map to exactly one section");

    assert_eq!(kjv_text_units, 31_102, "Kjv text units");
    assert_eq!(kjv_book_containers, 66, "Kjv book containers");
    assert_eq!(kjv_chapter_containers, 1_189, "Kjv chapter containers");
    assert_eq!(
        *by_section.get(&Section::Kjv).unwrap(),
        kjv_text_units + kjv_book_containers + kjv_chapter_containers,
        "Kjv section total must be exactly its text units + book + chapter containers"
    );

    assert_eq!(*by_section.get(&Section::Kretzmann).unwrap(), 50_602, "Kretzmann node count");

    // Concord = its own text units + its own containers -- computed
    // independently here (by corpus/prefix) and checked against the
    // section map's own count, so the two ways of knowing "is this
    // Concord's" cannot silently disagree.
    let concord_independent = g
        .nodes
        .iter()
        .filter(|(id, node)| match &node.payload {
            NodePayload::TextUnit { corpus, .. } => *corpus == "concord",
            NodePayload::Container { .. } => id.raw.starts_with("concord-"),
            _ => false,
        })
        .count();
    assert_eq!(
        *by_section.get(&Section::Concord).unwrap_or(&0),
        concord_independent,
        "Concord section total must equal its text units + containers"
    );

    // Core is everything left over; Lexicon is uninhabited today.
    assert_eq!(by_section.get(&Section::Lexicon), None, "Lexicon has no inhabitant yet");
    let core = *by_section.get(&Section::Core).unwrap();
    assert_eq!(
        core,
        total
            - *by_section.get(&Section::Kjv).unwrap()
            - *by_section.get(&Section::Concord).unwrap_or(&0)
            - *by_section.get(&Section::Kretzmann).unwrap(),
        "Core is the rest"
    );
}

// ------------------------------------------------------------------ (b)

#[test]
fn every_row_of_every_family_maps_to_a_section() {
    let g = committed_graph();

    let kjv_families =
        [RowFamily::CanonSuccession, RowFamily::CrossRefs, RowFamily::SpokenBy, RowFamily::SpokenAt];
    let core_families = [
        RowFamily::Attests,
        RowFamily::Succession,
        RowFamily::DatedBy,
        RowFamily::LocatedAt,
        RowFamily::Fulfills,
        RowFamily::Typology,
        RowFamily::NamedAfter,
        RowFamily::Catechism,
        RowFamily::Mentions,
        RowFamily::CorrespondsBible,
        RowFamily::TemporalAdjacency,
        RowFamily::Analogue,
    ];
    let concord_families = [RowFamily::ContainsConcord, RowFamily::Quotes, RowFamily::Confesses];
    let kretzmann_families = [RowFamily::CommentsOn];

    for f in kjv_families {
        assert_eq!(section_of_family(f), Section::Kjv, "{} must be Kjv", f.name());
    }
    for f in core_families {
        assert_eq!(section_of_family(f), Section::Core, "{} must be Core", f.name());
    }
    for f in concord_families {
        assert_eq!(section_of_family(f), Section::Concord, "{} must be Concord", f.name());
    }
    for f in kretzmann_families {
        assert_eq!(section_of_family(f), Section::Kretzmann, "{} must be Kretzmann", f.name());
    }

    // Every family in the closed enum is accounted for, ContainsBible
    // (split by row, not by family) included exactly once.
    let mut accounted: Vec<RowFamily> = Vec::new();
    accounted.extend(kjv_families);
    accounted.extend(core_families);
    accounted.extend(concord_families);
    accounted.extend(kretzmann_families);
    accounted.push(RowFamily::ContainsBible);
    accounted.sort_by_key(|f| f.ordinal());
    assert_eq!(accounted, RowFamily::ALL.to_vec(), "every family must be classified exactly once");

    // ContainsBible: split by row. Every row must map to a section (no
    // panic), and the shipped graph inhabits at least the Kjv bucket
    // (book/chapter containers).
    let mut contains_bible_sections: BTreeMap<Section, usize> = BTreeMap::new();
    for row in &g.contains_bible {
        *contains_bible_sections.entry(section_of_contains_bible(row)).or_default() += 1;
    }
    println!("DB-2a CONTAINS_BIBLE SECTIONS: {contains_bible_sections:?}");
    assert!(!g.contains_bible.is_empty(), "the shipped graph must carry contains_bible rows");
    assert_eq!(
        contains_bible_sections.get(&Section::Kjv).copied().unwrap_or(0),
        g.contains_bible.len(),
        "every shipped contains_bible row is a book/chapter container today (Kjv)"
    );

    // The specific per-family <-> subject-node agreements the placement
    // rule promises for "authored by the corpus adapter" families.
    //
    // One pre-existing data quirk, disclosed rather than papered over:
    // `atlas_etl::xrefs::filter_missing_first_verse` checks the TARGET's
    // first verse against the KJV verse table, not the FROM verse's own
    // existence (its name notwithstanding) -- so of the shipped graph's
    // 343,558 cross_refs rows, exactly one (`bible/63.1.15`, a verse
    // number 2 John does not have) has a `from` with no matching TextUnit
    // node. That is an xref_adapter/ETL data-quality gap, not a section-
    // map defect -- unrelated to this task and out of its scope -- so
    // this loop verifies the placement rule for every subject that DOES
    // exist and asserts the known-missing count stays exactly this one,
    // rather than silently skipping an unbounded and growing set.
    assert!(!g.cross_refs.is_empty());
    let mut cross_ref_subject_missing = 0usize;
    for row in &g.cross_refs {
        let id = text_node_id(&row.from);
        match g.nodes.get(&id) {
            Some(node) => {
                assert_eq!(section_of_node(node), Section::Kjv, "every CrossRef's subject is Kjv")
            }
            None => cross_ref_subject_missing += 1,
        }
    }
    assert_eq!(
        cross_ref_subject_missing, 1,
        "exactly the one known xref_adapter data-quality gap (bible/63.1.15) should be missing; \
         a different count means this pre-existing issue changed shape"
    );

    assert!(!g.comments_on.is_empty());
    for row in &g.comments_on {
        let id = row.item.erase();
        let node = g
            .nodes
            .get(&id)
            .unwrap_or_else(|| panic!("comments_on subject node {id:?} must exist"));
        assert_eq!(section_of_node(node), Section::Kretzmann, "every CommentsOn's subject is Kretzmann");
    }

    assert!(!g.located_at.is_empty());
    for row in &g.located_at {
        let id = row.event.erase();
        let node = g
            .nodes
            .get(&id)
            .unwrap_or_else(|| panic!("located_at subject node {id:?} must exist"));
        assert_eq!(section_of_node(node), Section::Core, "every LocatedAt's subject is Core");
    }
}

// ------------------------------------------------------------------ (c)

#[test]
fn justified_by_entries_share_their_source_rows_section() {
    let g = committed_graph();
    let jb = g
        .indexes
        .get(&RelationId::JustifiedBy)
        .expect("the shipped graph must carry a justified-by index");

    let mut by_section: BTreeMap<Section, usize> = BTreeMap::new();
    let mut count = 0usize;
    for pos in jb.fwd.keys() {
        let edge_id = match pos {
            Position::Edge(id) => id,
            Position::Node(_) => {
                panic!("a justified-by SUBJECT must be an edge position, found a node: {pos:?}")
            }
        };
        let family = justified_by_source_family(edge_id).unwrap_or_else(|| {
            panic!("justified-by source edge id `{edge_id:?}` names an unrecognised relation")
        });
        // ContainsBible never sources a justified-by entry today
        // (`add_justified_by` wires only DatedBy/Fulfills/Typology/
        // NamedAfter) -- so `None` is always the right container raw
        // here, and `section_of_justified_by` must agree with
        // `section_of_family` applied directly to that same family.
        let section = section_of_justified_by(family, None);
        assert_eq!(
            section,
            section_of_family(family),
            "a justified-by entry's section must equal its source row's own family section"
        );
        *by_section.entry(section).or_default() += 1;
        count += 1;
    }

    println!("DB-2a JUSTIFIED_BY SECTIONS: {count} entries -> {by_section:?}");
    assert!(count > 0, "the shipped graph must carry justified-by entries (DatedBy grounds alone guarantee this)");
    assert_eq!(
        by_section.keys().collect::<Vec<_>>(),
        vec![&Section::Core],
        "today's four justified-by source families (DatedBy/Fulfills/Typology/NamedAfter) are all Core"
    );
}
