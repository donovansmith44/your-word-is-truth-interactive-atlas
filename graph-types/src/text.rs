//! Text addressing: corpora, layers, loci. The base text abstraction is
//! a wrapper around strings — not Bible-shaped. Corpora carry skeletons;
//! translations are layers; addresses are layer-neutral; sub-unit spans
//! are layer-tagged.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::id::Interned;

/// A corpus of text. Each corpus defines its typed position scheme.
pub trait Corpus {
    type Ref: Ord + Clone + std::fmt::Debug;
    const ID: &'static str;
    fn cite(r: &Self::Ref) -> String;
}

/// The corpus's standing relative to Scripture. Asymmetric by
/// construction: canonical confidence and attestation legality derive
/// from the role, never from per-assertion claims.
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

/// The Bible corpus: canon-structural skeleton; N translation layers
/// render it; the KJV is the canonical layer.
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

/// The Book of Concord: RESERVED — materializes when ingested. Its
/// structural scheme is decided at ingestion; the placeholder ref keeps
/// the variant honest and compiling.
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

/// Svebilius' Catechism — Olaus Svebilius' exposition, in Bishop Daniel
/// Juslenius' Finnish translation (Skara, 1745), Englished from Tero
/// Kotti's 2007 edition. SVEB-1.
///
/// ROLE: `Reference`, not `NormaNormata`. Luther's Small Catechism is a
/// confession and lives in the Book of Concord (part 7); Svebilius'
/// *Simple Exposition* is a teaching work ABOUT it — questions a catechist
/// asks, not a symbol the church subscribes. The distinction matters here
/// because `CorpusRole` is what an attestation law would read.
///
/// ADDRESSING: `section` is the work's own chief division (1 Preface,
/// 2 the Law, 3 the Creed, 4 the Lord's Prayer, 5 Baptism, 6 Confession,
/// 7 the Lord's Supper, 8 the Penitential Psalms, 9 Confession of Sins);
/// `unit` is the source's OWN question number, which RESTARTS at 1 in each
/// section (verified across all seven Q&A sections). Two sections are not
/// question-and-answer at all — 8 is seven psalms quoted whole, 9 is a
/// single confession formula — and there `unit` is a sequential index over
/// the section's own prose blocks, disclosed rather than force-fit into a
/// question number the source never assigns.
///
/// The source's numbering is carried FAITHFULLY, gaps included: section 4
/// (the Lord's Prayer) has 53 questions but numbers running to 54 — the
/// source itself skips 53. Renumbering to close the gap would silently
/// disagree with every printed copy, so the gap stands and the parser
/// reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SvebiliusTag;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SvebiliusRef {
    pub section: u8,
    pub unit: u16,
}

impl Corpus for SvebiliusTag {
    type Ref = SvebiliusRef;
    const ID: &'static str = "svebilius";
    fn cite(r: &Self::Ref) -> String {
        format!("Sveb {}.{}", r.section, r.unit)
    }
}

/// Token indices — necessarily tagged with the layer whose tokenization
/// they index: verse-level loci are layer-neutral, but a sub-unit span
/// only means something in one translation's wording.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenSpan {
    pub layer: TranslationId,
    pub start: u16,
    pub end: u16, // invariant: start <= end, validated at construction
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

/// Corpus-typed locus: a container for corpus C can only hold C's loci —
/// Scripture smeared into an extrabiblical container's content is
/// unrepresentable.
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
/// SVEB-1.
pub type SvebiliusLocus = Locus<SvebiliusTag>;

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

/// Container content: a SET of same-corpus loci. ∅ is lawful identity;
/// overlaps between containers are lawful; contiguity is not assumed.
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

/// Corpus-erased text address for the wire/UI and cross-corpus edges.
/// Widening from a typed locus is free; narrowing is a checked parse.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextRef {
    Bible(VerseRef),
    Concord(ConcordRef),
    /// SVEB-1.
    Svebilius(SvebiliusRef),
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

impl From<SvebiliusLocus> for TextLocus {
    fn from(l: SvebiliusLocus) -> Self {
        TextLocus { at: TextRef::Svebilius(l.unit), span: l.span }
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

/// Per-layer renderings of one skeleton unit (sweep F1: ONE node per
/// skeleton position; all layers as payload; canonical layer required).
pub type LayerMap = BTreeMap<TranslationId, String>;
