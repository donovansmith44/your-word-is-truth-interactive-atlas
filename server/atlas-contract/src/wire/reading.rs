use serde::Serialize;

use atlas_core::time::TimeRange;
use atlas_core::wire::VerseGroup;

use super::catechism::CatechismRef;

#[derive(Debug, Serialize)]
pub struct Chapter {
    #[serde(rename = "ref")]
    pub sref: String,
    pub book: String,
    pub chapter: u16,
    pub verses: Vec<Verse>,
}

#[derive(Debug, Serialize)]
pub struct Verse {
    pub verse: u16,
    pub text: String,
    /// Batch R requirement 5: every curated place whose OWN `verse_links`
    /// names this verse, in place-id order (stable, not meaningful --
    /// there is no "primary mention" concept). Always present, possibly
    /// empty (most verses geocode to zero places) -- conditional presence
    /// lives on the CLIENT side (a mini-reader renders no hoverable span at
    /// all when this is empty), not as an omitted wire key, matching every
    /// other "always an array" field in this app's own wire (e.g.
    /// `Scene.quiet_places`).
    pub places: Vec<PlaceRef>,
    /// M-D3 (owner ruling U5): every person the graph's own `mentions`
    /// relation attests at this verse's own locus, in wire (row-insertion)
    /// order -- see `PersonRef`'s own doc comment for the full
    /// mentions-attested-only reasoning. Always present, possibly empty
    /// (most verses mention zero persons by name) -- SAME conditional-
    /// presence-lives-client-side convention `places` immediately above
    /// already establishes for this exact shape of field.
    pub persons: Vec<PersonRef>,
    /// Batch T requirement 5: this verse's own pericope heading, when it is
    /// a heading ANCHOR (`AtlasData::heading_for_verse`) -- omitted (not
    /// null), matching `Polity.transition`/`.fall`'s own conditional-
    /// presence convention, since the overwhelming majority of verses never
    /// anchor a heading at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<Heading>,
    /// Batch M-D2 (owner's cross-reference superscript directive, closed on
    /// the graph platform): this verse's own `cites` edge-summary COUNT at
    /// its TextUnit locus -- read straight off THE PORT (`GraphQuery::
    /// edge_summary`, design doc §5), the SAME generic query the `/api/node/
    /// {id}` handler itself calls (`graph::node_card`) -- never a
    /// second, bespoke tally. This is a genuinely NEW field (no bespoke
    /// predecessor existed), sourced NATIVELY from the generic contract from
    /// day one: the reader's own superscript markers ARE a query of the
    /// graph, per the design's own P1/P4 laws, just batched onto the
    /// EXISTING per-chapter response (one HTTP round trip for a whole
    /// chapter) rather than issuing 20-30 individual `GET /api/node/{id}`
    /// calls per chapter, which `client/IExplorableClient.cs`'s own concrete
    /// implementation is what a SINGLE node's lookup uses (see that file's
    /// own doc comment + `GraphExplorableClientTests` for the direct,
    /// unbatched proof the same contract is correct in isolation). Always
    /// present (0 for the overwhelming majority of verses, never omitted --
    /// the client's own superscript decision is `count == 0` -> nothing,
    /// never a missing-field special case).
    pub xref_count: usize,
    /// Batch RED-1: this verse's own aligned sub-verse red-letter spans
    /// (`GraphService.red_letter_spans`, precomputed once -- see that
    /// field's own doc comment for why this ONE companion is not
    /// graph-derivable), in ascending order. Always present, possibly
    /// empty -- SAME "always an array" convention `places`/`persons`
    /// above already establish (most verses carry none). KJV-ONLY by
    /// construction (decision 5: sub-verse precision is KJV-specific;
    /// this endpoint has never served any other translation -- ruling 5,
    /// "M1 is KJV-only").
    pub words_of_christ: Vec<WordsOfChristSpan>,
}

/// Batch R requirement 5 (place-in-verse hover -> marker blink): one place
/// mentioned in a verse, per `AtlasData::places_for_verse` (the reverse of
/// `Place::verse_links`). Deliberately lean -- id (to target a map marker)
/// and display name (for the client's own plain-text substring match against
/// the verse's rendered text, see `chapter`'s own doc comment for why there
/// is no richer per-mention offset data) -- mirrors `QuietPlace`'s own
/// "no more than the consumer needs" wire philosophy.
#[derive(Debug, Serialize)]
pub struct PlaceRef {
    pub id: String,
    pub name: String,
}

