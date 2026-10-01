mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::committed_graph;

use atlas_etl::concord::DOCUMENTS;
use atlas_graph::corpus_root::corpus_root_id;
use atlas_graph::sections::{
    justified_by_source_family, section_of_canon_succession, section_of_contains_bible, section_of_cross_ref,
    section_of_family, section_of_node, Section,
};
use atlas_graph_types::canon::RowFamily;
use atlas_graph_types::chrono::ChronoTarget;
use atlas_graph_types::edge::{at, entry_id, CanonSuccession, CrossRef, EdgeId, Namesake, RelationId};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::text::{BibleTag, ConcordRef, ConcordTag, Corpus, TextLocus, TextRef, VerseRef};

const ROOTS_PER_CORPUS: usize = 1;
const DOCUMENT_STEPS: usize = DOCUMENTS.len() - 1;

fn concord_articles() -> usize {
    let concord = common::concord_corpus();
    concord.documents.iter().map(|document| document.articles.len()).sum()
}

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

#[test]
fn every_node_maps_to_exactly_one_section_with_the_expected_per_section_counts() {
    let g = committed_graph();

    let mut by_section: BTreeMap<Section, usize> = BTreeMap::new();
    let mut kjv_text_units = 0usize;
    let mut kjv_root_containers = 0usize;
    let mut kjv_book_containers = 0usize;
    let mut kjv_chapter_containers = 0usize;
    let mut concord_text_units = 0usize;
    let mut concord_containers = 0usize;

    for (id, node) in &g.nodes {
        let section = section_of_node(node);
        *by_section.entry(section).or_default() += 1;

        if section == Section::Kjv {
            match &node.payload {
                NodePayload::TextUnit { .. } => kjv_text_units += 1,
                NodePayload::Container { .. } => {
                    if *id == corpus_root_id::<BibleTag>().erase() {
                        kjv_root_containers += 1;
                    } else if id.raw.starts_with("bible-book-") {
                        kjv_book_containers += 1;
                    } else if id.raw.starts_with("bible-chapter-") {
                        kjv_chapter_containers += 1;
                    } else {
                        panic!("a Kjv container must be the corpus root or a book or chapter container: {id:?}");
                    }
                }
                other => panic!("unexpected Kjv node payload for {id:?}: {other:?}"),
            }
        }
        if section == Section::Concord {
            match &node.payload {
                NodePayload::TextUnit { .. } => concord_text_units += 1,
                NodePayload::Container { .. } => concord_containers += 1,
                other => panic!("unexpected Concord node payload for {id:?}: {other:?}"),
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
    assert_eq!(kjv_root_containers, ROOTS_PER_CORPUS, "Kjv corpus root");
    assert_eq!(kjv_book_containers, 66, "Kjv book containers");
    assert_eq!(kjv_chapter_containers, 1_189, "Kjv chapter containers");
    assert_eq!(
        *by_section.get(&Section::Kjv).unwrap(),
        kjv_text_units + kjv_root_containers + kjv_book_containers + kjv_chapter_containers,
        "Kjv section total must be exactly its text units + its root + book + chapter containers"
    );

    assert_eq!(*by_section.get(&Section::Kretzmann).unwrap(), 50_602, "Kretzmann node count");

    assert_eq!(
        (concord_text_units, concord_containers),
        (g.reading[ConcordTag::ID].order.len(), ROOTS_PER_CORPUS + DOCUMENTS.len() + concord_articles()),
        "the Concord section holds its reading spine's text units, its root, its documents and their articles"
    );

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

struct Sweep {
    rows: usize,
    sections: BTreeMap<Section, usize>,
    missing: usize,
}

fn sections_of<T>(rows: &[T], section_of: impl Fn(&T) -> Section) -> BTreeMap<Section, usize> {
    let mut out = BTreeMap::new();
    for row in rows {
        *out.entry(section_of(row)).or_default() += 1;
    }
    out
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

#[derive(Debug)]
enum Expect {
    SameAsFamily,
    Elsewhere(&'static [Section]),
    NoRows,
}

#[test]
fn every_row_of_every_family_maps_to_a_section() {
    let g = committed_graph();

    let per_row_families = [RowFamily::ContainsBible, RowFamily::CanonSuccession, RowFamily::CrossRefs];
    let kjv_families = [RowFamily::SpokenBy, RowFamily::SpokenAt];
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
        RowFamily::ParentOf,
        RowFamily::Spouses,
        RowFamily::Participates,
        RowFamily::Authored,
        RowFamily::Shown,
        RowFamily::MapSuccession,
        RowFamily::Brethren,
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

    let mut accounted: Vec<RowFamily> = Vec::new();
    accounted.extend(kjv_families);
    accounted.extend(core_families);
    accounted.extend(concord_families);
    accounted.extend(kretzmann_families);
    accounted.extend(lexicon_families);
    accounted.extend(per_row_families);
    accounted.sort_by_key(|f| f.ordinal());
    assert_eq!(accounted, RowFamily::ALL.to_vec(), "every family must be classified exactly once");

    let contains_bible_sections = sections_of(&g.contains_bible, section_of_contains_bible);
    println!("DB-2a CONTAINS_BIBLE SECTIONS: {contains_bible_sections:?}");
    assert!(!g.contains_bible.is_empty(), "the shipped graph must carry contains_bible rows");
    assert_eq!(
        contains_bible_sections,
        BTreeMap::from([(Section::Kjv, g.contains_bible.len())]),
        "every shipped contains_bible row names the Bible root or a book/chapter container (Kjv)"
    );

    let concord_steps = DOCUMENT_STEPS + concord_articles() - DOCUMENTS.len();
    let canon_succession_sections = sections_of(&g.canon_succession, section_of_canon_succession);
    println!("DB-2a CANON_SUCCESSION SECTIONS: {canon_succession_sections:?}");
    assert_eq!(
        canon_succession_sections,
        BTreeMap::from([(Section::Kjv, g.canon_succession.len() - concord_steps), (Section::Concord, concord_steps)]),
        "the Bible's steps in Kjv; one step between each pair of Concord documents and each pair of articles within a document, in Concord"
    );
    let filed_away_from_their_prior: Vec<&CanonSuccession> = g
        .canon_succession
        .iter()
        .filter(|row| section_of_node(&g.nodes[&row.prior.erase()]) != section_of_canon_succession(row))
        .collect();
    assert_eq!(filed_away_from_their_prior, Vec::<&CanonSuccession>::new(), "a step lives in the section of the container it steps from");

    let cross_ref_sections = sections_of(&g.cross_refs, section_of_cross_ref);
    println!("CONTRACT-2 CROSS_REFS SECTIONS: {cross_ref_sections:?}");
    assert_eq!(cross_ref_sections.keys().copied().collect::<Vec<_>>(), vec![Section::Kjv, Section::Concord], "the Bible's cross references in Kjv; the Book of Concord's citations in Concord");
    let filed_away_from_the_text_that_cites: Vec<&CrossRef> = g
        .cross_refs
        .iter()
        .filter(|row| g.nodes.get(&text_node_id(&row.from)).is_some_and(|node| section_of_node(node) != section_of_cross_ref(row)))
        .collect();
    assert_eq!(filed_away_from_the_text_that_cites, Vec::<&CrossRef>::new(), "a citation lives in the section of the text that cites");

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
    subject!(dated_by, RowFamily::DatedBy, Expect::SameAsFamily, |r| r.event.erase());
    subject!(located_at, RowFamily::LocatedAt, Expect::SameAsFamily, |r| r.event.erase());
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
    subject!(parent_of, RowFamily::ParentOf, Expect::SameAsFamily, |r| r.parent.erase());
    subject!(spouses, RowFamily::Spouses, Expect::SameAsFamily, |r| r.a.erase());
    subject!(participates, RowFamily::Participates, Expect::SameAsFamily, |r| r.person.erase());
    subject!(authored, RowFamily::Authored, Expect::Elsewhere(&[Section::Kjv]), |r| r.book.erase());
    subject!(shown, RowFamily::Shown, Expect::SameAsFamily, |r| r.map.erase());
    subject!(map_succession, RowFamily::MapSuccession, Expect::SameAsFamily, |r| r.prior.erase());
    subject!(brethren, RowFamily::Brethren, Expect::SameAsFamily, |r| r.a.erase());

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

    let mut covered: Vec<RowFamily> = swept.iter().map(|(f, _, _)| *f).chain(per_row_families).collect();
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

        assert_eq!(s.missing, 0, "{}: every subject is a node", family.name());
    }

    let missing_from: Vec<String> = g
        .cross_refs
        .iter()
        .map(|r| text_node_id(&r.from))
        .filter(|id| !g.nodes.contains_key(id))
        .map(|id| id.raw)
        .collect();
    assert_eq!(missing_from, vec!["bible/63.1.15".to_string()]);
}

#[test]
fn justified_by_entries_share_their_source_rows_section() {
    let g = committed_graph();

    let mut from_rows: BTreeMap<EdgeId, RowFamily> = BTreeMap::new();
    let mut grounded: BTreeSet<EdgeId> = BTreeSet::new();

    let mut record = |edge_id: EdgeId, family: RowFamily, has_grounds: bool| {
        if has_grounds {
            grounded.insert(edge_id.clone());
        }
        if let Some(seen) = from_rows.get(&edge_id) {
            assert_eq!(
                *seen,
                family,
                "two families minted the same edge id {edge_id:?} -- the (relation, subject, \
                 object) key is not unique across families"
            );
        }
        from_rows.insert(edge_id, family);
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
        let family = from_rows.get(edge_id).unwrap_or_else(|| {
            panic!(
                "justified-by subject `{edge_id:?}` is not the edge id of any dated_by / \
                 fulfills / typology / named_after row"
            )
        });
        let section = section_of_family(*family);
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
    assert_eq!(
        subjects, grounded,
        "the justified-by subjects must be exactly the edge ids of the rows that carry grounds"
    );
}
