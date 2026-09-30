//! A node lives in the section of the adapter that authored it, and a row's family says which
//! adapter authored it -- except the families whose section follows the container each row
//! names, which are therefore decided per row and never by family alone.

use crate::canon::ids::any_node_id_str;
use crate::canon::{encode_row_in_family, obj, serialize, str_value, Canon, RowFamily, Value, DOMAIN_PREFIX};
use crate::edge::{CanonSuccession, Contains, CrossRef, EdgeId, RelationId};
use crate::graph::Graph;
use crate::id::ContentHash;
use crate::node::{Node, NodePayload};
use crate::sha256::sha256_prefixed_128;
use crate::text::{BibleTag, ConcordTag, Corpus, TextRef};

/// The five per-corpus sections. `Lexicon` has no inhabitant yet: it exists so the manifest
/// order below is already the final list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    Core,
    Kjv,
    Concord,
    Kretzmann,
    Lexicon,
}

impl Section {
    /// Required sections first, then the optional per-corpus additions.
    pub const MANIFEST_ORDER: [Section; 5] =
        [Section::Core, Section::Kjv, Section::Concord, Section::Kretzmann, Section::Lexicon];

    /// The sections a manifest lists, and what the version root is taken over.
    pub const SHIPPED: [Section; 5] = [Section::Core, Section::Kjv, Section::Concord, Section::Kretzmann, Section::Lexicon];

    pub fn name(self) -> &'static str {
        match self {
            Section::Core => "core",
            Section::Kjv => "kjv",
            Section::Concord => "concord",
            Section::Kretzmann => "kretzmann",
            Section::Lexicon => "lexicon",
        }
    }

    /// Every deploy carries these two; the rest are per-corpus additions a deploy may omit.
    pub fn required(self) -> bool {
        matches!(self, Section::Core | Section::Kjv)
    }
}

/// The one test both the node and the per-row answers use. A corpus root's raw id IS the corpus
/// key, matched exactly; beneath it the narrow book/chapter prefixes rather than the bare corpus
/// one, so a future container that shares the corpus prefix without that shape cannot inherit
/// its section by accident.
fn section_of_container_raw(raw: &str) -> Section {
    match raw {
        BibleTag::ID => Section::Kjv,
        ConcordTag::ID => Section::Concord,
        _ if raw.starts_with("bible-book-") || raw.starts_with("bible-chapter-") => Section::Kjv,
        _ if raw.starts_with("concord-") => Section::Concord,
        _ => Section::Core,
    }
}

/// The match is exhaustive, with no catch-all arm and the corpus matched against the constants
/// the corpora declare: a new node kind or a third corpus is a section decision, and a default
/// arm would file it under one section silently and move that section's hash.
pub fn section_of_node(node: &Node) -> Section {
    match &node.payload {
        NodePayload::TextUnit { corpus, .. } => match *corpus {
            BibleTag::ID => Section::Kjv,
            ConcordTag::ID => Section::Concord,
            other => unreachable!("TextUnit corpus {other}"),
        },
        NodePayload::CommentaryItem { .. } => Section::Kretzmann,
        NodePayload::LexiconEntry { .. } => Section::Lexicon,
        NodePayload::Container { .. } => section_of_container_raw(&node.id.raw),
        // Named one by one so a new variant cannot join them by default.
        NodePayload::Event { .. }
        | NodePayload::Narrative { .. }
        | NodePayload::Place { .. }
        | NodePayload::Person { .. }
        | NodePayload::PeopleGroup { .. }
        | NodePayload::Anchor { .. }
        | NodePayload::Era { .. }
        | NodePayload::Map { .. }
        | NodePayload::Polity { .. }
        | NodePayload::CatechismItem { .. }
        | NodePayload::Source { .. }
        | NodePayload::Translation { .. } => Section::Core,
    }
}

/// Asking this for a per-row family is a caller error, not a silently wrong answer.
pub fn section_of_family(f: RowFamily) -> Section {
    match f {
        RowFamily::ContainsBible | RowFamily::CanonSuccession | RowFamily::CrossRefs => {
            panic!("{f:?} has no per-family section -- it is split by what each row names")
        }
        RowFamily::SpokenBy | RowFamily::SpokenAt => Section::Kjv,
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
        | RowFamily::Analogue
        | RowFamily::ParentOf
        | RowFamily::Partners
        | RowFamily::Participates
        | RowFamily::Authored
        | RowFamily::Shown
        | RowFamily::MapSuccession => Section::Core,
        RowFamily::Occurs => Section::Lexicon,
    }
}

