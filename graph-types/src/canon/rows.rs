//! The row encodings: all 21 row families, and the parts they share.
//!
//! A row is the graph's unit of AUTHORSHIP -- the thing a human or an
//! adapter wrote -- and it is what the artifact stores; the index is
//! derived from it. So a row needs the same two promises the node
//! encoding makes (canonical bytes, total decode), plus one more:
//!
//! **The family rides OUTSIDE the row.** `encode_row_in_family` wraps the
//! row's value as `{"family":"located_at","row":{…}}`, and THAT is what
//! edge ids and section logical hashes hash (spec 3.4). Two families
//! whose rows happen to share a shape -- `SpokenBy` and `SpokenAt` differ
//! only in one member's name, `Analogue` and `TemporalAdjacency` in two --
//! can therefore never collide, because the family name is inside the
//! hashed bytes rather than assumed from context.
//!
//! Everything else is the node encoding's discipline, unchanged: object
//! keys in BTree order with no whitespace, enums as `{"Variant":payload}`
//! (unit variants carry `null`), `Option::None` as `null`, sets in
//! `BTreeSet` order, every closed object declaring its exact member set
//! through `expect_exact_keys` (R9), and every error carrying a
//! root-anchored path (R10).
//!
//! **Every `to_value` destructures `self`.** `let Self { a, b, .. } = self`
//! -- with every field NAMED and no `..` -- is how the encoder is closed
//! over its own type, exactly as `canon/node.rs` closes `payload_to_value`
//! over `NodePayload`. Reading `self.a` instead would let a field added to
//! a row struct compile straight out of the bytes: the row would keep
//! encoding, the decoder's struct literal would be the only thing that
//! complained, and only for a type whose decode is exercised. With the
//! destructure, a new field is a compile error in the one place that
//! decides what the bytes say. (Enum `to_value`s already destructure --
//! their `match` arms name every member.)
//!
//! Two things are worth naming, because they are where a row encoding
//! could quietly lose meaning:
//!
//! * **Typed ids re-validate their kind.** A `PlaceId` encodes through
//!   `AnyNodeId` as `"Place:jerusalem"` -- kind included -- and decodes
//!   back through `AnyNodeId::narrow`, so bytes that name an `Event`
//!   where the row wants a `Place` are a located error, not a silently
//!   mistyped row.
//! * **Validating constructors are re-run, never bypassed.**
//!   `TokenSpan::new` (start <= end), `LocusRange::new` (from <= to) and
//!   `Succession::new` (non-empty, distinct chain) are the only ways
//!   those types come into being here, so a hand-edited artifact cannot
//!   smuggle an inverted span or a duplicated chain link past the
//!   invariants the in-memory types promise.

use std::collections::{BTreeMap, BTreeSet};

use crate::chrono::{DatePlacement, DatedBy, Duration, PlacementBasis};
use crate::edge::{
    Analogue, Attests, CanonSuccession, CatechismLink, CommentsOn, Confesses, ContainerContent,
    Contains, Corresponds, CrossRef, Fulfills, Ground, Justification, LocatedAt, MentionedEntity,
    Mentions, NamedAfter, Namesake, Quotes, SpokenAt, SpokenBy, Succession, TemporalAdjacency,
    Typology,
};
use crate::id::{
    AnchorTag, CatechismItemTag, CommentaryItemTag, ContainerTag, EraTag, EventTag, KindTag,
    NarrativeTag, NodeId, PeopleGroupTag, PersonTag, PlaceTag, PolityTag, SourceTag,
};
use crate::text::{
    ConcordRef, Corpus, Locus, LocusRange, LocusSet, TextLocus, TextRef, TokenSpan, TranslationId,
    VerseRef,
};

use super::ids::{any_node_id_str, parse_any_node_id};
use super::{
    at_path, expect_arr, expect_exact_keys, expect_obj, expect_str, expect_variant, field,
    field_arr, field_i32, field_opt_str, field_str, field_u16, field_u32, field_u8, join, obj,
    opt_str,
    serialize, str_value, variant, Canon, CanonError, Value, ROOT,
};

// ------------------------------------------------------------- the manifest

/// The closed list of row tables (spec 5). The ORDER is the ordinal, and
/// the ordinal is what `edge_index.row_family` stores -- so a family may
/// be appended, never reordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RowFamily {
    ContainsBible,
    ContainsConcord,
    Attests,
    Succession,
    CanonSuccession,
    DatedBy,
    LocatedAt,
    Fulfills,
    Typology,
    NamedAfter,
    Catechism,
    CommentsOn,
    SpokenBy,
    SpokenAt,
    Mentions,
    CrossRefs,
    Quotes,
    Confesses,
    CorrespondsBible,
    TemporalAdjacency,
    Analogue,
}

