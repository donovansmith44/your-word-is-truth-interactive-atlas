//! Spec §2.1 (the section map) and §3.4 (the per-section logical dump and
//! the version root) -- moved into `graph-types` at DB-4a so
//! `MemStore::publish` can stamp THE root: the manifest root over the
//! shipped sections' logical hashes, the same number the section writer
//! puts in `manifest.toml` and `SqliteSnapshot::version` reads back.
//! Zero-dep: pure functions over `Node`, `RowFamily` and `Graph`.
//!
//! **Placement rule (spec §2.1, verbatim): a node lives in the section of
//! the adapter that authored it.** A row's family says which adapter
//! authored IT -- with one exception. `ContainsBible` rows are authored
//! by whichever adapter owns the CONTAINER the row names (a book/chapter
//! container is the KJV adapter's own; any other container -- a curated
//! passage container, none shipped yet -- is Core's), so that ONE family
//! is split by row, never decided by family alone. `section_of_family`
//! therefore refuses to answer for it; `section_of_contains_bible` is the
//! per-row answer, and `section_of_justified_by` (the one function DB-2b's
//! writer calls for a `justified-by` entry) special-cases the same family
//! the same way.
//!
//! `Lexicon` is a real section variant (spec's five-section manifest) with
//! no inhabitant yet: no node payload and no row family maps to it today.
//! Do not invent one here -- it exists so `Section::MANIFEST_ORDER` is
//! already the full, final list DB-2b writes against.

use crate::canon::ids::any_node_id_str;
use crate::canon::{encode_row_in_family, obj, serialize, str_value, Canon, RowFamily, Value, DOMAIN_PREFIX};
use crate::edge::{Contains, EdgeId, RelationId};
use crate::graph::Graph;
use crate::id::ContentHash;
use crate::node::{Node, NodePayload};
use crate::sha256::sha256_prefixed_128;
use crate::text::{BibleTag, ConcordTag, Corpus};

/// The five per-corpus SQLite sections (spec §2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    Core,
    Kjv,
    Concord,
    Kretzmann,
    Lexicon,
}

impl Section {
    /// DB-2b's write order: every deploy's required sections first, then
    /// the optional per-corpus additions, in the spec's own listing order.
    pub const MANIFEST_ORDER: [Section; 5] =
        [Section::Core, Section::Kjv, Section::Concord, Section::Kretzmann, Section::Lexicon];

    /// The sections a manifest lists today: `MANIFEST_ORDER` minus
    /// `Lexicon` (no tables before LEX-1). The version root is over these.
    pub const SHIPPED: [Section; 4] = [Section::Core, Section::Kjv, Section::Concord, Section::Kretzmann];

    pub fn name(self) -> &'static str {
        match self {
            Section::Core => "core",
            Section::Kjv => "kjv",
            Section::Concord => "concord",
            Section::Kretzmann => "kretzmann",
            Section::Lexicon => "lexicon",
        }
    }

    /// Every deploy carries Core and Kjv; the rest are optional per-corpus
    /// additions a deploy may omit (spec §2.1).
    pub fn required(self) -> bool {
        matches!(self, Section::Core | Section::Kjv)
    }
}

/// The ONE container-raw prefix test both `section_of_node`'s Container
/// arm and `section_of_contains_bible` use (spec §2.1). Today the only
/// `"bible-"`-prefixed containers are book/chapter containers
/// (`bible_container_adapter`), so "starts with `bible-book-`/
/// `bible-chapter-`" and "starts with `bible-`" agree completely on the
/// shipped graph -- stated here as the narrower, spec-literal prefixes so
/// a future `"bible-"`-prefixed container that is NOT book/chapter shaped
/// does not silently inherit Kjv by accident.
fn section_of_container_raw(raw: &str) -> Section {
    if raw.starts_with("bible-book-") || raw.starts_with("bible-chapter-") {
        Section::Kjv
    } else if raw.starts_with("concord-") {
        Section::Concord
    } else {
        Section::Core
    }
}

