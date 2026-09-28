//! The base text abstraction is a wrapper around strings, not Bible-shaped: a corpus carries a
//! skeleton, translations are layers, an address is layer-neutral, and a sub-unit span is
//! layer-tagged because only a layer's own tokenization can index it.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::id::Interned;

/// A corpus of text. Each corpus defines its typed position scheme.
pub trait Corpus {
    type Ref: Ord + Clone + std::fmt::Debug;
    const ID: &'static str;
    fn cite(r: &Self::Ref) -> String;
}

/// A corpus's standing relative to Scripture; `crate::ingest::Confidence` states the law that
/// derives from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CorpusRole {
    /// Scripture — the norming norm. The attestation law lives here.
    NormaNormans,
    /// The confessions — normed BY Scripture.
    NormaNormata,
    /// Ussher, Robertson, Crockett, Theographic, ...
    Reference,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TranslationId(pub Interned);

/// The Bible corpus: a canon-structural skeleton that N translation layers render, of which
/// the KJV is the canonical one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BibleTag;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VerseRef {
    pub book: u8,
    pub chapter: u16,
    pub verse: u16,
}

impl Corpus for BibleTag {
    type Ref = VerseRef;
    const ID: &'static str = "bible";
    fn cite(r: &Self::Ref) -> String {
        format!("{}.{}.{}", r.book, r.chapter, r.verse)
    }
}

/// Reserved: its structural scheme is decided at ingestion, and the placeholder reference
/// keeps the variant honest and compiling until then.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConcordTag;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConcordRef {
    pub part: u8,
    pub article: u16,
    pub paragraph: u16,
}

impl Corpus for ConcordTag {
    type Ref = ConcordRef;
    const ID: &'static str = "concord";
    fn cite(r: &Self::Ref) -> String {
        format!("BoC {}.{}.{}", r.part, r.article, r.paragraph)
    }
}

/// Necessarily tagged with the layer whose tokenization they index: a verse-level locus is
/// layer-neutral, but a sub-unit span only means something in one translation's wording.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenSpan {
    pub layer: TranslationId,
    pub start: u16,
    /// `start <= end`, validated by `TokenSpan::new`; the fields are public, so a struct
    /// literal can still violate it.
    pub end: u16,
}

impl TokenSpan {
    pub fn new(layer: TranslationId, start: u16, end: u16) -> Result<Self, SpanError> {
        if start <= end {
            Ok(TokenSpan { layer, start, end })
        } else {
            Err(SpanError::Inverted)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanError {
    Inverted,
}

/// The corpus parameter is the point: a container for corpus C can only hold C's loci, so
/// Scripture smeared into an extrabiblical container's content is unrepresentable.
#[derive(Debug)]
pub struct Locus<C: Corpus> {
    pub unit: C::Ref,
    pub span: Option<TokenSpan>,
}

impl<C: Corpus> Locus<C> {
    pub fn whole(unit: C::Ref) -> Self {
        Locus { unit, span: None }
    }
}

impl<C: Corpus> Clone for Locus<C> {
    fn clone(&self) -> Self {
        Locus { unit: self.unit.clone(), span: self.span.clone() }
    }
}
impl<C: Corpus> PartialEq for Locus<C> {
    fn eq(&self, o: &Self) -> bool {
        self.unit == o.unit && self.span == o.span
    }
}
impl<C: Corpus> Eq for Locus<C> {}
impl<C: Corpus> PartialOrd for Locus<C> {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl<C: Corpus> Ord for Locus<C> {
    fn cmp(&self, o: &Self) -> Ordering {
        self.unit.cmp(&o.unit).then_with(|| self.span.cmp(&o.span))
    }
}
impl<C: Corpus> std::hash::Hash for Locus<C>
where
    C::Ref: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, st: &mut H) {
        self.unit.hash(st);
        self.span.hash(st);
    }
}

pub type BibleLocus = Locus<BibleTag>;
pub type ConcordLocus = Locus<ConcordTag>;

/// Contiguous same-scheme range, from <= to.
#[derive(Debug)]
pub struct LocusRange<C: Corpus> {
    pub from: Locus<C>,
    pub to: Locus<C>,
}

impl<C: Corpus> LocusRange<C> {
    pub fn new(from: Locus<C>, to: Locus<C>) -> Result<Self, SpanError> {
        if from <= to {
            Ok(LocusRange { from, to })
        } else {
            Err(SpanError::Inverted)
        }
    }
}

impl<C: Corpus> Clone for LocusRange<C> {
    fn clone(&self) -> Self {
        LocusRange { from: self.from.clone(), to: self.to.clone() }
    }
}
impl<C: Corpus> PartialEq for LocusRange<C> {
    fn eq(&self, o: &Self) -> bool {
        self.from == o.from && self.to == o.to
    }
}
impl<C: Corpus> Eq for LocusRange<C> {}
impl<C: Corpus> PartialOrd for LocusRange<C> {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl<C: Corpus> Ord for LocusRange<C> {
    fn cmp(&self, o: &Self) -> Ordering {
        self.from.cmp(&o.from).then_with(|| self.to.cmp(&o.to))
    }
}
impl<C: Corpus> std::hash::Hash for LocusRange<C>
where
    C::Ref: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, st: &mut H) {
        self.from.hash(st);
        self.to.hash(st);
    }
}

pub type BibleLocusRange = LocusRange<BibleTag>;

/// A SET of same-corpus loci. The empty set is a lawful identity, overlaps between containers
/// are lawful, and contiguity is not assumed.
#[derive(Debug)]
pub struct LocusSet<C: Corpus>(pub BTreeSet<Locus<C>>);

impl<C: Corpus> Default for LocusSet<C> {
    fn default() -> Self {
        LocusSet(BTreeSet::new())
    }
}
impl<C: Corpus> Clone for LocusSet<C> {
    fn clone(&self) -> Self {
        LocusSet(self.0.clone())
    }
}

/// Corpus-erased, for the wire and for a cross-corpus edge. Widening from a typed locus is
/// free; narrowing is a checked parse.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextRef {
    Bible(VerseRef),
    Concord(ConcordRef),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextLocus {
    pub at: TextRef,
    pub span: Option<TokenSpan>,
}

impl From<BibleLocus> for TextLocus {
    fn from(l: BibleLocus) -> Self {
        TextLocus { at: TextRef::Bible(l.unit), span: l.span }
    }
}
impl From<ConcordLocus> for TextLocus {
    fn from(l: ConcordLocus) -> Self {
        TextLocus { at: TextRef::Concord(l.unit), span: l.span }
    }
}

impl TextLocus {
    /// Checked narrowing to Scripture — where law demands it.
    pub fn as_bible(&self) -> Option<BibleLocus> {
        match &self.at {
            TextRef::Bible(v) => Some(Locus { unit: v.clone(), span: self.span.clone() }),
            _ => None,
        }
    }
}

/// One node per skeleton position: every layer's rendering rides as payload, and the
/// canonical layer is required.
pub type LayerMap = BTreeMap<TranslationId, String>;