/// M-D3 (owner ruling U5, "in-text person and place name links,
/// mentions-attested ONLY -- no free-text matching ever"): one person the
/// graph's own `mentions` relation attests at a verse's own locus
/// (`GraphService::persons_by_verse`, precomputed off `Graph.mentions`,
/// NEVER a free-text/dictionary lookup against the verse's own rendered
/// words -- the exact "Sin"-the-city vs "sin"-the-noun hazard this
/// wording guards against: a name is only ever offered as a link because
/// a CURATED edge attests that specific entity at that specific verse,
/// not because some word in the text happens to match a name string
/// somewhere in the graph). Deliberately lean, mirroring `PlaceRef`'s
/// own "no more than the consumer needs" wire philosophy -- id (to open
/// the entity's own node popover) and display label (for the client's
/// own plain-text search locating WHERE in the verse's already-attested
/// text this entity's own name appears, the SAME PlaceMentions.cs
/// mechanism this batch generalizes rather than replaces -- attestation
/// is what THIS field guards; locating the substring within an ALREADY-
/// attested verse is a separate, later step, not a second matching
/// hazard).
#[derive(Debug, Serialize)]
pub struct PersonRef {
    pub id: String,
    pub name: String,
}

/// Batch T requirement 5: one resolved pericope heading, folded onto its own
/// anchor verse (`Verse.heading`) -- the SAME "fold onto the already-
/// shared fetch" precedent `VerseDetail.catechism`/`.events` already
/// establish, here for the CHAPTER fetch instead of the verse-detail one
/// (a reader rendering a whole chapter needs to know, per verse, whether a
/// heading belongs above it -- a second per-verse round trip would be
/// absurd). `event_id` is what a click opens (a new `EventNode`, client-side);
/// `title` is rendered directly, so the reader never needs a second fetch
/// just to show the heading text itself.
#[derive(Debug, Serialize)]
pub struct Heading {
    pub event_id: String,
    pub title: String,
    /// Batch HOTFIX-4 requirement 6: see `HeadingEntry::kind`'s own doc
    /// comment -- lets the reader's own heading rendering apply the quiet,
    /// non-traversable styling BEFORE a click, for a general-kind heading.
    pub kind: String,
    /// M-D1 requirement 1 (CHAPTER-BOUNDARY CONTINUATION): true iff this
    /// verse is NOT the container's own true first-covered verse but a
    /// later chapter its own coverage continues into -- lets the reader
    /// render a quiet continuation marker BEFORE a click, the same
    /// "affordance honesty travels as data, not inferred client-side"
    /// discipline `kind` above already establishes. Always present
    /// (`false` for every ordinary, PRIMARY heading -- the overwhelming
    /// majority), never omitted: unlike `heading` itself (conditional
    /// presence on `Verse`), once a heading exists at all its own
    /// continuation-ness is never in doubt.
    pub is_continuation: bool,
}

// M-D3 (owner ruling R5): `impl From<&atlas_core::data::HeadingEntry> for
// Heading` retired -- genuinely orphaned (grep-proven: no call site
// anywhere in this workspace). `chapter` (below) has built `Heading`
// directly from `graph.heading_index`'s own `atlas_graph::heading::
// HeadingEntry` since M-C2; this conversion's OWN source type
// (`atlas_core::data::HeadingEntry`, fed by `AtlasData::heading_for_verse`)
// has had no live reader since, and carried no lockstep test of its own
// (unlike `heading_for_verse` itself, which stays -- see that method's own
// doc comment / heading.rs's own module doc comment for the "dead, tested
// reference oracle" it remains). Recoverable from git history at the
// commit immediately preceding this one.

/// Batch RED-1 (owner order 2026-08-25, "Red letters on Jesus' words in
/// every translation"): one sub-verse red-letter span, CHAR (not byte)
/// offsets into this verse's own `text` field -- `graph.red_letter_spans`'s
/// own doc comment has the full "why char offsets" (C# `string` indexing is
/// UTF-16 code units; every character in this app's own KJV text is within
/// the Basic Multilingual Plane, so char count == UTF-16 code unit count
/// here, always). `start`/`end` are a half-open range (`text[start..end]`
/// in C# `Substring(start, end-start)` terms) -- KJV display renders this
/// EXACT sub-verse span; decision 5's own "ONE render rule."
#[derive(Debug, Serialize)]
pub struct WordsOfChristSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize)]
pub struct KretzmannChapter {
    pub verses: Vec<KretzmannChapterVerse>,
    pub version: String,
}

/// One verse's own commentary items, in document order. Only verses with
/// >=1 item appear at all (mirrors `Kretzmann.razor`'s own retired
/// client-side "if (items.Count > 0)" filter).
#[derive(Debug, Serialize)]
pub struct KretzmannChapterVerse {
    pub verse: u16,
    pub items: Vec<KretzmannChapterItem>,
}