/// Placement rule (spec §2.1): a node lives in the section of the adapter
/// that authored it.
///
///   `TextUnit{corpus: BibleTag::ID}` -> Kjv;
///   `TextUnit{corpus: ConcordTag::ID}` -> Concord;
///   `CommentaryItem` -> Kretzmann; `Container`: raw starts with
///   `"bible-book-"`/`"bible-chapter-"` -> Kjv, starts with `"concord-"`
///   -> Concord, else Core; every other kind -> Core. (`LexiconEntry` ->
///   Lexicon, when it exists -- no such node exists today.)
///
/// FINAL REVIEW items 2 + 9 (M4-2): the match is EXHAUSTIVE -- every
/// `NodePayload` kind is listed, there is no `_` arm, and the `TextUnit`
/// corpus is matched against `BibleTag::ID`/`ConcordTag::ID` (the consts
/// the corpora themselves define) rather than two string literals.
///
/// Why: DB-3 will add `NodePayload::LexiconEntry`, and LEX-1's acceptance
/// test is "no change to any other section's hash". A `_ => Core` arm
/// would have filed the new kind under Core silently and moved Core's
/// hash; now it is a compile error here, in the one place that decides.
/// The corpus arm is loud for the same reason the canon decoder
/// (`canon/node.rs`'s `corpus_from_value`) refuses an unknown corpus: a
/// third corpus is a new section decision, not a default.
pub fn section_of_node(node: &Node) -> Section {
    match &node.payload {
        NodePayload::TextUnit { corpus, .. } => match *corpus {
            BibleTag::ID => Section::Kjv,
            ConcordTag::ID => Section::Concord,
            other => unreachable!("TextUnit corpus {other}"),
        },
        NodePayload::CommentaryItem { .. } => Section::Kretzmann,
        // DB-3: the lexicon section's own kind (spec 7); no inhabitant until LEX-1.
        NodePayload::LexiconEntry { .. } => Section::Lexicon,
        NodePayload::Container { .. } => section_of_container_raw(&node.id.raw),
        // The eleven Core kinds, named one by one so a new variant cannot
        // join them by default.
        NodePayload::Event { .. }
        | NodePayload::Narrative { .. }
        | NodePayload::Place { .. }
        | NodePayload::Person { .. }
        | NodePayload::PeopleGroup { .. }
        | NodePayload::Anchor { .. }
        | NodePayload::Era { .. }
        | NodePayload::Polity { .. }
        | NodePayload::CatechismItem { .. }
        | NodePayload::Source { .. }
        | NodePayload::Translation { .. } => Section::Core,
    }
}

/// One family's section -- the CONSTANT answer for every family except
/// `ContainsBible`, which has none: it is split by row (see this module's
/// own header). Calling this with `RowFamily::ContainsBible` is a caller
/// error, not a silently-wrong constant -- use `section_of_contains_bible`
/// for that one family.
///
///   Kjv: `CanonSuccession`, `CrossRefs`, `SpokenBy`, `SpokenAt`.
///   Concord: `ContainsConcord`, `Quotes`, `Confesses`.
///   Kretzmann: `CommentsOn`.
///   Core: `Attests`, `Succession`, `DatedBy`, `LocatedAt`, `Fulfills`,
///   `Typology`, `NamedAfter`, `Catechism`, `Mentions`,
///   `CorrespondsBible`, `TemporalAdjacency`, `Analogue`.
pub fn section_of_family(f: RowFamily) -> Section {
    match f {
        RowFamily::ContainsBible => panic!(
            "ContainsBible has no per-family section -- it is split by row; \
             call section_of_contains_bible instead"
        ),
        RowFamily::CanonSuccession
        | RowFamily::CrossRefs
        | RowFamily::SpokenBy
        | RowFamily::SpokenAt => Section::Kjv,
        RowFamily::ContainsConcord | RowFamily::Quotes | RowFamily::Confesses => Section::Concord,
        RowFamily::CommentsOn => Section::Kretzmann,
        RowFamily::Attests
        | RowFamily::Succession
        | RowFamily::DatedBy
        | RowFamily::LocatedAt
        | RowFamily::Fulfills
        | RowFamily::Typology
        | RowFamily::NamedAfter
        | RowFamily::Catechism
        | RowFamily::Mentions
        | RowFamily::CorrespondsBible
        | RowFamily::TemporalAdjacency
        | RowFamily::Analogue => Section::Core,
    }
}

