use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

use atlas_graph_types::canon::ids::{parse_position, position_str};
use atlas_graph_types::canon::{encode_row_in_family, Canon, RowFamily};
use atlas_graph_types::edge::{Confesses, Corresponds, Justification, Quotes};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::Position;
use atlas_graph_types::node::Node;
use atlas_graph_types::text::{
    BibleLocusRange, BibleTag, ConcordRef, Locus, LocusRange, TextLocus, TextRef, VerseRef,
};

fn vr(book: u8, chapter: u16, verse: u16) -> VerseRef {
    VerseRef { book, chapter, verse }
}

fn blr(from: (u8, u16, u16), to: (u8, u16, u16)) -> BibleLocusRange {
    LocusRange::new(
        Locus::whole(vr(from.0, from.1, from.2)),
        Locus::whole(vr(to.0, to.1, to.2)),
    )
    .expect("a test range must be ordered")
}

const MAPS: usize = 10;
const MAP_STEPS: usize = MAPS - 1;
const SHOWN_ROWS: usize = 3_188;

fn committed_graph() -> &'static Graph {
    static CACHED: OnceLock<Graph> = OnceLock::new();
    CACHED.get_or_init(|| {
        atlas_graph::sqlite::reload::committed_graph(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")).expect("the committed sections read back (run atlas-graph-compile first)").0
    })
}

#[test]
fn every_node_round_trips_and_re_encodes_identically() {
    let graph = committed_graph();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut count = 0usize;

    for (id, n) in &graph.nodes {
        let bytes = n.encode();
        let back = Node::decode(&bytes)
            .unwrap_or_else(|e| panic!("node {id:?} failed to decode its own bytes: {e}"));
        assert_eq!(back.encode(), bytes, "node {id:?} is not a byte fixed point");
        assert_eq!(&back.id, id, "node {id:?} lost its id");
        assert_eq!(back.provenance, n.provenance, "node {id:?} lost its provenance");
        assert_eq!(
            format!("{:?}", back.payload),
            format!("{:?}", n.payload),
            "node {id:?} lost payload content"
        );
        *by_kind.entry(format!("{:?}", id.kind)).or_default() += 1;
        count += 1;
    }

    println!("DB-2a NODE CANON: {count} nodes round-tripped");
    for (kind, n) in &by_kind {
        println!("  node {kind}: {n}");
    }
    assert_eq!(count, 106_742 + MAPS, "the committed graph carries exactly 106,742 nodes (93,194 + 13,548 LexiconEntry at LEX-1) plus one Map per era");
}

fn round_trip_family<T: Canon>(rows: &[T], family: RowFamily) -> usize {
    for (i, row) in rows.iter().enumerate() {
        let value = row.to_value();
        let bytes = row.encode();
        let back = T::decode(&bytes).unwrap_or_else(|e| {
            panic!("{} row {i} failed to decode its own bytes: {e}", family.name())
        });
        assert_eq!(back.to_value(), value, "{} row {i} lost content", family.name());
        assert_eq!(back.encode(), bytes, "{} row {i} is not a byte fixed point", family.name());
        assert_eq!(
            encode_row_in_family(family, back.to_value()),
            encode_row_in_family(family, value),
            "{} row {i} is not a byte fixed point inside its family",
            family.name()
        );
    }
    rows.len()
}

