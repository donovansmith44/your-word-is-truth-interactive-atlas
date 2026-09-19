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
//!
//! FINAL REVIEW items 1 and M4-1 rewrote the last two tests. (b) now
//! sweeps EVERY non-`ContainsBible` family's rows through one
//! `subject_of(row) -> AnyNodeId` and states, per family, whether the
//! placement rule promises the subject sits in the family's own section
//! (`Expect::SameAsFamily`) or deliberately elsewhere
//! (`Expect::Elsewhere` -- a curated Core row about the KJV text); three
//! families used to be checked and seventeen were not. (c) now TOUCHES
//! ROWS: it rebuilds each source row's `entry_id` the way
//! `event_world::add_justified_by` does and maps every `justified-by`
//! subject back to the row that minted it, instead of reading the
//! relation out of the edge id's own text. Each test's own doc comment
//! says what would make its assertions fail.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

use atlas_graph::sections::{
    justified_by_source_family, section_of_contains_bible, section_of_family, section_of_node,
    section_of_justified_by, Section,
};
use atlas_graph_types::canon::RowFamily;
use atlas_graph_types::chrono::ChronoTarget;
use atlas_graph_types::edge::{at, entry_id, EdgeId, Namesake, RelationId};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::text::{ConcordRef, TextLocus, TextRef, VerseRef};

fn committed_graph() -> &'static Graph {
    static CACHED: OnceLock<Graph> = OnceLock::new();
    CACHED.get_or_init(|| {
        // DB-5: the committed SECTIONS read back (sqlite::reload), indexed
        // exactly as the served path indexes -- graph.bin is gone.
        atlas_graph::sqlite::reload::committed_graph(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")).expect("the committed sections read back (run atlas-graph-compile first)").0
    })
}

/// The SAME node-id shape `graph_types::graph::Graph`'s own private
/// `text_node` builds from a locus (documented, not guessed, in
/// `event_world.rs`'s own `bible_locus_node_id` doc comment: this is the
/// stable, public format every text-locus edge endpoint already uses).
/// Mirrored here, in ONE place, because both of those functions are
/// private to their crates; every subject and every justified-by endpoint
/// below goes through these two.
fn bible_verse_node_id(v: &VerseRef) -> AnyNodeId {
    AnyNodeId {
        kind: NodeKind::TextUnit,
        raw: format!("bible/{}.{}.{}", v.book, v.chapter, v.verse),
    }
}

fn concord_ref_node_id(c: &ConcordRef) -> AnyNodeId {
    AnyNodeId {
        kind: NodeKind::TextUnit,
        raw: format!("concord/{}.{}.{}", c.part, c.article, c.paragraph),
    }
}

fn text_node_id(tl: &TextLocus) -> AnyNodeId {
    match &tl.at {
        TextRef::Bible(v) => bible_verse_node_id(v),
        TextRef::Concord(c) => concord_ref_node_id(c),
    }
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

    // Core is everything left over; Lexicon (LEX-1) is exactly the entries.
    assert_eq!(by_section.get(&Section::Lexicon), Some(&13_548), "LEX-1: the 13,548 LexiconEntry nodes");
    let core = *by_section.get(&Section::Core).unwrap();
    assert_eq!(
        core,
        total
            - *by_section.get(&Section::Kjv).unwrap()
            - *by_section.get(&Section::Concord).unwrap_or(&0)
            - *by_section.get(&Section::Kretzmann).unwrap()
            - *by_section.get(&Section::Lexicon).unwrap(),
        "Core is the rest"
    );
}

// ------------------------------------------------------------------ (b)

/// One row's SUBJECT node: the node the placement rule looks at when it
/// asks "which adapter authored this row?". Every non-`ContainsBible`
/// family goes through this one shape (`fn(&Row) -> AnyNodeId`), so the
/// sweep below is uniform and no family can be quietly left out.
struct Sweep {
    rows: usize,
    sections: BTreeMap<Section, usize>,
    missing: usize,
}