/// The one family split by row (spec §2.1): a `ContainsBible` row's
/// section follows its OWN container, not a family-wide constant --
/// book/chapter containers are the KJV adapter's; any other container (a
/// curated passage container -- none shipped yet) is Core's.
pub fn section_of_contains_bible(row: &Contains<BibleTag>) -> Section {
    section_of_container_raw(&row.container.0)
}

/// R3: the one function DB-2b's writer calls for a `justified-by` index
/// entry. These entries are synthesised from a ROW's own justification
/// (`event_world::add_justified_by`), never authored directly, so they
/// live in the section of that SOURCE row -- `section_of_family` applied
/// to the source family, except `ContainsBible`, which decides by
/// `source_container_raw` through the SAME rule `section_of_contains_bible`
/// applies to an actual row. (No `ContainsBible` row is a `justified-by`
/// source in the shipped graph today -- `add_justified_by` wires only
/// `DatedBy`/`Fulfills`/`Typology`/`NamedAfter` -- but the signature stays
/// general so a future source family needs no new function.)
pub fn section_of_justified_by(
    source_family: RowFamily,
    source_container_raw: Option<&str>,
) -> Section {
    match source_family {
        RowFamily::ContainsBible => section_of_container_raw(
            source_container_raw
                .expect("a ContainsBible justified-by source carries its container's raw id"),
        ),
        other => section_of_family(other),
    }
}

/// The small helper the justified-by real-data proof needs (survey gotcha
/// 2): a `justified-by` index entry's SUBJECT position is
/// `Position::Edge(source_edge_id)`, and `source_edge_id`'s own string
/// spells `"{RelationId:?}:{hash}"` (`graph_types::edge::entry_id`) -- so
/// the relation that minted the source row is recoverable from the edge
/// id's own text, with no new index needed. `add_justified_by` wires only
/// four source relations today (`DatedBy`, `Fulfillment`, `Typology`,
/// `NamedAfter`); this maps each to the `RowFamily` `section_of_justified_by`
/// wants THROUGH `RowFamily::relation` (DB-2b, RELMAP-1: the total
/// family->relation map), so the `Fulfillment`/`Fulfills` spelling split
/// is decided in one place, not re-spelled here.
pub fn justified_by_source_family(source_edge_id: &EdgeId) -> Option<RowFamily> {
    use crate::graph::EdgeRel;
    let (relation, _hash) = source_edge_id.0.split_once(':')?;
    let rel = RelationId::ALL.iter().copied().find(|r| format!("{r:?}") == relation)?;
    RowFamily::ALL.iter().copied().find(|f| {
        f.relation() == EdgeRel::Directed(rel)
            && matches!(f, RowFamily::DatedBy | RowFamily::Fulfills | RowFamily::Typology | RowFamily::NamedAfter)
    })
}

// ---------------------------------------------------------------------
// DB-4a: the tables a section carries, its logical dump, and the root
// ---------------------------------------------------------------------

/// Spec §5.0: every section's `PRAGMA user_version`; part of every
/// manifest line, therefore part of the root.
pub const SECTION_SCHEMA_VERSION: u32 = 14;

/// The row tables each section carries (spec §5.3–5.7). `ContainsBible`
/// is the one family with two homes -- curated passage containers in
/// core, book/chapter containers in kjv (`section_of_contains_bible`
/// decides per row).
pub fn row_tables_of(section: Section) -> &'static [RowFamily] {
    match section {
        Section::Core => &[
            RowFamily::ContainsBible,
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
        ],
        Section::Kjv => &[
            RowFamily::ContainsBible,
            RowFamily::CanonSuccession,
            RowFamily::CrossRefs,
            RowFamily::SpokenBy,
            RowFamily::SpokenAt,
        ],
        Section::Concord => &[RowFamily::ContainsConcord, RowFamily::Quotes, RowFamily::Confesses],
        Section::Kretzmann => &[RowFamily::CommentsOn],
        Section::Lexicon => &[],
    }
}

