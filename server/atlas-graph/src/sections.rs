//! DB-2a spec §2.1: the section map -- which per-corpus SQLite section
//! (DB-2b's writer target) a node or row lands in.
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

use atlas_graph_types::canon::RowFamily;
use atlas_graph_types::edge::{Contains, EdgeId, RelationId};
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleTag, ConcordTag, Corpus};

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
    use atlas_graph_types::graph::EdgeRel;
    let (relation, _hash) = source_edge_id.0.split_once(':')?;
    let rel = RelationId::ALL.iter().copied().find(|r| format!("{r:?}") == relation)?;
    RowFamily::ALL.iter().copied().find(|f| {
        f.relation() == EdgeRel::Directed(rel)
            && matches!(f, RowFamily::DatedBy | RowFamily::Fulfills | RowFamily::Typology | RowFamily::NamedAfter)
    })
}