/// The per-row answer: the section follows the container the row names.
pub fn section_of_contains_bible(row: &Contains<BibleTag>) -> Section {
    section_of_container_raw(&row.container.0)
}

/// The per-row answer: a step lives with the container it steps from.
pub fn section_of_canon_succession(row: &CanonSuccession) -> Section {
    section_of_container_raw(&row.prior.0)
}

/// The per-row answer: a citation lives with the text that cites, so the Book of Concord's
/// citations of Scripture ship with the Book of Concord.
pub fn section_of_cross_ref(row: &CrossRef) -> Section {
    match row.from.at {
        TextRef::Bible(_) => Section::Kjv,
        TextRef::Concord(_) => Section::Concord,
    }
}

/// The section of the row at `row_ord` of `family`: the family's own, or for a family split per
/// row, the one that row names. A synthesised justified-by entry lives in the section of its
/// source row, so this answers for it too.
pub fn section_of_row(g: &Graph, family: RowFamily, row_ord: usize) -> Section {
    match family {
        RowFamily::ContainsBible => section_of_contains_bible(&g.contains_bible[row_ord]),
        RowFamily::CanonSuccession => section_of_canon_succession(&g.canon_succession[row_ord]),
        RowFamily::CrossRefs => section_of_cross_ref(&g.cross_refs[row_ord]),
        other => section_of_family(other),
    }
}

/// An edge id's own text spells the relation that minted it, so the source family is
/// recoverable from the id with no extra index.
pub fn justified_by_source_family(source_edge_id: &EdgeId) -> Option<RowFamily> {
    use crate::graph::EdgeRel;
    let (relation, _hash) = source_edge_id.0.split_once(':')?;
    let rel = RelationId::ALL.iter().copied().find(|r| format!("{r:?}") == relation)?;
    RowFamily::ALL.iter().copied().find(|f| {
        f.relation() == EdgeRel::Directed(rel)
            && matches!(f, RowFamily::DatedBy | RowFamily::Fulfills | RowFamily::Typology | RowFamily::NamedAfter)
    })
}

/// Part of every manifest line, and therefore part of the root.
pub const SECTION_SCHEMA_VERSION: u32 = 16;

/// A per-row family appears under both of its homes.
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
            RowFamily::ParentOf,
            RowFamily::Partners,
            RowFamily::Participates,
            RowFamily::Authored,
            RowFamily::Shown,
            RowFamily::MapSuccession,
        ],
        Section::Kjv => &[
            RowFamily::ContainsBible,
            RowFamily::CanonSuccession,
            RowFamily::CrossRefs,
            RowFamily::SpokenBy,
            RowFamily::SpokenAt,
        ],
        Section::Concord => &[RowFamily::ContainsConcord, RowFamily::CanonSuccession, RowFamily::CrossRefs, RowFamily::Quotes, RowFamily::Confesses],
        Section::Kretzmann => &[RowFamily::CommentsOn],
        Section::Lexicon => &[RowFamily::Occurs],
    }
}

/// Whether the section carries a reading spine.
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

/// The tables beyond the nodes, rows and spine. Their rows reach the dump through the graph's
/// attached extras, and a law pins this list equal to the column specs the writer declares.
pub fn extra_tables_of(section: Section) -> &'static [&'static str] {
    match section {
        Section::Core => &[
            "place",
            "era",
            "polity_era",
            "event_date",
            "heading_index",
            "canon_book",
            "canon_chapter_verses",
            "book_meta",
            "chronology_anchor",
            "book_narration_window",
            "landmark",
            "land_mask_region",
            "catechism_part",
            "catechism_item",
            "catechism_item_verse",
            "catechism_question",
            "catechism_question_verse",
            "place_history",
            "place_history_name",
            "place_history_blurb",
            "place_history_verse",
            "place_name_alias",
            "place_name_alias_verse",
            "source_category",
            "source_entry",
            "provenance_entry",
        ],
        Section::Kjv => &["verse", "red_letter_span", "kjv_token"],
        Section::Concord => &["concord_unit", "concord_token"],
        Section::Kretzmann => &[],
        Section::Lexicon => &["lexicon_entry", "lexicon_domain", "token"],
    }
}