/// Whether the section carries a `reading_spine` (spec §5.4, §5.5).
pub fn has_spine(section: Section) -> bool {
    matches!(section, Section::Kjv | Section::Concord)
}

/// The corpus a section's spine belongs to.
pub fn spine_corpus(section: Section) -> Option<&'static str> {
    match section {
        Section::Kjv => Some(BibleTag::ID),
        Section::Concord => Some(ConcordTag::ID),
        _ => None,
    }
}

/// The tables the section's logical dump (spec §3.4) walks, in order:
/// `node`, each row family's table in `row_tables_of` order, then
/// `reading_spine` where present. `meta`, `justification`, `ground` and
/// `edge_index` are NOT in the dump: informational or derived.
pub fn logical_table_order(section: Section) -> Vec<&'static str> {
    let mut v = vec!["node"];
    v.extend(row_tables_of(section).iter().map(|f| f.name()));
    if has_spine(section) {
        v.push("reading_spine");
    }
    v
}

/// The `reading_spine` line body: `{"corpus":…,"node_id":…,"ord":N}`.
pub fn spine_line_body(corpus: &str, ord: i64, node_id: &str) -> Vec<u8> {
    serialize(&obj(vec![("corpus", str_value(corpus)), ("node_id", str_value(node_id)), ("ord", Value::Int(ord))]))
}

fn line(out: &mut Vec<u8>, table: &str, body: &[u8]) {
    out.extend_from_slice(table.as_bytes());
    out.push(b'\t');
    out.extend_from_slice(body);
    out.push(b'\n');
}

/// Spec §3.4, DB-2b's binding format: for each table in
/// `logical_table_order(section)`, for each row in primary-key order,
/// `<table>\t<canonical row JSON>\n`. Nodes in `any_node_id_str` BYTE
/// order (what SQLite's `ORDER BY id` yields), rows in ord order
/// (`ContainsBible` filtered by `section_of_contains_bible`), the spine in
/// ord order. Recomputable from the section file by streaming its tables
/// (`atlas_graph::sqlite::logical::logical_dump_of_db`); the two must agree.
pub fn logical_dump_section(g: &Graph, section: Section) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut nodes: Vec<&Node> = g.nodes.values().filter(|n| section_of_node(n) == section).collect();
    nodes.sort_by_cached_key(|n| any_node_id_str(&n.id));
    for n in nodes {
        line(&mut out, "node", &n.encode());
    }
    macro_rules! rows {
        ($family:expr, $vec:expr) => {
            for row in $vec.iter() {
                line(&mut out, $family.name(), &encode_row_in_family($family, row.to_value()));
            }
        };
    }
    for f in row_tables_of(section) {
        let f = *f;
        match f {
            RowFamily::ContainsBible => {
                for row in g.contains_bible.iter().filter(|r| section_of_contains_bible(r) == section) {
                    line(&mut out, f.name(), &encode_row_in_family(f, row.to_value()));
                }
            }
            RowFamily::ContainsConcord => rows!(f, g.contains_concord),
            RowFamily::Attests => rows!(f, g.attests),
            RowFamily::Succession => rows!(f, g.succession),
            RowFamily::CanonSuccession => rows!(f, g.canon_succession),
            RowFamily::DatedBy => rows!(f, g.dated_by),
            RowFamily::LocatedAt => rows!(f, g.located_at),
            RowFamily::Fulfills => rows!(f, g.fulfills),
            RowFamily::Typology => rows!(f, g.typology),
            RowFamily::NamedAfter => rows!(f, g.named_after),
            RowFamily::Catechism => rows!(f, g.catechism),
            RowFamily::CommentsOn => rows!(f, g.comments_on),
            RowFamily::SpokenBy => rows!(f, g.spoken_by),
            RowFamily::SpokenAt => rows!(f, g.spoken_at),
            RowFamily::Mentions => rows!(f, g.mentions),
            RowFamily::CrossRefs => rows!(f, g.cross_refs),
            RowFamily::Quotes => rows!(f, g.quotes),
            RowFamily::Confesses => rows!(f, g.confesses),
            RowFamily::CorrespondsBible => rows!(f, g.corresponds_bible),
            RowFamily::TemporalAdjacency => rows!(f, g.temporal_adjacency),
            RowFamily::Analogue => rows!(f, g.analogue),
        }
    }
    if let Some(corpus) = spine_corpus(section) {
        if let Some(spine) = g.reading.get(corpus) {
            for (i, id) in spine.order.iter().enumerate() {
                line(&mut out, "reading_spine", &spine_line_body(corpus, i as i64, &any_node_id_str(id)));
            }
        }
    }
    out
}