impl RowFamily {
    pub const ALL: [RowFamily; 21] = [
        RowFamily::ContainsBible,
        RowFamily::ContainsConcord,
        RowFamily::Attests,
        RowFamily::Succession,
        RowFamily::CanonSuccession,
        RowFamily::DatedBy,
        RowFamily::LocatedAt,
        RowFamily::Fulfills,
        RowFamily::Typology,
        RowFamily::NamedAfter,
        RowFamily::Catechism,
        RowFamily::CommentsOn,
        RowFamily::SpokenBy,
        RowFamily::SpokenAt,
        RowFamily::Mentions,
        RowFamily::CrossRefs,
        RowFamily::Quotes,
        RowFamily::Confesses,
        RowFamily::CorrespondsBible,
        RowFamily::TemporalAdjacency,
        RowFamily::Analogue,
    ];

    /// The SQLite table name, verbatim from spec 5.
    pub fn name(self) -> &'static str {
        match self {
            RowFamily::ContainsBible => "contains_bible",
            RowFamily::ContainsConcord => "contains_concord",
            RowFamily::Attests => "attests",
            RowFamily::Succession => "succession",
            RowFamily::CanonSuccession => "canon_succession",
            RowFamily::DatedBy => "dated_by",
            RowFamily::LocatedAt => "located_at",
            RowFamily::Fulfills => "fulfills",
            RowFamily::Typology => "typology",
            RowFamily::NamedAfter => "named_after",
            RowFamily::Catechism => "catechism",
            RowFamily::CommentsOn => "comments_on",
            RowFamily::SpokenBy => "spoken_by",
            RowFamily::SpokenAt => "spoken_at",
            RowFamily::Mentions => "mentions",
            RowFamily::CrossRefs => "cross_refs",
            RowFamily::Quotes => "quotes",
            RowFamily::Confesses => "confesses",
            RowFamily::CorrespondsBible => "corresponds_bible",
            RowFamily::TemporalAdjacency => "temporal_adjacency",
            RowFamily::Analogue => "analogue",
        }
    }

    /// Derived from `ALL` rather than from a second match, so the ordinal
    /// and the list cannot drift apart.
    pub fn ordinal(self) -> u8 {
        Self::ALL.iter().position(|f| *f == self).expect("ALL lists every variant") as u8
    }

    pub fn from_ordinal(o: u8) -> Option<RowFamily> {
        Self::ALL.get(usize::from(o)).copied()
    }
}

/// The row's canonical bytes with its family OUTSIDE:
/// `{"family":"located_at","row":{…}}`. This is what edge ids and section
/// logical hashes hash (spec 3.4) -- see this module's own header for why
/// the family cannot be left implicit.
pub fn encode_row_in_family(family: RowFamily, row_value: Value) -> Vec<u8> {
    serialize(&obj(vec![("family", str_value(family.name())), ("row", row_value)]))
}

// ------------------------------------------------------------ tiny helpers

/// Decode a nested `Canon` value, splicing its own (root-anchored) path
/// into the caller's trail -- the composition `at_path` exists for.
fn sub<T: Canon>(v: &Value, path: &str) -> Result<T, CanonError> {
    T::from_value(v).map_err(|e| at_path(path, e))
}