fn sweep<T>(rows: &[T], g: &Graph, subject: impl Fn(&T) -> AnyNodeId) -> Sweep {
    let mut out = Sweep { rows: rows.len(), sections: BTreeMap::new(), missing: 0 };
    for row in rows {
        match g.nodes.get(&subject(row)) {
            Some(node) => *out.sections.entry(section_of_node(node)).or_default() += 1,
            None => out.missing += 1,
        }
    }
    out
}

/// What the placement rule promises about a family's SUBJECT nodes.
#[derive(Debug)]
enum Expect {
    /// The family is authored by the adapter that also authored its
    /// subject: every subject node must sit in the family's OWN section.
    /// This is the case the placement rule is about, and the only case in
    /// which "the family's section" and "the subject's section" are the
    /// same claim.
    SameAsFamily,
    /// The family is authored by ONE adapter ABOUT another corpus's text:
    /// a curated `fulfills` row is Core's, but the verse it points at is
    /// the KJV adapter's. The subject's section is therefore NOT the
    /// family's, and saying so explicitly is the point -- an accidental
    /// `SameAsFamily` here would be a false law.
    Elsewhere(&'static [Section]),
    /// Uninhabited in the shipped graph (the encoder is proven on a
    /// hand-built row in `canon_real_data.rs`).
    NoRows,
}

/// (b) EVERY non-`ContainsBible` family's real rows, swept through ONE
/// `subject_of(row) -> AnyNodeId` helper, with the subject node's section
/// checked against what the placement rule promises for that family
/// (M4-1: three families used to be checked, seventeen were not).
///
/// What would make each assertion fail:
///
/// * `section_of_family` disagreeing with the constant table -- a family
///   moved section without this file being told.
/// * A family's subject nodes landing in a section the table does not
///   list: an adapter started authoring rows about a corpus it did not
///   before (e.g. a Concord-side `mentions`), or `section_of_node` began
///   classifying that kind differently.
/// * A `SameAsFamily` family whose subjects are NOT its own section: the
///   placement rule "a node lives in the section of the adapter that
///   authored it" has come apart from "a row lives in the section of the
///   adapter that authored IT" for that family, which is exactly the
///   assumption DB-2b's writer would inherit.
/// * A subject with NO node at all. That is pinned at exactly ONE
///   (`cross_refs`, `bible/63.1.15`); a different count means the known
///   ETL gap changed shape, or a new one opened.
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
    let lexicon_families = [RowFamily::Occurs];

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
    for f in lexicon_families {
        assert_eq!(section_of_family(f), Section::Lexicon, "{} must be Lexicon", f.name());
    }

    // Every family in the closed enum is accounted for, ContainsBible
    // (split by row, not by family) included exactly once.
    let mut accounted: Vec<RowFamily> = Vec::new();
    accounted.extend(kjv_families);
    accounted.extend(core_families);
    accounted.extend(concord_families);
    accounted.extend(kretzmann_families);
    accounted.extend(lexicon_families);
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

    // ---- the sweep: every OTHER family, one `subject_of` each ----------
    let mut swept: Vec<(RowFamily, Expect, Sweep)> = Vec::new();
    macro_rules! subject {
        ($field:ident, $family:expr, $expect:expr, $of:expr) => {
            swept.push(($family, $expect, sweep(&g.$field, g, $of)))
        };
    }