/// A section's logical hash: `sha256_prefixed_128(DOMAIN_PREFIX, dump)`.
pub fn logical_hash(dump: &[u8]) -> ContentHash {
    hash_from_digest(sha256_prefixed_128(DOMAIN_PREFIX, dump))
}

/// ON: the 16-byte digest IS the hash. OFF (the sibling repo's u64 state,
/// never served by the atlas after DB-4a): the digest's first eight bytes,
/// so these functions compile and stay law-tested in both states.
#[cfg(feature = "canon-ids")]
fn hash_from_digest(d: [u8; 16]) -> ContentHash {
    ContentHash(d)
}
#[cfg(not(feature = "canon-ids"))]
fn hash_from_digest(d: [u8; 16]) -> ContentHash {
    let mut eight = [0u8; 8];
    eight.copy_from_slice(&d[..8]);
    ContentHash(u64::from_be_bytes(eight))
}

/// Spec §2.2: `name|logical|schema_version|required\n` per section, in the
/// order given (manifest order). `logical` is the hash's `hex()`.
pub fn manifest_lines(entries: &[(&str, &str, u32, bool)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, logical, schema_version, required) in entries {
        out.extend_from_slice(format!("{name}|{logical}|{schema_version}|{}\n", if *required { "true" } else { "false" }).as_bytes());
    }
    out
}

/// The root over manifest lines: `sha256_prefixed_128(DOMAIN_PREFIX, lines)`.
pub fn root_of_lines(lines: &[u8]) -> ContentHash {
    hash_from_digest(sha256_prefixed_128(DOMAIN_PREFIX, lines))
}

/// THE version root (spec §3.4): the manifest root over `Section::SHIPPED`,
/// each section's logical hash over its own logical dump. Equal to what
/// the section writer writes as `manifest.toml`'s `root` and to
/// `SqliteSnapshot::version()`.
pub fn version_root(g: &Graph) -> ContentHash {
    let logicals: Vec<(Section, String)> = Section::SHIPPED.iter().map(|s| (*s, logical_hash(&logical_dump_section(g, *s)).hex())).collect();
    let entries: Vec<(&str, &str, u32, bool)> =
        logicals.iter().map(|(s, l)| (s.name(), l.as_str(), SECTION_SCHEMA_VERSION, s.required())).collect();
    root_of_lines(&manifest_lines(&entries))
}

#[cfg(test)]
mod laws {
    use super::*;
    use crate::edge::{Analogue, Justification, LocatedAt};
    use crate::id::{AnyNodeId, EventId, NodeKind, PlaceId};

    fn unit(raw: &str, corpus: &'static str) -> Node {
        let mut renderings = crate::text::LayerMap::new();
        renderings.insert(crate::text::TranslationId("kjv".into()), "x".into());
        Node { id: AnyNodeId { kind: NodeKind::TextUnit, raw: raw.into() }, payload: NodePayload::TextUnit { corpus, renderings }, provenance: "p".into() }
    }
    fn fixture() -> Graph {
        let mut g = Graph::default();
        for n in [unit("bible/1.1.2", "bible"), unit("bible/1.1.1", "bible"), unit("concord/1.1.1", "concord")] {
            g.nodes.insert(n.id.clone(), n);
        }
        g.reading.insert(
            "bible",
            crate::graph::ReadingSpine {
                order: vec![AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() }, AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.2".into() }],
            },
        );
        g.located_at.push(LocatedAt { event: EventId::new("e1"), place: PlaceId::new("p"), provenance: "p".into(), justification: Justification::default() });
        g.analogue.push(Analogue { a: EventId::new("e1"), b: EventId::new("e2"), provenance: "p".into() });
        g.build_indexes();
        g
    }