/// The one spelling of an extra table's row body, for the writer attaching it and the reader
/// re-encoding a query result: an object of column-value pairs, keys in byte order, no space.
pub fn extra_line_body(cols: Vec<(&str, Value)>) -> Vec<u8> {
    serialize(&obj(cols))
}

/// In dump order. The informational and derived tables are deliberately absent: a dump covers
/// only what the rows themselves say.
pub fn logical_table_order(section: Section) -> Vec<&'static str> {
    let mut v = vec!["node"];
    v.extend(row_tables_of(section).iter().map(|f| f.name()));
    if has_spine(section) {
        v.push("reading_spine");
    }
    v.extend(extra_tables_of(section));
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

/// The binding format: per table, per row in primary-key order, `<table>\t<canonical row>\n`,
/// with nodes in id byte order -- what a database's own ordered scan yields. A reader
/// recomputes this by streaming the section file, and the two must agree byte for byte.
pub fn logical_dump_section(g: &Graph, section: Section) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut nodes: Vec<&Node> = g.nodes.values().filter(|n| section_of_node(n) == section).collect();
    nodes.sort_by_cached_key(|n| any_node_id_str(&n.id));
    for n in nodes {
        line(&mut out, "node", &n.encode());
    }
    macro_rules! rows {
        ($family:expr, $rows:expr) => {
            for row in $rows {
                line(&mut out, $family.name(), &encode_row_in_family($family, row.to_value()));
            }
        };
    }
    for f in row_tables_of(section) {
        let f = *f;
        match f {
            RowFamily::ContainsBible => rows!(f, g.contains_bible.iter().filter(|r| section_of_contains_bible(r) == section)),
            RowFamily::CanonSuccession => rows!(f, g.canon_succession.iter().filter(|r| section_of_canon_succession(r) == section)),
            RowFamily::ContainsConcord => rows!(f, g.contains_concord.iter()),
            RowFamily::Attests => rows!(f, g.attests.iter()),
            RowFamily::Succession => rows!(f, g.succession.iter()),
            RowFamily::DatedBy => rows!(f, g.dated_by.iter()),
            RowFamily::LocatedAt => rows!(f, g.located_at.iter()),
            RowFamily::Fulfills => rows!(f, g.fulfills.iter()),
            RowFamily::Typology => rows!(f, g.typology.iter()),
            RowFamily::NamedAfter => rows!(f, g.named_after.iter()),
            RowFamily::Catechism => rows!(f, g.catechism.iter()),
            RowFamily::CommentsOn => rows!(f, g.comments_on.iter()),
            RowFamily::SpokenBy => rows!(f, g.spoken_by.iter()),
            RowFamily::SpokenAt => rows!(f, g.spoken_at.iter()),
            RowFamily::Mentions => rows!(f, g.mentions.iter()),
            RowFamily::CrossRefs => rows!(f, g.cross_refs.iter().filter(|r| section_of_cross_ref(r) == section)),
            RowFamily::Quotes => rows!(f, g.quotes.iter()),
            RowFamily::Confesses => rows!(f, g.confesses.iter()),
            RowFamily::CorrespondsBible => rows!(f, g.corresponds_bible.iter()),
            RowFamily::TemporalAdjacency => rows!(f, g.temporal_adjacency.iter()),
            RowFamily::Analogue => rows!(f, g.analogue.iter()),
            RowFamily::Occurs => rows!(f, g.occurs.iter()),
            RowFamily::ParentOf => rows!(f, g.parent_of.iter()),
            RowFamily::Partners => rows!(f, g.partners.iter()),
            RowFamily::Participates => rows!(f, g.participates.iter()),
            RowFamily::Authored => rows!(f, g.authored.iter()),
            RowFamily::Shown => rows!(f, g.shown.iter()),
            RowFamily::MapSuccession => rows!(f, g.map_succession.iter()),
        }
    }
    if let Some(corpus) = spine_corpus(section) {
        if let Some(spine) = g.reading.get(corpus) {
            for (i, id) in spine.order.iter().enumerate() {
                line(&mut out, "reading_spine", &spine_line_body(corpus, i as i64, &any_node_id_str(id)));
            }
        }
    }
    for table in extra_tables_of(section) {
        if let Some(bodies) = g.extra_tables.get(table) {
            for body in bodies {
                line(&mut out, table, body);
            }
        }
    }
    out
}