    subject!(contains_concord, RowFamily::ContainsConcord, Expect::SameAsFamily, |r| r
        .container
        .erase());
    subject!(attests, RowFamily::Attests, Expect::SameAsFamily, |r| r.event.erase());
    subject!(succession, RowFamily::Succession, Expect::SameAsFamily, |r| r.narrative.erase());
    subject!(canon_succession, RowFamily::CanonSuccession, Expect::SameAsFamily, |r| r
        .prior
        .erase());
    subject!(dated_by, RowFamily::DatedBy, Expect::SameAsFamily, |r| r.event.erase());
    subject!(located_at, RowFamily::LocatedAt, Expect::SameAsFamily, |r| r.event.erase());
    // Curated rows ABOUT the KJV text: Core's rows, the KJV adapter's
    // subjects. Named, not assumed.
    subject!(fulfills, RowFamily::Fulfills, Expect::Elsewhere(&[Section::Kjv]), |r| {
        bible_verse_node_id(&r.prophecy.from.unit)
    });
    subject!(typology, RowFamily::Typology, Expect::Elsewhere(&[Section::Kjv]), |r| {
        bible_verse_node_id(&r.type_passage.from.unit)
    });
    subject!(named_after, RowFamily::NamedAfter, Expect::SameAsFamily, |r| match &r.namesake {
        Namesake::PeopleGroup(gid) => gid.erase(),
        Namesake::Place(p) => p.erase(),
        Namesake::Polity(p) => p.erase(),
    });
    // Catechism is the one family whose subjects span TWO corpora: a
    // Small Catechism item is linked both to the Scripture it teaches
    // (6,531 KJV loci) and, through `concord-sc-overlap.toml`, to the
    // Concord passages that quote it (37). Core's rows, two adapters'
    // subjects -- which is precisely why the subject's section is not the
    // row's section for this family.
    subject!(
        catechism,
        RowFamily::Catechism,
        Expect::Elsewhere(&[Section::Kjv, Section::Concord]),
        |r| text_node_id(&r.locus)
    );
    subject!(comments_on, RowFamily::CommentsOn, Expect::SameAsFamily, |r| r.item.erase());
    subject!(spoken_by, RowFamily::SpokenBy, Expect::SameAsFamily, |r| {
        bible_verse_node_id(&r.locus.from.unit)
    });
    subject!(spoken_at, RowFamily::SpokenAt, Expect::SameAsFamily, |r| {
        bible_verse_node_id(&r.locus.from.unit)
    });
    subject!(mentions, RowFamily::Mentions, Expect::Elsewhere(&[Section::Kjv]), |r| {
        text_node_id(&r.locus)
    });
    subject!(cross_refs, RowFamily::CrossRefs, Expect::SameAsFamily, |r| text_node_id(&r.from));
    subject!(quotes, RowFamily::Quotes, Expect::NoRows, |r| text_node_id(&r.quoting));
    subject!(confesses, RowFamily::Confesses, Expect::NoRows, |r| {
        concord_ref_node_id(&r.confessing.unit)
    });
    subject!(
        corresponds_bible,
        RowFamily::CorrespondsBible,
        Expect::NoRows,
        |r| bible_verse_node_id(&r.a.unit)
    );
    subject!(temporal_adjacency, RowFamily::TemporalAdjacency, Expect::SameAsFamily, |r| r
        .earlier
        .erase());
    subject!(analogue, RowFamily::Analogue, Expect::SameAsFamily, |r| r.a.erase());
    subject!(occurs, RowFamily::Occurs, Expect::SameAsFamily, |r| r.entry.erase());

    println!("DB-2a SUBJECT SWEEP: {} families", swept.len());
    for (family, expect, s) in &swept {
        println!(
            "  {}: {} rows, subjects {:?}, missing {} ({expect:?})",
            family.name(),
            s.rows,
            s.sections,
            s.missing
        );
    }

    // Closure: the sweep covers every family except the one split by row.
    let mut covered: Vec<RowFamily> =
        swept.iter().map(|(f, _, _)| *f).chain([RowFamily::ContainsBible]).collect();
    covered.sort_by_key(|f| f.ordinal());
    assert_eq!(covered, RowFamily::ALL.to_vec(), "every family must be swept exactly once");