    #[test]
    fn the_section_dump_walks_node_then_families_then_spine_in_byte_order() {
        let g = fixture();
        let kjv = String::from_utf8(logical_dump_section(&g, Section::Kjv)).unwrap();
        let tags: Vec<&str> = kjv.lines().map(|l| l.split('\t').next().unwrap()).collect();
        assert_eq!(tags, ["node", "node", "reading_spine", "reading_spine"], "kjv: two bible nodes, no rows, the spine");
        assert!(kjv.starts_with("node\t{\"id\":\"TextUnit:bible/1.1.1\""), "byte order of any_node_id_str, not insertion order: {kjv}");
        assert!(kjv.contains("\nreading_spine\t{\"corpus\":\"bible\",\"node_id\":\"TextUnit:bible/1.1.1\",\"ord\":0}\n"));
        let core = String::from_utf8(logical_dump_section(&g, Section::Core)).unwrap();
        let tags: Vec<&str> = core.lines().map(|l| l.split('\t').next().unwrap()).collect();
        assert_eq!(tags, ["located_at", "analogue"], "core: no nodes here, the two rows in row_tables_of order");
        assert!(logical_dump_section(&g, Section::Kretzmann).is_empty());
        assert_eq!(logical_table_order(Section::Kjv).first().copied(), Some("node"));
    }

    #[test]
    fn the_root_is_the_manifest_root_over_the_shipped_sections_and_moves_with_a_row() {
        let g = fixture();
        let lines: Vec<(String, String, u32, bool)> = Section::SHIPPED
            .iter()
            .map(|s| (s.name().to_string(), logical_hash(&logical_dump_section(&g, *s)).hex(), SECTION_SCHEMA_VERSION, s.required()))
            .collect();
        let borrowed: Vec<(&str, &str, u32, bool)> = lines.iter().map(|(n, l, v, r)| (n.as_str(), l.as_str(), *v, *r)).collect();
        let text = String::from_utf8(manifest_lines(&borrowed)).unwrap();
        assert_eq!(text.lines().count(), 4);
        assert!(text.starts_with(&format!("core|{}|14|true\n", lines[0].1)));
        assert!(text.contains("|14|false\n"), "concord and kretzmann are optional");
        assert_eq!(version_root(&g), root_of_lines(text.as_bytes()));
        let mut g2 = fixture();
        g2.located_at[0].provenance = "q".into();
        assert_ne!(version_root(&g), version_root(&g2), "a row byte moves the root (spec 3.1 defect 1, closed)");
        let mut g3 = fixture();
        g3.build_indexes(); // derived state is not in the root
        assert_eq!(version_root(&g), version_root(&g3));
    }

    #[test]
    fn placement_moved_verbatim_and_every_family_has_one_home() {
        assert_eq!(section_of_family(RowFamily::CommentsOn), Section::Kretzmann);
        assert_eq!(section_of_justified_by(RowFamily::ContainsBible, Some("bible-chapter-GEN-1")), Section::Kjv);
        assert_eq!(section_of_justified_by(RowFamily::ContainsBible, Some("passage-creation")), Section::Core);
        assert_eq!(row_tables_of(Section::Lexicon).len(), 0);
        assert_eq!(Section::SHIPPED.len(), 4);
        // The closedness guard that replaces the retired whole-graph dump's
        // destructure: every family is written by exactly one shipped
        // section, ContainsBible by two.
        for f in RowFamily::ALL {
            let homes = Section::SHIPPED.iter().filter(|s| row_tables_of(**s).contains(&f)).count();
            assert_eq!(homes, if f == RowFamily::ContainsBible { 2 } else { 1 }, "{f:?}");
        }
    }
}