#[test]
fn every_row_of_every_family_round_trips() {
    let Graph {
        nodes: _,
        contains_bible,
        contains_concord,
        attests,
        succession,
        canon_succession,
        dated_by,
        located_at,
        fulfills,
        typology,
        named_after,
        catechism,
        comments_on,
        spoken_by,
        spoken_at,
        mentions,
        cross_refs,
        quotes,
        confesses,
        corresponds_bible,
        temporal_adjacency,
        analogue,
        occurs,
        parent_of,
        partners,
        participates,
        authored,
        shown,
        map_succession,
        reading: _,
        extra_tables: _,
        indexes: _,
        symmetric_indexes: _,
        pid_index: _,
        edge_rows: _,
        spine_index: _,
    } = committed_graph();
    let mut counts: Vec<(RowFamily, usize)> = Vec::new();

    macro_rules! fam {
        ($field:ident, $family:expr) => {
            counts.push(($family, round_trip_family($field, $family)))
        };
    }

    fam!(contains_bible, RowFamily::ContainsBible);
    fam!(contains_concord, RowFamily::ContainsConcord);
    fam!(attests, RowFamily::Attests);
    fam!(succession, RowFamily::Succession);
    fam!(canon_succession, RowFamily::CanonSuccession);
    fam!(dated_by, RowFamily::DatedBy);
    fam!(located_at, RowFamily::LocatedAt);
    fam!(fulfills, RowFamily::Fulfills);
    fam!(typology, RowFamily::Typology);
    fam!(named_after, RowFamily::NamedAfter);
    fam!(catechism, RowFamily::Catechism);
    fam!(comments_on, RowFamily::CommentsOn);
    fam!(spoken_by, RowFamily::SpokenBy);
    fam!(spoken_at, RowFamily::SpokenAt);
    fam!(mentions, RowFamily::Mentions);
    fam!(cross_refs, RowFamily::CrossRefs);
    fam!(quotes, RowFamily::Quotes);
    fam!(confesses, RowFamily::Confesses);
    fam!(corresponds_bible, RowFamily::CorrespondsBible);
    fam!(temporal_adjacency, RowFamily::TemporalAdjacency);
    fam!(analogue, RowFamily::Analogue);
    fam!(occurs, RowFamily::Occurs);
    fam!(parent_of, RowFamily::ParentOf);
    fam!(partners, RowFamily::Partners);
    fam!(participates, RowFamily::Participates);
    fam!(authored, RowFamily::Authored);
    fam!(shown, RowFamily::Shown);
    fam!(map_succession, RowFamily::MapSuccession);

    let total: usize = counts.iter().map(|(_, n)| *n).sum();
    println!("DB-2a ROW CANON: {total} rows round-tripped across {} families", counts.len());
    for (family, n) in &counts {
        println!("  [{}] {}: {n}", family.ordinal(), family.name());
    }

    assert_eq!(counts.len(), RowFamily::ALL.len(), "every row family must be walked");
    let walked: Vec<RowFamily> = counts.iter().map(|(f, _)| *f).collect();
    assert_eq!(walked, RowFamily::ALL.to_vec(), "families must be walked in ordinal order");

    let expected: Vec<(RowFamily, usize)> = vec![
        (RowFamily::ContainsBible, 2_378),
        (RowFamily::ContainsConcord, 270),
        (RowFamily::Attests, 33_355),
        (RowFamily::Succession, 13),
        (RowFamily::CanonSuccession, 1_253),
        (RowFamily::DatedBy, 912),
        (RowFamily::LocatedAt, 955),
        (RowFamily::Fulfills, 24),
        (RowFamily::Typology, 16),
        (RowFamily::NamedAfter, 18),
        (RowFamily::Catechism, 6_568),
        (RowFamily::CommentsOn, 50_602),
        (RowFamily::SpokenBy, 470),
        (RowFamily::SpokenAt, 6_381),
        (RowFamily::Mentions, 35_852),
        (RowFamily::CrossRefs, 343_558),
        (RowFamily::Quotes, 0),
        (RowFamily::Confesses, 0),
        (RowFamily::CorrespondsBible, 0),
        (RowFamily::TemporalAdjacency, 911),
        (RowFamily::Analogue, 1),
        (RowFamily::Occurs, 431_280),
        (RowFamily::ParentOf, 1_776),
        (RowFamily::Partners, 104),
        (RowFamily::Participates, 714),
        (RowFamily::Authored, 0),
        (RowFamily::Shown, SHOWN_ROWS),
        (RowFamily::MapSuccession, MAP_STEPS),
    ];
    assert_eq!(counts, expected, "per-family row counts");
    assert_eq!(total, 917_411 + SHOWN_ROWS + MAP_STEPS, "the committed graph carries exactly 917,411 rows (483,412 + 431,280 Occurs at LEX-1 + 125 at D3 + 2,594 kin/partner/participation rows at D5) plus what the maps show and their steps");

    assert_eq!(
        round_trip_family(
            &[Quotes {
                quoting: TextLocus { at: TextRef::Bible(vr(40, 4, 4)), span: None },
                quoted: blr((5, 8, 3), (5, 8, 3)),
                provenance: "curated/quotes".into(),
            }],
            RowFamily::Quotes
        ),
        1
    );
    assert_eq!(
        round_trip_family(
            &[Confesses {
                confessing: Locus::whole(ConcordRef { part: 1, article: 2, paragraph: 3 }),
                confessed: blr((45, 3, 28), (45, 3, 28)),
                provenance: "concord".into(),
                justification: Justification::default(),
            }],
            RowFamily::Confesses
        ),
        1
    );
    assert_eq!(
        round_trip_family(
            &[Corresponds::<BibleTag> {
                a: Locus { unit: vr(43, 3, 16), span: None },
                b: Locus { unit: vr(43, 3, 17), span: None },
                provenance: "alignment".into(),
            }],
            RowFamily::CorrespondsBible
        ),
        1
    );
}