    for (family, expect, s) in &swept {
        let seen: Vec<Section> = s.sections.keys().copied().collect();
        match expect {
            Expect::SameAsFamily => {
                assert!(s.rows > 0, "{} claims SameAsFamily but has no rows", family.name());
                assert_eq!(
                    seen,
                    vec![section_of_family(*family)],
                    "{}: every subject node must sit in the family's own section",
                    family.name()
                );
            }
            Expect::Elsewhere(sections) => {
                assert!(s.rows > 0, "{} claims Elsewhere but has no rows", family.name());
                assert_eq!(seen, sections.to_vec(), "{} subject sections", family.name());
                assert!(
                    !seen.contains(&section_of_family(*family)),
                    "{}: Elsewhere means NOT the family's own section -- say SameAsFamily instead",
                    family.name()
                );
            }
            Expect::NoRows => {
                assert_eq!(s.rows, 0, "{} is uninhabited in the shipped graph", family.name());
                assert!(seen.is_empty());
            }
        }

        // One pre-existing data quirk, disclosed rather than papered over:
        // `atlas_etl::xrefs::filter_missing_first_verse` checks the
        // TARGET's first verse against the KJV verse table, not the FROM
        // verse's own existence (its name notwithstanding) -- so of the
        // shipped graph's 343,558 cross_refs rows, exactly one
        // (`bible/63.1.15`, a verse number 2 John does not have) has a
        // `from` with no matching TextUnit node. That is an
        // xref_adapter/ETL data-quality gap, not a section-map defect --
        // unrelated to this task and out of its scope -- so the count is
        // pinned at exactly this one rather than silently skipped.
        let expected_missing = usize::from(*family == RowFamily::CrossRefs);
        assert_eq!(
            s.missing,
            expected_missing,
            "{}: subjects with no node (cross_refs carries exactly the one known \
             xref_adapter data-quality gap, bible/63.1.15; every other family carries none) -- \
             a different count means this pre-existing issue changed shape",
            family.name()
        );
    }

    // The one missing cross_ref subject is THAT verse, not just any one.
    let missing_from: Vec<String> = g
        .cross_refs
        .iter()
        .map(|r| text_node_id(&r.from))
        .filter(|id| !g.nodes.contains_key(id))
        .map(|id| id.raw)
        .collect();
    assert_eq!(missing_from, vec!["bible/63.1.15".to_string()]);
}

// ------------------------------------------------------------------ (c)