fn field_sub<T: Canon>(
    m: &BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<T, CanonError> {
    let (v, p) = field(m, path, key)?;
    sub::<T>(v, &p)
}

fn field_opt_sub<T: Canon>(
    m: &BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<Option<T>, CanonError> {
    let (v, p) = field(m, path, key)?;
    match v {
        Value::Null => Ok(None),
        _ => Ok(Some(sub::<T>(v, &p)?)),
    }
}

/// A canonical SET from its array spelling. The array must be strictly
/// increasing in the set's own `Ord`, because that is the only order
/// `to_value` can emit: a duplicate or a re-ordering would decode to a
/// set that re-encodes to DIFFERENT bytes -- a second spelling of one
/// value, which is exactly what canonical forbids. Same discipline the
/// byte parser applies to object keys.
fn set_from_array<T: Canon + Ord>(arr: &[Value], path: &str) -> Result<BTreeSet<T>, CanonError> {
    let mut set: BTreeSet<T> = BTreeSet::new();
    for (i, item) in arr.iter().enumerate() {
        let p = join(path, &i.to_string());
        let decoded = sub::<T>(item, &p)?;
        if set.last().is_some_and(|prev| *prev >= decoded) {
            return Err(CanonError::new(
                p,
                "a set is spelled strictly increasing; this member repeats or precedes the one before it",
            ));
        }
        set.insert(decoded);
    }
    Ok(set)
}

fn opt_value<T: Canon>(o: &Option<T>) -> Value {
    o.as_ref().map_or(Value::Null, Canon::to_value)
}

/// A typed node id rides as the canonical `Kind:raw` string, so the KIND
/// is carried in the bytes and re-checked on the way back in.
fn id_value<K: KindTag>(id: &NodeId<K>) -> Value {
    str_value(&any_node_id_str(&id.erase()))
}

fn id_from_value<K: KindTag>(v: &Value, path: &str) -> Result<NodeId<K>, CanonError> {
    let s = expect_str(v, path)?;
    let any = parse_any_node_id(&s, path)?;
    any.narrow::<K>().map_err(|mismatch| {
        CanonError::new(
            path,
            format!(
                "node id `{s}` names kind {:?}, but this field holds a {:?} id",
                mismatch.found, mismatch.expected
            ),
        )
    })
}

fn field_id<K: KindTag>(
    m: &BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<NodeId<K>, CanonError> {
    let (v, p) = field(m, path, key)?;
    id_from_value::<K>(v, &p)
}

/// A unit variant's payload is `null` and nothing else -- the same
/// closedness `expect_exact_keys` gives struct variants.
fn expect_unit(v: &Value, path: &str) -> Result<(), CanonError> {
    match v {
        Value::Null => Ok(()),
        other => Err(CanonError::new(
            path,
            format!("a unit variant's payload is null, found {}", other.type_name()),
        )),
    }
}

fn unknown_variant<T>(name: &str, path: String) -> Result<T, CanonError> {
    Err(CanonError::new(path, format!("unknown variant `{name}`")))
}

// -------------------------------------------------------------- text parts

const VERSE_REF_KEYS: &[&str] = &["book", "chapter", "verse"];
const CONCORD_REF_KEYS: &[&str] = &["article", "paragraph", "part"];
const TOKEN_SPAN_KEYS: &[&str] = &["end", "layer", "start"];
const LOCUS_KEYS: &[&str] = &["span", "unit"];
const LOCUS_RANGE_KEYS: &[&str] = &["from", "to"];
const TEXT_LOCUS_KEYS: &[&str] = &["at", "span"];

impl Canon for VerseRef {
    fn to_value(&self) -> Value {
        let Self { book, chapter, verse } = self;
        obj(vec![
            ("book", Value::Int(i64::from(*book))),
            ("chapter", Value::Int(i64::from(*chapter))),
            ("verse", Value::Int(i64::from(*verse))),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, VERSE_REF_KEYS)?;
        Ok(VerseRef {
            book: field_u8(m, ROOT, "book")?,
            chapter: field_u16(m, ROOT, "chapter")?,
            verse: field_u16(m, ROOT, "verse")?,
        })
    }
}

impl Canon for ConcordRef {
    fn to_value(&self) -> Value {
        let Self { article, paragraph, part } = self;
        obj(vec![
            ("article", Value::Int(i64::from(*article))),
            ("paragraph", Value::Int(i64::from(*paragraph))),
            ("part", Value::Int(i64::from(*part))),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, CONCORD_REF_KEYS)?;
        Ok(ConcordRef {
            part: field_u8(m, ROOT, "part")?,
            article: field_u16(m, ROOT, "article")?,
            paragraph: field_u16(m, ROOT, "paragraph")?,
        })
    }
}

impl Canon for TokenSpan {
    fn to_value(&self) -> Value {
        let Self { end, layer, start } = self;
        obj(vec![
            ("end", Value::Int(i64::from(*end))),
            ("layer", str_value(&layer.0)),
            ("start", Value::Int(i64::from(*start))),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, TOKEN_SPAN_KEYS)?;
        let layer = TranslationId(field_str(m, ROOT, "layer")?);
        let start = field_u16(m, ROOT, "start")?;
        let end = field_u16(m, ROOT, "end")?;
        // start <= end is re-checked, not assumed.
        TokenSpan::new(layer, start, end)
            .map_err(|e| CanonError::new(ROOT, format!("invalid token span: {e:?}")))
    }
}

impl<C: Corpus> Canon for Locus<C>
where
    C::Ref: Canon,
{
    fn to_value(&self) -> Value {
        let Self { span, unit } = self;
        obj(vec![("span", opt_value(span)), ("unit", unit.to_value())])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, LOCUS_KEYS)?;
        Ok(Locus {
            unit: field_sub::<C::Ref>(m, ROOT, "unit")?,
            span: field_opt_sub::<TokenSpan>(m, ROOT, "span")?,
        })
    }
}

impl<C: Corpus> Canon for LocusRange<C>
where
    C::Ref: Canon,
{
    fn to_value(&self) -> Value {
        let Self { from, to } = self;
        obj(vec![("from", from.to_value()), ("to", to.to_value())])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, LOCUS_RANGE_KEYS)?;
        let from = field_sub::<Locus<C>>(m, ROOT, "from")?;
        let to = field_sub::<Locus<C>>(m, ROOT, "to")?;
        // from <= to is re-checked, not assumed.
        LocusRange::new(from, to)
            .map_err(|e| CanonError::new(ROOT, format!("invalid locus range: {e:?}")))
    }
}

impl<C: Corpus> Canon for LocusSet<C>
where
    C::Ref: Canon,
{
    fn to_value(&self) -> Value {
        // `BTreeSet` order IS the canonical order -- no sort needed, and
        // none permitted.
        let Self(loci) = self;
        Value::Arr(loci.iter().map(Canon::to_value).collect())
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        Ok(LocusSet(set_from_array::<Locus<C>>(expect_arr(v, ROOT)?, ROOT)?))
    }
}

impl Canon for TextRef {
    fn to_value(&self) -> Value {
        match self {
            TextRef::Bible(v) => variant("Bible", v.to_value()),
            TextRef::Concord(c) => variant("Concord", c.to_value()),
        }
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let (name, body) = expect_variant(v, ROOT)?;
        let p = join(ROOT, name);
        match name {
            "Bible" => Ok(TextRef::Bible(sub::<VerseRef>(body, &p)?)),
            "Concord" => Ok(TextRef::Concord(sub::<ConcordRef>(body, &p)?)),
            other => unknown_variant(other, p),
        }
    }
}

impl Canon for TextLocus {
    fn to_value(&self) -> Value {
        let Self { at, span } = self;
        obj(vec![("at", at.to_value()), ("span", opt_value(span))])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, TEXT_LOCUS_KEYS)?;
        Ok(TextLocus {
            at: field_sub::<TextRef>(m, ROOT, "at")?,
            span: field_opt_sub::<TokenSpan>(m, ROOT, "span")?,
        })
    }
}

// ----------------------------------------------------- justification parts

const JUSTIFICATION_KEYS: &[&str] = &["grounds", "text"];

impl Canon for Ground {
    fn to_value(&self) -> Value {
        match self {
            Ground::Scripture(r) => variant("Scripture", r.to_value()),
            Ground::Anchor(a) => variant("Anchor", id_value(a)),
            Ground::Source(s) => variant("Source", id_value(s)),
        }
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let (name, body) = expect_variant(v, ROOT)?;
        let p = join(ROOT, name);
        match name {
            "Scripture" => Ok(Ground::Scripture(sub(body, &p)?)),
            "Anchor" => Ok(Ground::Anchor(id_from_value::<AnchorTag>(body, &p)?)),
            "Source" => Ok(Ground::Source(id_from_value::<SourceTag>(body, &p)?)),
            other => unknown_variant(other, p),
        }
    }
}

impl Canon for Justification {
    fn to_value(&self) -> Value {
        let Self { grounds, text } = self;
        obj(vec![
            ("grounds", Value::Arr(grounds.iter().map(Canon::to_value).collect())),
            ("text", opt_str(text)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, JUSTIFICATION_KEYS)?;
        let (grounds, gp) = field_arr(m, ROOT, "grounds")?;
        Ok(Justification {
            text: field_opt_str(m, ROOT, "text")?,
            grounds: set_from_array::<Ground>(grounds, &gp)?,
        })
    }
}

// --------------------------------------------------------- chronology parts

const DURATION_KEYS: &[&str] = &["days", "months", "years"];
const ANCHOR_BINDING_KEYS: &[&str] = &["anchor", "offset"];
const REIGN_YEAR_KEYS: &[&str] = &["reign", "year_of_reign"];
const SEQUENCE_AFTER_KEYS: &[&str] = &["prior", "spacing"];
const ERA_ONLY_KEYS: &[&str] = &["era"];

impl Canon for Duration {
    fn to_value(&self) -> Value {
        let Self { days, months, years } = self;
        obj(vec![
            ("days", Value::Int(i64::from(*days))),
            ("months", Value::Int(i64::from(*months))),
            ("years", Value::Int(i64::from(*years))),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, DURATION_KEYS)?;
        Ok(Duration {
            years: field_i32(m, ROOT, "years")?,
            months: field_u8(m, ROOT, "months")?,
            days: field_u8(m, ROOT, "days")?,
        })
    }
}

impl Canon for DatePlacement {
    fn to_value(&self) -> Value {
        match self {
            DatePlacement::AnchorBinding { anchor, offset } => variant(
                "AnchorBinding",
                obj(vec![("anchor", id_value(anchor)), ("offset", offset.to_value())]),
            ),
            DatePlacement::ReignYear { reign, year_of_reign } => variant(
                "ReignYear",
                obj(vec![
                    ("reign", id_value(reign)),
                    ("year_of_reign", Value::Int(i64::from(*year_of_reign))),
                ]),
            ),
            DatePlacement::SequenceAfter { prior, spacing } => variant(
                "SequenceAfter",
                obj(vec![("prior", id_value(prior)), ("spacing", spacing.to_value())]),
            ),
            DatePlacement::EraOnly { era } => {
                variant("EraOnly", obj(vec![("era", id_value(era))]))
            }
        }
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let (name, body) = expect_variant(v, ROOT)?;
        let p = join(ROOT, name);
        let m = expect_obj(body, &p)?;
        match name {
            "AnchorBinding" => {
                expect_exact_keys(m, &p, ANCHOR_BINDING_KEYS)?;
                Ok(DatePlacement::AnchorBinding {
                    anchor: field_id::<AnchorTag>(m, &p, "anchor")?,
                    offset: field_sub::<Duration>(m, &p, "offset")?,
                })
            }
            "ReignYear" => {
                expect_exact_keys(m, &p, REIGN_YEAR_KEYS)?;
                Ok(DatePlacement::ReignYear {
                    reign: field_id::<AnchorTag>(m, &p, "reign")?,
                    year_of_reign: field_u8(m, &p, "year_of_reign")?,
                })
            }
            "SequenceAfter" => {
                expect_exact_keys(m, &p, SEQUENCE_AFTER_KEYS)?;
                Ok(DatePlacement::SequenceAfter {
                    prior: field_id::<EventTag>(m, &p, "prior")?,
                    spacing: field_sub::<Duration>(m, &p, "spacing")?,
                })
            }
            "EraOnly" => {
                expect_exact_keys(m, &p, ERA_ONLY_KEYS)?;
                Ok(DatePlacement::EraOnly { era: field_id::<EraTag>(m, &p, "era")? })
            }
            other => unknown_variant(other, p),
        }
    }
}

impl Canon for PlacementBasis {
    fn to_value(&self) -> Value {
        match self {
            PlacementBasis::Textual => variant("Textual", Value::Null),
            PlacementBasis::Traditional => variant("Traditional", Value::Null),
        }
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let (name, body) = expect_variant(v, ROOT)?;
        let p = join(ROOT, name);
        let basis = match name {
            "Textual" => PlacementBasis::Textual,
            "Traditional" => PlacementBasis::Traditional,
            other => return unknown_variant(other, p),
        };
        expect_unit(body, &p)?;
        Ok(basis)
    }
}

// ------------------------------------------------------------- entity parts

impl Canon for Namesake {
    fn to_value(&self) -> Value {
        match self {
            Namesake::PeopleGroup(g) => variant("PeopleGroup", id_value(g)),
            Namesake::Place(p) => variant("Place", id_value(p)),
            Namesake::Polity(p) => variant("Polity", id_value(p)),
        }
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let (name, body) = expect_variant(v, ROOT)?;
        let p = join(ROOT, name);
        match name {
            "PeopleGroup" => Ok(Namesake::PeopleGroup(id_from_value::<PeopleGroupTag>(body, &p)?)),
            "Place" => Ok(Namesake::Place(id_from_value::<PlaceTag>(body, &p)?)),
            "Polity" => Ok(Namesake::Polity(id_from_value::<PolityTag>(body, &p)?)),
            other => unknown_variant(other, p),
        }
    }
}

impl Canon for MentionedEntity {
    fn to_value(&self) -> Value {
        match self {
            MentionedEntity::Place(p) => variant("Place", id_value(p)),
            MentionedEntity::Person(p) => variant("Person", id_value(p)),
            MentionedEntity::PeopleGroup(g) => variant("PeopleGroup", id_value(g)),
            MentionedEntity::Event(e) => variant("Event", id_value(e)),
        }
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let (name, body) = expect_variant(v, ROOT)?;
        let p = join(ROOT, name);
        match name {
            "Place" => Ok(MentionedEntity::Place(id_from_value::<PlaceTag>(body, &p)?)),
            "Person" => Ok(MentionedEntity::Person(id_from_value::<PersonTag>(body, &p)?)),
            "PeopleGroup" => {
                Ok(MentionedEntity::PeopleGroup(id_from_value::<PeopleGroupTag>(body, &p)?))
            }
            "Event" => Ok(MentionedEntity::Event(id_from_value::<EventTag>(body, &p)?)),
            other => unknown_variant(other, p),
        }
    }
}

impl<C: Corpus> Canon for ContainerContent<C>
where
    C::Ref: Canon,
{
    fn to_value(&self) -> Value {
        match self {
            ContainerContent::Loci(set) => variant("Loci", set.to_value()),
            ContainerContent::Container(c) => variant("Container", id_value(c)),
        }
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let (name, body) = expect_variant(v, ROOT)?;
        let p = join(ROOT, name);
        match name {
            "Loci" => Ok(ContainerContent::Loci(sub::<LocusSet<C>>(body, &p)?)),
            "Container" => {
                Ok(ContainerContent::Container(id_from_value::<ContainerTag>(body, &p)?))
            }
            other => unknown_variant(other, p),
        }
    }
}

// --------------------------------------------------------- the 21 families

const CONTAINS_KEYS: &[&str] = &["container", "content", "justification", "provenance"];
const ATTESTS_KEYS: &[&str] = &["attestation", "event", "justification", "provenance"];
const SUCCESSION_KEYS: &[&str] = &["chain", "justification", "narrative", "provenance"];
const CANON_SUCCESSION_KEYS: &[&str] = &["justification", "next", "prior", "provenance"];
const DATED_BY_KEYS: &[&str] = &["basis", "event", "justification", "placement", "provenance"];
const LOCATED_AT_KEYS: &[&str] = &["event", "justification", "place", "provenance"];
const FULFILLS_KEYS: &[&str] = &["fulfillment", "justification", "prophecy", "provenance"];
const TYPOLOGY_KEYS: &[&str] =
    &["antitype_passage", "justification", "note", "provenance", "type_passage"];
const NAMED_AFTER_KEYS: &[&str] = &["eponym", "justification", "namesake", "provenance"];
const CATECHISM_KEYS: &[&str] = &["item", "justification", "locus", "provenance"];
const COMMENTS_ON_KEYS: &[&str] = &["item", "justification", "on", "provenance"];
const SPOKEN_BY_KEYS: &[&str] = &["justification", "locus", "provenance", "speaker"];
const SPOKEN_AT_KEYS: &[&str] = &["justification", "locus", "place", "provenance"];
const MENTIONS_KEYS: &[&str] = &["entity", "locus", "provenance"];
const CROSS_REF_KEYS: &[&str] =
    &["from", "provenance", "target_display", "to", "to_last", "votes"];
const QUOTES_KEYS: &[&str] = &["provenance", "quoted", "quoting"];
const CONFESSES_KEYS: &[&str] = &["confessed", "confessing", "justification", "provenance"];
const CORRESPONDS_KEYS: &[&str] = &["a", "b", "provenance"];
const TEMPORAL_ADJACENCY_KEYS: &[&str] = &["earlier", "later", "provenance"];
const ANALOGUE_KEYS: &[&str] = &["a", "b", "provenance"];

impl<C: Corpus> Canon for Contains<C>
where
    C::Ref: Canon,
{
    fn to_value(&self) -> Value {
        let Self { container, content, justification, provenance } = self;
        obj(vec![
            ("container", id_value(container)),
            ("content", content.to_value()),
            ("justification", justification.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, CONTAINS_KEYS)?;
        Ok(Contains {
            container: field_id::<ContainerTag>(m, ROOT, "container")?,
            content: field_sub::<ContainerContent<C>>(m, ROOT, "content")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for Attests {
    fn to_value(&self) -> Value {
        let Self { attestation, event, justification, provenance } = self;
        obj(vec![
            ("attestation", attestation.to_value()),
            ("event", id_value(event)),
            ("justification", justification.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, ATTESTS_KEYS)?;
        Ok(Attests {
            event: field_id::<EventTag>(m, ROOT, "event")?,
            attestation: field_sub(m, ROOT, "attestation")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for Succession {
    fn to_value(&self) -> Value {
        let Self { chain, justification, narrative, provenance } = self;
        obj(vec![
            ("chain", Value::Arr(chain.iter().map(id_value).collect())),
            ("justification", justification.to_value()),
            ("narrative", id_value(narrative)),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, SUCCESSION_KEYS)?;
        let (chain_items, cp) = field_arr(m, ROOT, "chain")?;
        let mut chain = Vec::with_capacity(chain_items.len());
        for (i, item) in chain_items.iter().enumerate() {
            chain.push(id_from_value::<EventTag>(item, &join(&cp, &i.to_string()))?);
        }
        // Non-empty and distinct are re-checked through the row's own
        // constructor -- a malformed chain cannot be stored, only fail to
        // decode.
        Succession::new(
            field_id::<NarrativeTag>(m, ROOT, "narrative")?,
            chain,
            field_str(m, ROOT, "provenance")?,
            field_sub::<Justification>(m, ROOT, "justification")?,
        )
        .map_err(|e| CanonError::new(cp, format!("invalid succession chain: {e:?}")))
    }
}

impl Canon for CanonSuccession {
    fn to_value(&self) -> Value {
        let Self { justification, next, prior, provenance } = self;
        obj(vec![
            ("justification", justification.to_value()),
            ("next", id_value(next)),
            ("prior", id_value(prior)),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, CANON_SUCCESSION_KEYS)?;
        Ok(CanonSuccession {
            prior: field_id::<ContainerTag>(m, ROOT, "prior")?,
            next: field_id::<ContainerTag>(m, ROOT, "next")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for DatedBy {
    fn to_value(&self) -> Value {
        let Self { basis, event, justification, placement, provenance } = self;
        obj(vec![
            ("basis", basis.to_value()),
            ("event", id_value(event)),
            ("justification", justification.to_value()),
            ("placement", placement.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, DATED_BY_KEYS)?;
        Ok(DatedBy {
            event: field_id::<EventTag>(m, ROOT, "event")?,
            placement: field_sub::<DatePlacement>(m, ROOT, "placement")?,
            basis: field_sub::<PlacementBasis>(m, ROOT, "basis")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
            provenance: field_str(m, ROOT, "provenance")?,
        })
    }
}

impl Canon for LocatedAt {
    fn to_value(&self) -> Value {
        let Self { event, justification, place, provenance } = self;
        obj(vec![
            ("event", id_value(event)),
            ("justification", justification.to_value()),
            ("place", id_value(place)),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, LOCATED_AT_KEYS)?;
        Ok(LocatedAt {
            event: field_id::<EventTag>(m, ROOT, "event")?,
            place: field_id::<PlaceTag>(m, ROOT, "place")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for Fulfills {
    fn to_value(&self) -> Value {
        let Self { fulfillment, justification, prophecy, provenance } = self;
        obj(vec![
            ("fulfillment", fulfillment.to_value()),
            ("justification", justification.to_value()),
            ("prophecy", prophecy.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, FULFILLS_KEYS)?;
        Ok(Fulfills {
            prophecy: field_sub(m, ROOT, "prophecy")?,
            fulfillment: field_sub(m, ROOT, "fulfillment")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for Typology {
    fn to_value(&self) -> Value {
        let Self { antitype_passage, justification, note, provenance, type_passage } = self;
        obj(vec![
            ("antitype_passage", antitype_passage.to_value()),
            ("justification", justification.to_value()),
            ("note", opt_str(note)),
            ("provenance", str_value(provenance)),
            ("type_passage", type_passage.to_value()),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, TYPOLOGY_KEYS)?;
        Ok(Typology {
            type_passage: field_sub(m, ROOT, "type_passage")?,
            antitype_passage: field_sub(m, ROOT, "antitype_passage")?,
            note: field_opt_str(m, ROOT, "note")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for NamedAfter {
    fn to_value(&self) -> Value {
        let Self { eponym, justification, namesake, provenance } = self;
        obj(vec![
            ("eponym", id_value(eponym)),
            ("justification", justification.to_value()),
            ("namesake", namesake.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, NAMED_AFTER_KEYS)?;
        Ok(NamedAfter {
            namesake: field_sub::<Namesake>(m, ROOT, "namesake")?,
            eponym: field_id::<PersonTag>(m, ROOT, "eponym")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for CatechismLink {
    fn to_value(&self) -> Value {
        let Self { item, justification, locus, provenance } = self;
        obj(vec![
            ("item", id_value(item)),
            ("justification", justification.to_value()),
            ("locus", locus.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, CATECHISM_KEYS)?;
        Ok(CatechismLink {
            locus: field_sub::<TextLocus>(m, ROOT, "locus")?,
            item: field_id::<CatechismItemTag>(m, ROOT, "item")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for CommentsOn {
    fn to_value(&self) -> Value {
        let Self { item, justification, on, provenance } = self;
        obj(vec![
            ("item", id_value(item)),
            ("justification", justification.to_value()),
            ("on", on.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, COMMENTS_ON_KEYS)?;
        Ok(CommentsOn {
            item: field_id::<CommentaryItemTag>(m, ROOT, "item")?,
            on: field_sub(m, ROOT, "on")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for SpokenBy {
    fn to_value(&self) -> Value {
        let Self { justification, locus, provenance, speaker } = self;
        obj(vec![
            ("justification", justification.to_value()),
            ("locus", locus.to_value()),
            ("provenance", str_value(provenance)),
            ("speaker", id_value(speaker)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, SPOKEN_BY_KEYS)?;
        Ok(SpokenBy {
            locus: field_sub(m, ROOT, "locus")?,
            speaker: field_id::<PersonTag>(m, ROOT, "speaker")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for SpokenAt {
    fn to_value(&self) -> Value {
        let Self { justification, locus, place, provenance } = self;
        obj(vec![
            ("justification", justification.to_value()),
            ("locus", locus.to_value()),
            ("place", id_value(place)),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, SPOKEN_AT_KEYS)?;
        Ok(SpokenAt {
            locus: field_sub(m, ROOT, "locus")?,
            place: field_id::<PlaceTag>(m, ROOT, "place")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl Canon for Mentions {
    fn to_value(&self) -> Value {
        let Self { entity, locus, provenance } = self;
        obj(vec![
            ("entity", entity.to_value()),
            ("locus", locus.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, MENTIONS_KEYS)?;
        Ok(Mentions {
            locus: field_sub::<TextLocus>(m, ROOT, "locus")?,
            entity: field_sub::<MentionedEntity>(m, ROOT, "entity")?,
            provenance: field_str(m, ROOT, "provenance")?,
        })
    }
}

impl Canon for CrossRef {
    fn to_value(&self) -> Value {
        let Self { from, provenance, target_display, to, to_last, votes } = self;
        obj(vec![
            ("from", from.to_value()),
            ("provenance", str_value(provenance)),
            ("target_display", str_value(target_display)),
            ("to", to.to_value()),
            ("to_last", opt_value(to_last)),
            ("votes", Value::Int(i64::from(*votes))),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, CROSS_REF_KEYS)?;
        Ok(CrossRef {
            from: field_sub::<TextLocus>(m, ROOT, "from")?,
            to: field_sub::<TextLocus>(m, ROOT, "to")?,
            to_last: field_opt_sub::<TextLocus>(m, ROOT, "to_last")?,
            target_display: field_str(m, ROOT, "target_display")?,
            votes: field_u32(m, ROOT, "votes")?,
            provenance: field_str(m, ROOT, "provenance")?,
        })
    }
}

impl Canon for Quotes {
    fn to_value(&self) -> Value {
        let Self { provenance, quoted, quoting } = self;
        obj(vec![
            ("provenance", str_value(provenance)),
            ("quoted", quoted.to_value()),
            ("quoting", quoting.to_value()),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, QUOTES_KEYS)?;
        Ok(Quotes {
            quoting: field_sub::<TextLocus>(m, ROOT, "quoting")?,
            quoted: field_sub(m, ROOT, "quoted")?,
            provenance: field_str(m, ROOT, "provenance")?,
        })
    }
}

impl Canon for Confesses {
    fn to_value(&self) -> Value {
        let Self { confessed, confessing, justification, provenance } = self;
        obj(vec![
            ("confessed", confessed.to_value()),
            ("confessing", confessing.to_value()),
            ("justification", justification.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, CONFESSES_KEYS)?;
        Ok(Confesses {
            confessing: field_sub(m, ROOT, "confessing")?,
            confessed: field_sub(m, ROOT, "confessed")?,
            provenance: field_str(m, ROOT, "provenance")?,
            justification: field_sub::<Justification>(m, ROOT, "justification")?,
        })
    }
}

impl<C: Corpus> Canon for Corresponds<C>
where
    C::Ref: Canon,
{
    fn to_value(&self) -> Value {
        let Self { a, b, provenance } = self;
        obj(vec![
            ("a", a.to_value()),
            ("b", b.to_value()),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, CORRESPONDS_KEYS)?;
        Ok(Corresponds {
            a: field_sub::<Locus<C>>(m, ROOT, "a")?,
            b: field_sub::<Locus<C>>(m, ROOT, "b")?,
            provenance: field_str(m, ROOT, "provenance")?,
        })
    }
}

impl Canon for TemporalAdjacency {
    fn to_value(&self) -> Value {
        let Self { earlier, later, provenance } = self;
        obj(vec![
            ("earlier", id_value(earlier)),
            ("later", id_value(later)),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, TEMPORAL_ADJACENCY_KEYS)?;
        Ok(TemporalAdjacency {
            earlier: field_id::<EventTag>(m, ROOT, "earlier")?,
            later: field_id::<EventTag>(m, ROOT, "later")?,
            provenance: field_str(m, ROOT, "provenance")?,
        })
    }
}

impl Canon for Analogue {
    fn to_value(&self) -> Value {
        let Self { a, b, provenance } = self;
        obj(vec![
            ("a", id_value(a)),
            ("b", id_value(b)),
            ("provenance", str_value(provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, ANALOGUE_KEYS)?;
        Ok(Analogue {
            a: field_id::<EventTag>(m, ROOT, "a")?,
            b: field_id::<EventTag>(m, ROOT, "b")?,
            provenance: field_str(m, ROOT, "provenance")?,
        })
    }
}

/// DB-2b (RELMAP-1): the TOTAL family -> relation map. Every row family
/// lowers into exactly one relation (directed or symmetric); this is
/// the single spelling of that pairing, exhaustive by construction (no
/// wildcard arm), so adding a family without deciding its relation is a
/// compile error. `JustifiedBy`, `DerivedFrom` and `Parallel` have no
/// family: `JustifiedBy` is synthesised from grounds
/// (`event_world::add_justified_by`); the other two have zero producers.
impl RowFamily {
    pub fn relation(self) -> crate::graph::EdgeRel {
        use crate::edge::{RelationId as R, SymRelationId as S};
        use crate::graph::EdgeRel::{Directed, Symmetric};
        match self {
            RowFamily::ContainsBible | RowFamily::ContainsConcord => Directed(R::Contains),
            RowFamily::Attests => Directed(R::Attests),
            RowFamily::Succession | RowFamily::CanonSuccession => Directed(R::Succession),
            RowFamily::DatedBy => Directed(R::DatedBy),
            RowFamily::LocatedAt => Directed(R::LocatedAt),
            RowFamily::Fulfills => Directed(R::Fulfillment),
            RowFamily::Typology => Directed(R::Typology),
            RowFamily::NamedAfter => Directed(R::NamedAfter),
            RowFamily::Catechism => Symmetric(S::CatechismLink),
            RowFamily::CommentsOn => Directed(R::CommentsOn),
            RowFamily::SpokenBy => Directed(R::SpokenBy),
            RowFamily::SpokenAt => Directed(R::SpokenAt),
            RowFamily::Mentions => Directed(R::Mentions),
            RowFamily::CrossRefs => Directed(R::Cites),
            RowFamily::Quotes => Directed(R::Quotes),
            RowFamily::Confesses => Directed(R::Confesses),
            RowFamily::CorrespondsBible => Symmetric(S::Corresponds),
            RowFamily::TemporalAdjacency => Symmetric(S::TemporalAdjacency),
            RowFamily::Analogue => Symmetric(S::Analogue),
        }
    }
}