#[test]
fn canonical_id_strings_parse_back_for_every_node_and_every_index_position() {
    let g = committed_graph();
    let mut count = 0usize;

    fn check(p: &Position, count: &mut usize) {
        let s = position_str(p);
        let back = parse_position(&s, "$.position")
            .unwrap_or_else(|e| panic!("position `{s}` failed to parse back: {e}"));
        assert_eq!(&back, p, "position `{s}` did not round-trip");
        *count += 1;
    }

    for id in g.nodes.keys() {
        check(&Position::Node(id.clone()), &mut count);
    }
    for ix in g.indexes.values() {
        for p in ix.fwd.keys().chain(ix.inv.keys()) {
            check(p, &mut count);
        }
    }
    for ix in g.symmetric_indexes.values() {
        for p in ix.fwd.keys().chain(ix.inv.keys()) {
            check(p, &mut count);
        }
    }

    println!("DB-2a POSITION CANON: {count} position strings round-tripped");
    assert!(count > 90_000, "every node is a position at minimum, found {count}");
}

struct RealSources {
    kjv_json: String,
    xrefs_tsv: String,
    atlas: atlas_core::data::AtlasData,
    eras: Vec<atlas_core::data::Era>,
    brainfuel: atlas_etl::brainfuel::BrainFuelCorpus,
    concord: atlas_graph::concord_adapter::ConcordBundle,
    kretzmann: atlas_etl::kretzmann::KretzmannCorpus,
    red_letter: atlas_etl::red_letter::RedLetterCorpus,
}

fn real_sources() -> RealSources {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");

    let kjv_json =
        std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt"))
        .expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = atlas_etl::compile::compile(&raw_dir, &curated_dir)
        .expect("data/raw + data/curated must compile")
        .data;
    let eras = atlas.eras.clone();
    let brainfuel = atlas_etl::brainfuel::read_all(&raw_dir.join("brain-fuel-bible"))
        .expect("data/raw/brain-fuel-bible must exist");
    let concord_corpus = atlas_etl::concord::read_all(&raw_dir.join("concord"), &curated_dir)
        .expect("data/raw/concord must exist");
    let sc_overlap_text = std::fs::read_to_string(curated_dir.join("concord-sc-overlap.toml"))
        .expect("data/curated/concord-sc-overlap.toml must exist");
    let sc_overlap = atlas_etl::concord::parse_sc_overlap(&sc_overlap_text)
        .expect("concord-sc-overlap.toml must parse");
    let kretzmann = atlas_etl::kretzmann::read_all(&raw_dir.join("kretzmann"), &atlas.verses)
        .expect("data/raw/kretzmann must exist");
    let (restored_verses, _case_report) =
        atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &atlas.verses);
    let red_letter = atlas_etl::red_letter::read_all(&raw_dir.join("red-letter"), &restored_verses)
        .expect("data/raw/red-letter must exist");

    RealSources {
        kjv_json,
        xrefs_tsv,
        atlas,
        eras,
        brainfuel,
        concord: atlas_graph::concord_adapter::ConcordBundle {
            corpus: concord_corpus,
            sc_overlap,
        },
        kretzmann,
        red_letter,
    }
}