/// A section's logical hash: `sha256_prefixed_128(DOMAIN_PREFIX, dump)`.
pub fn logical_hash(dump: &[u8]) -> ContentHash {
    hash_from_digest(sha256_prefixed_128(DOMAIN_PREFIX, dump))
}

/// The digest IS the hash here; the other spelling truncates it, so both states compile and
/// stay law-tested.
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

/// One line per section, in the order given: `name|logical|schema_version|required`.
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

/// The manifest root over the shipped sections' logical hashes -- equal to what the section
/// writer records and what a snapshot reads back.
pub fn version_root(g: &Graph) -> ContentHash {
    let logicals: Vec<(Section, String)> = Section::SHIPPED.iter().map(|s| (*s, logical_hash(&logical_dump_section(g, *s)).hex())).collect();
    let entries: Vec<(&str, &str, u32, bool)> =
        logicals.iter().map(|(s, l)| (s.name(), l.as_str(), SECTION_SCHEMA_VERSION, s.required())).collect();
    root_of_lines(&manifest_lines(&entries))
}

#[cfg(test)]
mod laws {
    use super::*;
    use crate::edge::{Analogue, CrossRef, Justification, LocatedAt};
    use crate::id::{AnyNodeId, EventId, NodeKind, PlaceId};
    use crate::text::{ConcordRef, TextLocus, TextRef, VerseRef};

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
        assert_eq!(text.lines().count(), 5);
        assert!(text.ends_with(&format!("lexicon|{}|{SECTION_SCHEMA_VERSION}|false\n", lines[4].1)), "the lexicon line is last and optional");
        assert!(text.starts_with(&format!("core|{}|{SECTION_SCHEMA_VERSION}|true\n", lines[0].1)));
        assert!(text.contains(&format!("|{SECTION_SCHEMA_VERSION}|false\n")), "concord and kretzmann are optional");
        assert_eq!(version_root(&g), root_of_lines(text.as_bytes()));
        let mut g2 = fixture();
        g2.located_at[0].provenance = "q".into();
        assert_ne!(version_root(&g), version_root(&g2), "a row byte moves the root (spec 3.1 defect 1, closed)");
        let mut g3 = fixture();
        g3.build_indexes();
        assert_eq!(version_root(&g), version_root(&g3));
    }

    #[test]
    fn extra_tables_follow_the_graph_native_tables_and_move_the_root() {
        assert_eq!(extra_tables_of(Section::Kjv), &["verse", "red_letter_span", "kjv_token"]);
        assert_eq!(extra_tables_of(Section::Concord), &["concord_unit", "concord_token"]);
        assert!(extra_tables_of(Section::Kretzmann).is_empty());
        assert_eq!(extra_tables_of(Section::Lexicon), &["lexicon_entry", "lexicon_domain", "token"]);
        assert_eq!(extra_tables_of(Section::Core).len(), 26);
        let order = logical_table_order(Section::Core);
        assert_eq!(order.last().copied(), Some("provenance_entry"));
        assert!(order.iter().position(|t| *t == "place").unwrap() > order.iter().position(|t| *t == "analogue").unwrap());
        let mut all: Vec<&str> = Section::SHIPPED.iter().flat_map(|s| extra_tables_of(*s).iter().copied()).collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n, "extra table names are unique across sections");

        let g = fixture();
        let base = version_root(&g);
        let mut g2 = fixture();
        g2.extra_tables.insert(
            "verse",
            vec![extra_line_body(vec![
                ("book", Value::Int(0)),
                ("chapter", Value::Int(1)),
                ("node_id", str_value("TextUnit:bible/0.1.1")),
                ("verse", Value::Int(1)),
            ])],
        );
        assert_ne!(version_root(&g2), base, "an extra row moves the root");
        let kjv = String::from_utf8(logical_dump_section(&g2, Section::Kjv)).unwrap();
        assert!(
            kjv.ends_with(
                "reading_spine\t{\"corpus\":\"bible\",\"node_id\":\"TextUnit:bible/1.1.2\",\"ord\":1}\nverse\t{\"book\":0,\"chapter\":1,\"node_id\":\"TextUnit:bible/0.1.1\",\"verse\":1}\n"
            ),
            "extras come AFTER the spine, keys in byte order: {kjv}"
        );
        assert_eq!(logical_dump_section(&g2, Section::Core), logical_dump_section(&g, Section::Core), "a kjv extra does not touch core");
        let mut g3 = fixture();
        g3.extra_tables.insert("not_a_table", vec![b"{}".to_vec()]);
        assert_eq!(version_root(&g3), base, "a table no section lists is not in any dump");
    }

    #[test]
    fn a_corpus_root_files_under_its_corpus_section_like_the_containers_beneath_it() {
        // Arrange
        let shapes = ["bible", "bible-book-GEN", "concord", "concord-doc-small-catechism"];
        // Act
        let sections = shapes.map(section_of_container_raw);
        // Assert
        assert_eq!(sections, [Section::Kjv, Section::Kjv, Section::Concord, Section::Concord]);
    }

    #[test]
    fn placement_moved_verbatim_and_every_family_has_one_home() {
        assert_eq!(section_of_family(RowFamily::CommentsOn), Section::Kretzmann);
        assert_eq!(row_tables_of(Section::Lexicon), &[RowFamily::Occurs]);
        assert_eq!(section_of_family(RowFamily::Occurs), Section::Lexicon);
        assert_eq!(Section::SHIPPED.len(), 5);
        assert_eq!(Section::SHIPPED.to_vec(), Section::MANIFEST_ORDER.to_vec());
        for f in RowFamily::ALL {
            let homes = Section::SHIPPED.iter().filter(|s| row_tables_of(**s).contains(&f)).count();
            assert_eq!(homes, if matches!(f, RowFamily::ContainsBible | RowFamily::CanonSuccession | RowFamily::CrossRefs) { 2 } else { 1 }, "{f:?}");
        }
    }

    #[test]
    fn a_citation_lives_in_the_section_of_the_text_that_cites() {
        // Arrange
        let (bible, concord) = (citation(TextRef::Bible(VERSE)), citation(TextRef::Concord(PARAGRAPH)));
        // Act
        let sections = [section_of_cross_ref(&bible), section_of_cross_ref(&concord)];
        // Assert
        assert_eq!(sections, [Section::Kjv, Section::Concord]);
    }

    #[test]
    fn a_citation_is_dumped_with_the_section_of_the_text_that_cites() {
        // Arrange
        let mut g = Graph::default();
        g.cross_refs = vec![citation(TextRef::Bible(VERSE)), citation(TextRef::Concord(PARAGRAPH))];
        // Act
        let dumps = [Section::Kjv, Section::Concord].map(|s| String::from_utf8(logical_dump_section(&g, s)).unwrap());
        // Assert
        assert_eq!(
            dumps,
            [
                concat!(
                    "cross_refs\t",
                    r#"{"family":"cross_refs","row":{"from":{"at":{"Bible":{"book":0,"chapter":1,"verse":1}},"span":null},"provenance":"p","target_display":"GEN.1.1","to":{"at":{"Bible":{"book":0,"chapter":1,"verse":1}},"span":null},"to_last":null,"votes":0}}"#,
                    "\n"
                )
                .to_string(),
                concat!(
                    "cross_refs\t",
                    r#"{"family":"cross_refs","row":{"from":{"at":{"Concord":{"article":2,"paragraph":3,"part":7}},"span":null},"provenance":"p","target_display":"GEN.1.1","to":{"at":{"Bible":{"book":0,"chapter":1,"verse":1}},"span":null},"to_last":null,"votes":0}}"#,
                    "\n"
                )
                .to_string(),
            ]
        );
    }

    const VERSE: VerseRef = VerseRef { book: 0, chapter: 1, verse: 1 };
    const PARAGRAPH: ConcordRef = ConcordRef { part: 7, article: 2, paragraph: 3 };

    fn citation(from: TextRef) -> CrossRef {
        CrossRef {
            from: TextLocus { at: from, span: None },
            to: TextLocus { at: TextRef::Bible(VERSE), span: None },
            to_last: None,
            target_display: "GEN.1.1".into(),
            votes: 0,
            provenance: "p".into(),
        }
    }
}