/// (c) Every `justified-by` index entry lands in its SOURCE ROW's section.
///
/// The proof touches ROWS. `add_justified_by` mints each entry's subject
/// as `Position::Edge(entry_id(relation, &subject, &object))` over a row
/// of `dated_by`/`fulfills`/`typology`/`named_after`; this test rebuilds
/// that same edge id from each of those rows -- through the SAME public
/// `entry_id`, with the (subject, object) pair `add_justified_by` builds,
/// not by re-deriving the id's text -- and so holds a total map
/// `EdgeId -> (RowFamily, Option<container raw>)` back to the row that
/// produced it.
///
/// What would make each assertion fail:
///
/// * **Subject found in the row map.** A `justified-by` subject that is
///   not any row's edge id means an entry was synthesised from something
///   other than a row -- an orphan the artifact's `edge_index` could not
///   attribute to a section at all. (Before this rewrite the test parsed
///   the relation out of the edge id's own text, so an orphan with a
///   familiar prefix passed.)
/// * **Section agreement.** `section_of_justified_by(family, raw)` must
///   equal the section the SOURCE ROW itself maps to -- per-row for
///   `ContainsBible`, the family constant otherwise. A divergence means
///   DB-2b's writer would file a justification pointer in a different
///   section from the claim it justifies.
/// * **`justified_by_source_family` agreement.** `sections.rs` keeps a
///   second, hand-maintained relation -> family map (four arms, parsed
///   from the edge-id string). It must name the SAME family the row did.
///   This is the assertion that catches that map drifting from
///   `add_justified_by`'s own wiring (review item 10 -- until DB-2b
///   replaces it with a total map in graph-types).
/// * **Exact coverage.** The set of `justified-by` subjects must equal
///   the set of edge ids of rows WITH non-empty grounds -- no entry
///   without a grounded row, no grounded row without an entry.
#[test]
fn justified_by_entries_share_their_source_rows_section() {
    let g = committed_graph();

    // The map back to the rows. `Option<&str>` is the source row's own
    // container raw -- `None` for every family except `ContainsBible`,
    // which `section_of_justified_by` decides per row. No `ContainsBible`
    // row is a justified-by source today (`add_justified_by` wires only
    // these four relations), which is itself asserted below.
    let mut from_rows: BTreeMap<EdgeId, (RowFamily, Option<String>)> = BTreeMap::new();
    let mut grounded: BTreeSet<EdgeId> = BTreeSet::new();

    let mut record = |edge_id: EdgeId, family: RowFamily, has_grounds: bool| {
        if has_grounds {
            grounded.insert(edge_id.clone());
        }
        if let Some((seen, _)) = from_rows.get(&edge_id) {
            assert_eq!(
                *seen,
                family,
                "two families minted the same edge id {edge_id:?} -- the (relation, subject, \
                 object) key is not unique across families"
            );
        }
        from_rows.insert(edge_id, (family, None));
    };

    for row in &g.dated_by {
        let subject = at(&row.event.erase());
        let object = match row.placement.target() {
            ChronoTarget::Anchor(a) => at(&a.erase()),
            ChronoTarget::Prior(p) => at(&p.erase()),
            ChronoTarget::Era(er) => at(&er.erase()),
        };
        record(
            entry_id(RelationId::DatedBy, &subject, &object),
            RowFamily::DatedBy,
            !row.justification.grounds.is_empty(),
        );
    }
    for row in &g.fulfills {
        let subject = Position::Node(bible_verse_node_id(&row.prophecy.from.unit));
        let object = Position::Node(bible_verse_node_id(&row.fulfillment.from.unit));
        record(
            entry_id(RelationId::Fulfillment, &subject, &object),
            RowFamily::Fulfills,
            !row.justification.grounds.is_empty(),
        );
    }
    for row in &g.typology {
        let subject = Position::Node(bible_verse_node_id(&row.type_passage.from.unit));
        let object = Position::Node(bible_verse_node_id(&row.antitype_passage.from.unit));
        record(
            entry_id(RelationId::Typology, &subject, &object),
            RowFamily::Typology,
            !row.justification.grounds.is_empty(),
        );
    }
    for row in &g.named_after {
        let subject = match &row.namesake {
            Namesake::PeopleGroup(gid) => at(&gid.erase()),
            Namesake::Place(p) => at(&p.erase()),
            Namesake::Polity(p) => at(&p.erase()),
        };
        let object = at(&row.eponym.erase());
        record(
            entry_id(RelationId::NamedAfter, &subject, &object),
            RowFamily::NamedAfter,
            !row.justification.grounds.is_empty(),
        );
    }

    let jb = g
        .indexes
        .get(&RelationId::JustifiedBy)
        .expect("the shipped graph must carry a justified-by index");

    let mut by_section: BTreeMap<Section, usize> = BTreeMap::new();
    let mut subjects: BTreeSet<EdgeId> = BTreeSet::new();
    let mut count = 0usize;
    for pos in jb.fwd.keys() {
        let edge_id = match pos {
            Position::Edge(id) => id,
            Position::Node(_) => {
                panic!("a justified-by SUBJECT must be an edge position, found a node: {pos:?}")
            }
        };
        // (i) The subject IS some row's edge id -- no synthesised orphan.
        let (family, container_raw) = from_rows.get(edge_id).unwrap_or_else(|| {
            panic!(
                "justified-by subject `{edge_id:?}` is not the edge id of any dated_by / \
                 fulfills / typology / named_after row"
            )
        });
        // (ii) The entry's section IS the source row's own section.
        let section = section_of_justified_by(*family, container_raw.as_deref());
        let row_section = match *family {
            RowFamily::ContainsBible => unreachable!("no ContainsBible justified-by source today"),
            other => section_of_family(other),
        };
        assert_eq!(
            section, row_section,
            "a justified-by entry's section must equal its source ROW's own section \
             (row family {})",
            family.name()
        );
        // The second, hand-maintained spelling in `sections.rs` must name
        // the same family the row did.
        assert_eq!(
            justified_by_source_family(edge_id),
            Some(*family),
            "justified_by_source_family disagrees with the row that minted `{edge_id:?}`"
        );
        *by_section.entry(section).or_default() += 1;
        subjects.insert(edge_id.clone());
        count += 1;
    }

    println!("DB-2a JUSTIFIED_BY SECTIONS: {count} entries -> {by_section:?}");
    assert!(count > 0, "the shipped graph must carry justified-by entries (DatedBy grounds alone guarantee this)");
    assert_eq!(
        by_section.keys().collect::<Vec<_>>(),
        vec![&Section::Core],
        "today's four justified-by source families (DatedBy/Fulfills/Typology/NamedAfter) are all Core"
    );
    // Exact coverage, both directions.
    assert_eq!(
        subjects, grounded,
        "the justified-by subjects must be exactly the edge ids of the rows that carry grounds"
    );
}