fn build(s: &RealSources) -> Graph {
    atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(
        &s.kjv_json,
        &s.xrefs_tsv,
        &s.atlas,
        &s.eras,
        Some(&s.brainfuel),
        Some(&s.concord),
        Some(&s.kretzmann),
        Some(&s.red_letter),
    )
    .expect("the real committed sources must build")
    .0
}

fn compare_rows<T: Canon>(a: &[T], b: &[T], family: RowFamily) -> usize {
    assert_eq!(a.len(), b.len(), "{} row count differs between builds", family.name());
    for (i, (ra, rb)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            encode_row_in_family(family, ra.to_value()),
            encode_row_in_family(family, rb.to_value()),
            "{} row {i} encoded differently in two independent builds",
            family.name()
        );
    }
    a.len()
}

#[test]
fn encoding_is_deterministic_across_two_independent_builds() {
    let a = build(&real_sources());
    let b = build(&real_sources());

    assert_eq!(a.nodes.len(), b.nodes.len(), "node count differs between builds");
    let mut node_bytes = 0usize;
    for ((ida, na), (idb, nb)) in a.nodes.iter().zip(b.nodes.iter()) {
        assert_eq!(ida, idb, "node key order differs between builds");
        let ba = na.encode();
        let bb = nb.encode();
        assert_eq!(ba, bb, "node {ida:?} encoded differently in two independent builds");
        node_bytes += ba.len();
    }

    let mut rows = 0usize;
    macro_rules! fam {
        ($field:ident, $family:expr) => {
            rows += compare_rows(&a.$field, &b.$field, $family)
        };
    }
    fam!(contains_bible, RowFamily::ContainsBible);
    fam!(contains_concord, RowFamily::ContainsConcord);
    fam!(attests, RowFamily::Attests);
    fam!(succession, RowFamily::Succession);
    fam!(canon_succession, RowFamily::CanonSuccession);
    fam!(dated_by, RowFamily::DatedBy);
    fam!(located_at, RowFamily::LocatedAt);
    fam!(fulfills, RowFamily::Fulfills);
    fam!(typology, RowFamily::Typology);
    fam!(named_after, RowFamily::NamedAfter);
    fam!(catechism, RowFamily::Catechism);
    fam!(comments_on, RowFamily::CommentsOn);
    fam!(spoken_by, RowFamily::SpokenBy);
    fam!(spoken_at, RowFamily::SpokenAt);
    fam!(mentions, RowFamily::Mentions);
    fam!(cross_refs, RowFamily::CrossRefs);
    fam!(quotes, RowFamily::Quotes);
    fam!(confesses, RowFamily::Confesses);
    fam!(corresponds_bible, RowFamily::CorrespondsBible);
    fam!(temporal_adjacency, RowFamily::TemporalAdjacency);
    fam!(analogue, RowFamily::Analogue);
    fam!(occurs, RowFamily::Occurs);
    fam!(parent_of, RowFamily::ParentOf);
    fam!(partners, RowFamily::Partners);
    fam!(participates, RowFamily::Participates);
    fam!(authored, RowFamily::Authored);
    fam!(shown, RowFamily::Shown);
    fam!(map_succession, RowFamily::MapSuccession);

    println!(
        "DB-2a DETERMINISM: {} nodes ({node_bytes} canon bytes) + {rows} rows byte-identical across two independent builds",
        a.nodes.len()
    );
}