/// KRETZ-SCALE-1 (batch-corp1-review.md Q-1, batch-corp1-report.md §5,
/// batch-finalp1-brief.md ticket 2, SANCTIONED SERVER ADDITION): one
/// `CommentaryItem` row within one verse of `GET /api/kretzmann/chapter/{cref}`'s
/// own chapter-scoped response.
#[derive(Debug, Serialize)]
pub struct KretzmannChapterItem {
    pub id: String,
    pub heading: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VerseDetail {
    #[serde(rename = "ref")]
    pub sref: String,
    pub text: String,
    /// Batch RED-1: this verse's own aligned sub-verse red-letter spans --
    /// see `Verse.words_of_christ`'s own doc comment (identical shape/
    /// convention, this endpoint's own single-verse sibling).
    pub words_of_christ: Vec<WordsOfChristSpan>,
    pub book_meta: BookMeta,
    pub events: Vec<VerseEvent>,
    pub cross_refs: Vec<CrossRef>,
    /// Batch F: catechism items citing this verse, via the SAME aggregation
    /// core `GET /api/catechism/{sref}` uses for a passage
    /// (`AtlasData::catechism_items_for_span`, a single-verse span here) --
    /// folded directly into this ALREADY-shared verse-detail fetch (rather
    /// than requiring a second round trip) so `CatechismSeamSection`
    /// (client) reads it off the exact same memoized `AtlasClient.Verse`
    /// call `VerseTextSectionProvider`/`CrossRefsSection` already share --
    /// "one fetch, not three," now not four either. Always present, possibly
    /// empty (most verses cite none) -- conditional presence lives on the
    /// CLIENT side (no section at all when empty), matching every other
    /// "always an array" field in this app's own wire.
    pub catechism: Vec<CatechismRef>,
    /// Batch PROV-1 (owner order 1, "the source from which it came"): THIS
    /// VERSE's own source -- the provenance id of the `TextUnit` node
    /// `text` above was rendered from, read straight off `Node::provenance`
    /// (`kjv_adapter`: `"kjv"`). The focus card's own attribution: the
    /// affordance beside a verse says "The King James Version" because THIS
    /// field said `kjv`, never because a client-side default assumed it.
    pub provenance: String,
    /// Batch PROV-1: every distinct provenance id behind the CROSS
    /// REFERENCES section -- the owner's own headline case ("sourced from
    /// openbible.com"). A LIST, not a string, deliberately: the day a
    /// second cross-reference corpus lands, this section starts naming
    /// both instead of quietly continuing to name the first. It is the
    /// `cross_refs` FAMILY's own distinct set (`ProvenanceIndex::
    /// by_family`), which is a true statement about every individual row
    /// exactly while the family is single-sourced -- pinned as such by
    /// `atlas-graph/tests/provenance_registry_real_data.rs`, and honest
    /// either way, since a family that gained a second source would serve
    /// both here rather than pick one.
    ///
    /// Always present, possibly empty (a verse with no cross references),
    /// same "always an array" convention as `cross_refs` itself.
    pub cross_refs_provenance: Vec<String>,
    /// Batch PROV-1: the same, for THE SMALL CATECHISM section -- the
    /// `catechism` family's own distinct provenance set.
    pub catechism_provenance: Vec<String>,
    // Batch T requirement 3 ("verse popover: event membership replaces
    // prev/next"): Batch N's own `narrative_positions` field (chronological
    // PRIOR/FOLLOWING, verse-keyed) is RETIRED here, cleanly -- verse-level
    // traversal no longer exists (see CONTRACT.md's own retirement note).
    // The PRE-EXISTING `events` field above (`Vec<VerseEvent>`, id +
    // label + verse_groups + places, populated the same way since before
    // this batch) is what the client's own NEW "EVENT" section reads
    // instead: it already names every EVENT-kind PASSAGE citing this verse,
    // which is exactly "event membership" -- no new wire field needed for
    // that half. Chronological PRIOR/FOLLOWING now lives entirely on the
    // EVENT node (`GET /api/narrative/event/{id}`, unchanged plumbing,
    // called by a new client-side caller -- see `events::event` below for
    // the richer id-keyed EVENT fetch that node also uses).
}

#[derive(Debug, Serialize)]
pub struct BookMeta {
    pub author: String,
    pub write_place: Option<String>,
    pub write_from: Option<i32>,
    pub write_to: Option<i32>,
}

/// The verse-detail endpoint's event shape: `SceneEvent`'s fields
/// (id/label/when/verse_groups) plus the event's place ids, so the client
/// can jump from a verse to "explore this event on the map" without a
/// second round trip.
#[derive(Debug, Serialize)]
pub struct VerseEvent {
    pub id: String,
    pub label: String,
    /// Batch HOTFIX-4 (drive-by fix, found while adding `kind` below):
    /// OMITTED (not the server's own internal undated placeholder) for a
    /// general-kind passage -- matches `EventDetail.when`'s own
    /// established fabrication guard exactly (`to_scene_event`'s own `se.when`
    /// is `e.when` unconditionally, which for `kind == "general"` is
    /// `TimeRange::undated()`, never meant to be presented as a real date).
    /// Dormant until now (this row's own CLIENT rendering only ever read
    /// `Id`/`Label`, never `When`), but a real gap against house doctrine
    /// nonetheless -- fixed here rather than left for a future reader of
    /// this exact struct to copy the bad precedent forward.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<TimeRange>,
    pub verse_groups: Vec<VerseGroup>,
    pub places: Vec<String>,
    /// Batch HOTFIX-4 requirement 6 (AFFORDANCE HONESTY): this EVENT-kind
    /// PASSAGE's own `Event::kind` ("event" | "general") -- same reason as
    /// `Heading::kind`, one surface over (a verse's own EVENT membership
    /// row, not a reader heading): the client applies the quiet,
    /// non-traversable styling to a general-kind row BEFORE the click, no
    /// second fetch needed to find out.
    pub kind: String,
    /// Batch PROV-1 (owner order 1): THIS ROW's own source -- the
    /// provenance id of the Event NODE this membership row points at, read
    /// straight off `Node::provenance` (`event_world::event_provenance`:
    /// `"theographic"` for an imported event, `"curated"` for one this
    /// project authored). Additive-only, the `words_of_christ` precedent --
    /// every field above serves byte-identically to before this batch.
    ///
    /// Deliberately the TARGET NODE's provenance, not the `Attests` row's:
    /// this row answers "which event is this verse part of," and the honest
    /// attribution for THAT claim is whoever says the event exists. The
    /// `Attests` rows' own provenance is served where it is actually
    /// rendered -- `EventDetail::witnesses_provenance`, on the PARALLEL
    /// ACCOUNTS section.
    ///
    /// This field IS the genuinely mixed one: a verse belonging to both an
    /// imported event and a hand-authored one names BOTH sources on its own
    /// section heading, which is the total-capture-honesty case at the
    /// verse frontier.
    pub provenance: String,
}

#[derive(Debug, Serialize)]
pub struct CrossRef {
    pub target: String,
    pub votes: i32,
    pub preview: String,
    /// PROV-1 FIX ROUND 1 (review M-3): the attribution for the CROSS
    /// REFERENCES section these rows belong to, so a PASSAGE node's section
    /// gets a "?" like a verse's.
    ///
    /// GRANULARITY, SAID ONCE AND PLAINLY (fix round 2, review M-NEW-1):
    /// this is a SECTION-level value carried on the ELEMENT because the
    /// endpoint is a bare array with no envelope to hang it on. Every
    /// element of one response receives the identical set. It is NOT
    /// per-row attribution and must never be described as such -- a
    /// comment in `reading::verse` did describe it that way and was
    /// corrected. Per-row is not available here to be had:
    /// `graph.cross_refs_by_from` holds `atlas_core::data::CrossRef`, which
    /// carries `target` and `votes` only; the graph-types row that does
    /// carry `provenance` has it projected away before this call site sees
    /// it. Widening the companion index to keep it is a contract-shaped
    /// decision this batch refused, and nothing here should imply it
    /// already happened.
    ///
    /// THE ORIGINAL DISCLOSURE GAVE A FALSE REASON and the review was right
    /// to say so: `GET /api/xrefs/{sref}` returns a bare `Vec<CrossRef>`,
    /// but the array was never where the field goes -- the ELEMENT is a
    /// struct with room for an additive field, which is precisely the move
    /// this batch already made twice (`VerseEvent.provenance`,
    /// `EventAnalogue.provenance`). No existing field moves; the response
    /// stays an array.
    ///
    /// A SET (`Vec<String>`), never one collapsed id, and that is measured
    /// rather than stylistic: `atlas_core::xrefs::aggregate_span_xrefs`
    /// UNIONS the `cites` rows of every member verse of the span and sums
    /// their votes, so one `CrossRef` is an aggregate of several
    /// underlying rows and there is no single row to attribute it to. The
    /// honest key is the `cross_refs` family's own distinct set -- the
    /// identical value `VerseDetail.cross_refs_provenance` already
    /// carries, pinned as `{openbible.info-cross-references}` by
    /// `the_per_family_provenance_map_of_the_real_artifact_is_pinned`. A
    /// second source landing in `cites` would surface on every row instead
    /// of hiding behind the first, which is the leper lesson in a type.
    pub provenance: Vec<String>,
}
