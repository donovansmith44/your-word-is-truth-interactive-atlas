//! The atlas data model: the compiled-file schema that ETL writes and the
//! server reads. Every record type derives `Serialize + Deserialize`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::time::{TimeRange, Year};

/// Verse counts per chapter for one book.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CanonBook {
    pub code: String,
    pub name: String,
    pub chapters: Vec<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Canon {
    pub books: Vec<CanonBook>,
}

/// A geocoded place. `verse_links` are canonical verse ids attached by
/// geocoding (not by event participation) and drive scripture-mode lighting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub verse_links: Vec<String>,
}

/// A datable happening. `places[0]` is the anchor place used for arrow
/// endpoints; `places` may list more than one place (e.g. a campaign
/// touching several locations), all of which light up in time mode.
///
/// Batch T ("events as the narrative nodes"): every `Event` IS the
/// EVENT-kind half of the owner's own `PASSAGE` abstraction (batch-t-brief.md
/// requirement 1, verbatim: "the Bible has a set of books and a set of
/// passages... passages may be events, they may be general passages").
/// `label` already serves as this passage's own TITLE (kept under its
/// pre-existing field name, not renamed, to avoid a mechanical ~15-file
/// rename with zero behavior change across code this batch does not
/// otherwise touch -- see the batch report's own disclosed decision).
/// `kind` is new (`"event"` for every real `Event` today -- structurally,
/// an `Event` always carries `when`/`places`, so it cannot represent a
/// dateless/placeless GENERAL-kind passage; that half of the owner's model
/// is a future batch's own sibling table, not built here, per this batch's
/// own scoped coverage decision) and ETL-validated against the two-value
/// enum, same "plain String, checked at ETL time, friendlier error" pattern
/// [`Landmark::kind`] already establishes.
///
/// Owner's own end-state (2026-08-21 ruling, fix-round-1): every verse in
/// the Bible ultimately belongs to at least one titled container like this
/// one -- today's Gospels+Acts+13-narratives coverage is a scoped subset,
/// not the destination; the full migration is real future work, not this
/// fix. `verses` is a CONTAINER's own verse SET, never mutated onto a verse
/// record -- the empty set is a lawful identity, and two containers
/// legitimately overlapping the same verse (see `heading_precedence`) is an
/// expected, not exceptional, shape as the graph grows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub label: String,
    pub when: TimeRange,
    pub places: Vec<String>,
    pub verses: Vec<String>,
    /// Batch T: `"event"` | `"general"` (validated by
    /// `atlas_etl::validate::run`). `"event"` carries a real `when`/
    /// `places` (traditional date + defensible place mapping); `"general"`
    /// (Batch T2: now real curated data, not just a modeled-but-unused
    /// enum value -- see `atlas_core::time::TimeRange::undated`'s own doc
    /// comment) is a titled container with neither -- `when` holds the
    /// undated sentinel and `places` stays empty, both supplied by the ETL
    /// parser itself, never curator-typed (`atlas_etl::curated::
    /// parse_events_extra` hard-errors if a curator writes `kind =
    /// "general"` together with a `from_year`/`to_year`/`places`, so "do
    /// not fabricate a date/place" is structural, not just a convention).
    /// Defaults to `"event"` so every `events.json` written before Batch T
    /// (and every existing test fixture) keeps deserializing with no
    /// migration.
    #[serde(default = "default_event_kind")]
    pub kind: String,
    /// Batch T requirement 1: PARALLEL WITNESSES -- "the set of per-book
    /// passages that recount the same event... one witness passage per
    /// Gospel." Empty for the overwhelming majority of events (every one
    /// this batch does not explicitly curate parallel accounts for) --
    /// NOT an error, and not "zero witnesses" in the ETL-validated sense
    /// (see `atlas_etl::validate::run`'s own witness-group check): an empty
    /// list means "this event has exactly one IMPLICIT witness," resolved
    /// server-side from `verses` grouped by book (`scene`/`handlers::event`'s
    /// own synthesis) -- never a fabricated placeholder, and never a reason
    /// to withhold this event's own reader heading or EVENT popover.
    #[serde(default)]
    pub witnesses: Vec<EventWitness>,
    /// Batch T requirement 1: provenance -- which Robertson's *Harmony of
    /// the Gospels* (1922, public domain) section grounds this event's own
    /// title/date/grouping, when Robertson-sourced (OT/Acts events this
    /// batch re-grounds without consulting Robertson leave this `None`,
    /// honestly, rather than inventing a section number). Curator-facing
    /// only -- carried onto the wire (`EventDetailOut::robertson_section`)
    /// so the mapping stays auditable, per the ambiguity ruling: "keep a
    /// robertson_section provenance field so the mapping stays auditable"
    /// even where this app's own DISPLAYED title is our own clearer
    /// phrasing (CC0), not Robertson's own archaic wording.
    #[serde(default)]
    pub robertson_section: Option<String>,
    /// Batch T2 (Acts provenance, owner's own ambiguity ruling, verbatim:
    /// "acts sections get their own provenance key, NOT robertson_section"):
    /// Acts's own sibling provenance field to `robertson_section` above --
    /// which Acts pericope-sectioning source grounds this event's own
    /// title/grouping, when Acts-sourced. Robertson's own 1922 Harmony is
    /// Gospels-only, so an Acts event's provenance is never claimed under
    /// that field even though the SHAPE (a short auditable citation
    /// string) is identical -- keeping the two fields separate means a
    /// reader/curator can never mistake an Acts section for a
    /// Robertson-verified one. `None` for every Gospel event (and for any
    /// Acts event this project has not yet sectioned). Counts as a real
    /// (layer-1) container in `heading_precedence`, exactly like
    /// `robertson_section` does -- see that function's own doc comment.
    #[serde(default)]
    pub acts_section: Option<String>,
    /// Batch W1 (whole-Bible titled verse containers, req 1's own provenance
    /// vocabulary: "kjv_superscription | theographic | atlas_section (our
    /// own sectioning, the sanctioned Acts-precedent fallback)"): the
    /// general, whole-Bible sibling of `acts_section` above -- same shape (a
    /// short auditable citation/disclosure string), same "counts as a real
    /// layer-1 container in `heading_precedence`" treatment, used for every
    /// book OUTSIDE the Gospels (`robertson_section`) and Acts
    /// (`acts_section`) once this batch's own OT sectioning starts (and,
    /// same mechanism, future non-Robertson/non-Acts books after it). Set
    /// two ways, mirroring how `robertson_section`/`acts_section` are each
    /// set: (a) inline, on a brand-new `[[event]]` row authored directly in
    /// `data/curated/passages/*.toml` (this batch's own new one-file-per-book
    /// directory, parsed by the SAME `EventToml`/`parse_events_extra`
    /// `robertson_section` already uses), for a container with no
    /// pre-existing compiled id to attach to; (b) via
    /// `data/curated/atlas-sections.toml`, a flat `event_id`-keyed merge
    /// file applied AFTER every event id is known (Theographic +
    /// events-extra.toml + passages/*.toml combined) -- the SAME merge
    /// timing/mechanism `acts-sections.toml` already established -- for
    /// PROMOTING a pre-existing bare Theographic event (already real
    /// title/date/place, CC BY-SA 4.0, already credited in LICENSES.md) to
    /// heading-worthy without a duplicate `[[event]]` row. `None` for every
    /// Gospel/Acts event and for any other book this project has not yet
    /// sectioned.
    #[serde(default)]
    pub atlas_section: Option<String>,
    /// Batch W3 (Job-Song of Solomon, Psalms granularity): the KJV's OWN
    /// provenance key, req 1's own vocabulary verbatim: "kjv_superscription
    /// | theographic | atlas_section" -- distinct from `atlas_section`
    /// (our own sectioning) because this field's own citation IS literal
    /// KJV text (public domain, redistributed per LICENSES.md's own "KJV
    /// text" row), never our own phrasing -- e.g. Psalm 3's own "A Psalm of
    /// David, when he fled from Absalom his son," quoted verbatim from
    /// PSA.3.1 itself, or Psalm 119's own acrostic Hebrew-letter stanza
    /// headers ("ALEPH.", "BETH.", ...), also literal verse-1-of-stanza
    /// text. Same shape, same "counts as a real layer-1 container in
    /// `heading_precedence`" treatment as `robertson_section`/
    /// `acts_section`/`atlas_section` above -- set ONLY inline, on a
    /// brand-new `[[event]]` row in `data/curated/passages/*.toml` (the
    /// SAME `EventToml`/`parse_events_extra` schema those three already
    /// use): every Batch W3 superscription-titled container is a brand-new
    /// container (Theographic models no per-Psalm "event" to promote via a
    /// merge file the way `atlas-sections.toml` promotes a bare Theographic
    /// event -- verified against the real compiled data before this field
    /// was added), so no `kjv-superscriptions.toml` merge-file sibling to
    /// `atlas-sections.toml` exists (would be dead code -- nothing to
    /// merge). `None` for every container whose own title is NOT itself a
    /// literal KJV citation (our own CC0 phrasing instead uses
    /// `atlas_section`, unchanged).
    #[serde(default)]
    pub kjv_superscription: Option<String>,
    /// Batch T requirement 1: citation-integrity note for THIS event's own
    /// date/grouping (distinct from each witness's own, narrower
    /// `EventWitness::ref_note`) -- names only sources actually consulted,
    /// same discipline `PolityDelta::ref_note`/`CatechismItem::ref_note`
    /// already establish. `None` is honest, not a gap, for an event whose
    /// date/grouping needed no further note beyond `robertson_section`
    /// itself (or needed neither).
    #[serde(default)]
    pub ref_note: Option<String>,
    /// Batch T requirement 2: sub-year chronological ordering. This atlas's
    /// `Year` model is year-granular (`TimeRange`, signed i32) -- two
    /// events genuinely a few days apart within the SAME traditional year
    /// (Palm Sunday through Resurrection, all AD 33) would otherwise be
    /// indistinguishable to `atlas_etl::validate::run`'s own chronological-
    /// leg check, which trusts a narrative's own curated leg ORDER as the
    /// chronology. `order_key` is that check's own explicit sub-year
    /// tiebreak (never a fake year offset, per the brief's own explicit
    /// instruction) -- within one calendar year, a narrative's legs must be
    /// non-decreasing by `order_key` too, not just by `when.from_year`.
    /// Defaults to 0 (meaningless, and never checked, for the vast majority
    /// of events, which are the ONLY event of their own narrative dated to
    /// their own year).
    #[serde(default)]
    pub order_key: i32,
}

fn default_event_kind() -> String {
    "event".to_string()
}

/// Manual (not derived) `Default`, purely so the many pre-existing
/// hand-built `Event { id: ..., label: ..., when: ..., places: ...,
/// verses: ... }` literals across this workspace's own tests/ETL (none of
/// which need to say anything about this batch's own new fields) can add a
/// single `..Default::default()` rather than each spelling out
/// `kind`/`witnesses`/`robertson_section`/`ref_note`/`order_key` by hand --
/// `#[derive(Default)]` isn't available here since `TimeRange` itself has
/// no `Default` (it validates through the fallible `TimeRange::new`); the
/// placeholder `when` below is never actually read by any site using this
/// (every one of them still states its own `when` explicitly).
impl Default for Event {
    fn default() -> Self {
        Event {
            id: String::new(),
            label: String::new(),
            when: TimeRange { from_year: 1, to_year: 1 },
            places: Vec::new(),
            verses: Vec::new(),
            kind: default_event_kind(),
            witnesses: Vec::new(),
            robertson_section: None,
            acts_section: None,
            atlas_section: None,
            kjv_superscription: None,
            ref_note: None,
            order_key: 0,
        }
    }
}

/// Batch T requirement 1: one witness account of an EVENT-kind `PASSAGE` --
/// "each witness a (book, verse-range, translation-mapped) record," the
/// owner's own words verbatim. `translations` is a REAL mapping (not a
/// single flat `Vec` with an implicit "it's KJV" comment) -- see
/// `crate::translation`'s own module doc comment for the fail-loud lookup
/// this indirection exists to support. `book` is a canonical 3-letter code
/// (e.g. `"MAT"`) -- deliberately NOT re-derivable from the verse ids alone
/// (a witness's own verses are always single-book by construction, but
/// carrying `book` explicitly means a witness with a currently-empty/
/// unresolvable translation entry still identifies WHICH Gospel it is,
/// e.g. for the client's own "Matthew" caption).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventWitness {
    pub book: String,
    /// translation code (lowercase, `crate::translation::DEFAULT_TRANSLATION`
    /// == `"kjv"` today) -> flat, individually-canonical verse ids, same
    /// convention `Event::verses`/`CatechismItem::verses` already use.
    pub translations: HashMap<String, Vec<String>>,
    /// This WITNESS's own citation-integrity note (e.g. "Robertson §164;
    /// Eusebian canon table I") -- distinct from the parent `Event::ref_note`,
    /// which grounds the EVENT's own date/grouping as a whole, not this one
    /// book's own account.
    #[serde(default)]
    pub ref_note: Option<String>,
    /// This WITNESS's own Robertson Harmony section, when it differs from
    /// (or the parent event carries none of) `Event::robertson_section` --
    /// most witnesses share their parent event's own section and leave this
    /// `None` rather than repeating it.
    #[serde(default)]
    pub robertson_section: Option<String>,
}

/// An ordered chain of event ids (`legs`) that `scene::build_arrows` turns
/// into consecutive arrows. ETL validates legs are non-decreasing by
/// `when.from_year`; scene composition trusts that invariant rather than
/// re-deriving it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Narrative {
    pub id: String,
    pub name: String,
    pub color: String,
    pub legs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era {
    pub id: String,
    pub name: String,
    pub from_year: Year,
    pub to_year: Year,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookMeta {
    pub book: String,
    pub author: String,
    pub write_place: Option<String>,
    pub write_from: Option<i32>,
    pub write_to: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossRef {
    pub target: String,
    pub votes: i32,
}

/// A curated landmark label (river/sea/mountain/region) rendered always-on,
/// non-interactive, on the World map — the "actual good and informative
/// map" landmark-labels feature. `kind` is one of `"water"`, `"mountain"`,
/// `"region"` (atlas-etl's `validate::run_landmarks` enforces the enum;
/// this struct itself accepts any string so a hand-authored curated file
/// with a typo fails ETL validation with a clear message rather than
/// silently refusing to deserialize with serde's own less-helpful error).
///
/// Batch C2 (design-direction.md's Atlas plate detail SECOND ADDENDUM,
/// "populated far field... broad standing regions... sized by geographic
/// extent"): `size` is an optional hint, one of `"sm"`, `"md"`, `"lg"`
/// (also enum-checked by `validate::run_landmarks`, same friendlier-error
/// reason as `kind`), `None` for every landmark curated before this batch.
/// Two effects, both in map.js: (1) font-size, mirroring BorderLayer's own
/// polity-label `data-size` sm/md/lg tiers; (2) tier-0 visibility —
/// `applyLabelTier` treats ANY landmark carrying an explicit size hint as a
/// deliberately-curated far-field context label, visible even at the FAR
/// (zoomed-out) tier regardless of its `kind`'s own default tier rule,
/// which otherwise only exempts `kind == "water"`. A landmark with no
/// `size` at all keeps the exact PRE-Batch-C2 kind-based tier behavior —
/// this field only ever ADDS visibility, never removes any.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Landmark {
    pub name: String,
    pub kind: String,
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub size: Option<String>,
}

/// Batch HOTFIX-6 (graph-wide chronology audit, `data/curated/
/// chronology-anchors.toml`'s own header has the full schema/design
/// rationale): one authoritative date on this project's own declared
/// traditional scale (Ussher's Annals of the World for the OT, this atlas's
/// own already-established AD-33 Passion anchor for the NT) -- THE canonical
/// chronology reference, consulted by both `atlas_etl::validate`'s own
/// fail-loud ETL-time checks and (loaded straight back off
/// `chronology-anchors.json`, the SAME compiled artifact, per the
/// controller's own "single source, two enforcement layers" instruction)
/// the property tests in `atlas_core::narrative` that assert the compiled
/// atlas actually ADHERES to this table (Amendment E's E1/E3/E4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronologyAnchor {
    /// Stable, machine-readable row key -- never renumbered/repositioned.
    /// Forward-compatible per the controller's own "single-feed chronology"
    /// end-state note: a future date-resolution table can reference this id
    /// to compute an anchor-relative year arithmetically without this
    /// file's own shape changing.
    pub id: String,
    pub label: String,
    /// The single authoritative year on this atlas's own declared scale.
    pub year: Year,
    /// The real compiled event id this anchor equals, when one binds
    /// CLEANLY (no disclosed scale tension) -- `None` where no single
    /// compiled event corresponds, or where binding would misrepresent an
    /// already-disclosed adjacency as a bug (an honest gap, not a
    /// shortcut) -- see `chronology-anchors.toml`'s own "DISCLOSED
    /// ADJACENCIES" header note.
    #[serde(default)]
    pub event_id: Option<String>,
    /// True for the anchors used as the E4 (era-partition) property test's
    /// own structural era boundaries -- always carries `event_id` when true
    /// (E4 needs a real timeline position to gate on).
    #[serde(default)]
    pub era_boundary: bool,
    pub source: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// Batch HOTFIX-6: the widest span one canonical book's own narrative
/// NARRATES (never the span it was written in) -- `data/curated/
/// book-narration-windows.toml`'s own header has the full design rationale
/// and the recounting-witness mechanism (`atlas_core::chronology`) this
/// window check is paired with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookNarrationWindow {
    /// Canonical 3-letter book code (`canon::BOOKS`), e.g. `"GEN"`.
    pub book: String,
    pub from_year: Year,
    pub to_year: Year,
    #[serde(default)]
    pub note: Option<String>,
}

/// Batch E (time-accurate places): one curated period name for a place,
/// e.g. "Luz" for `bethel-1` before Jacob's naming. `when` is the window
/// this name applies for; `verses` are canonical refs SUPPORTING the name
/// claim (etl-validated to exist in the compiled KJV text, same trust class
/// `Event.verses`/`Place.verse_links` already have).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceNameEntry {
    pub name: String,
    pub when: TimeRange,
    pub verses: Vec<String>,
}

/// One curated hover blurb for a place, active over `when`. `breadth` is
/// `"era"` (a period-specific blurb) or `"broad"` (a whole-sweep summary
/// shown when the selected window spans more than one of this place's own
/// `"era"` ranges) -- validated against that two-value enum by
/// `atlas-etl::validate` (same "plain String, checked at ETL time" pattern
/// [`Landmark::kind`] already uses, for the same friendlier-error reason).
/// No `verses` field -- blurb text is curator-written prose (ours/CC0), not
/// a claim keyed to a specific verse the way names/established/destroyed are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceBlurbEntry {
    pub text: String,
    pub when: TimeRange,
    pub breadth: String,
}

/// A biblically-supported established/destroyed date claim. `when` is
/// either a single year (`from_year == to_year`, the curated TOML's `year =
/// ...` shorthand) or a genuine range (`from = ...` / `to = ...`) --
/// collapsed to one `TimeRange` here since the client's own `YearText.
/// FormatRange` already renders an equal-endpoints range as a single year,
/// so no separate "was this a year or a range" flag is needed on the wire.
/// `note` is a short qualifier (e.g. `"traditional"`) the client renders as
/// a leading "c." on the date when present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceDateClaim {
    pub when: TimeRange,
    pub verses: Vec<String>,
    pub note: Option<String>,
}

/// One place's whole curated history record (`data/curated/place-history.toml`,
/// compiled to `place-history.json`). `id` matches a real compiled `Place.id`
/// (etl-validated -- an unknown id hard-fails, same trust class as
/// `Event.places`). Not every place has one of these; `AtlasData::place_history`
/// only holds entries for the ~15-25 places this batch actually curated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceHistory {
    pub id: String,
    #[serde(default)]
    pub names: Vec<PlaceNameEntry>,
    #[serde(default)]
    pub blurbs: Vec<PlaceBlurbEntry>,
    pub established: Option<PlaceDateClaim>,
    pub destroyed: Option<PlaceDateClaim>,
}

/// Batch E3 (KJV display-name alias layer -- owner bug report 2026-08-20:
/// "there are two locations, cush and gihon, that are both lit up on
/// genesis 2 even though cush isn't mentioned in gen 2:13"). ONE curated
/// translation-keyed display-name override for a place whose plain default
/// `Place.name` (Theographic's own canonical/modern name) is NOT the name
/// the KJV text actually uses -- e.g. `cush-2` -> "Ethiopia" (GEN.2.13).
/// Distinct axis from `PlaceHistory::names` (Batch E's period-accurate,
/// TIME-windowed renames like Luz->Bethel): an alias is a single, flat,
/// translation-keyed fact independent of any calendar window -- it is the
/// FALLBACK `resolve_display_name` reaches for whenever no curated period
/// name is active (including scripture mode, which has no window at all),
/// never a competitor to an active period name. `translations` mirrors
/// `EventWitness::translations`'s exact shape/philosophy (`crate::translation`'s
/// own doc comment) -- `"kjv"` (`crate::translation::DEFAULT_TRANSLATION`)
/// is the only key any curated alias carries today; the shape survives a
/// future translation without restructuring. `verses` are the supporting
/// citation(s) a curator checked this alias against (etl-validated: must
/// parse and exist in the compiled KJV text) -- translation-agnostic (a
/// canonical verse id names the same verse regardless of which
/// translation's text is being read).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceNameAlias {
    pub id: String,
    pub translations: HashMap<String, String>,
    pub verses: Vec<String>,
}

/// Batch B2 ("borders v2, the cartographer's edition" -- supersedes the
/// snapshot-year GeoJSON model Batch B/C2 shipped, `AtlasData::borders` +
/// `nearest_border_year` below, both now deleted). One hand-authored
/// timerange era of ONE polity: `name` is time-accurate (may differ across a
/// single polity's own eras -- "Egypt" then later "Ptolemaic Egypt" is the
/// SAME `Polity::id`, so it keeps one color across the rename, see
/// `Polity::color_key`'s own comment), `from`/`to` are the (validated
/// non-overlapping-within-one-polity, see `atlas_etl::validate::run_polities`)
/// signed-year window this era's shape is drawn for, `ref_note` names only
/// the public-domain/general-knowledge source actually consulted while
/// drawing THIS era's rings (citation-integrity rule -- never a source that
/// wasn't opened), and `rings` is one or more CLOSED (first point repeats as
/// the last) simple polygon rings of `(lat, lon)` pairs -- deliberately
/// `[lat, lon]`, NOT GeoJSON's own `[lon, lat]` convention, matching how
/// every other coordinate pair in this app's curated data reads (`Place.lat`/
/// `Place.lon`, `Landmark.lat`/`Landmark.lon`) and how Leaflet's own
/// `L.latLng`/polygon APIs take points -- one less silent-transpose bug for a
/// curator hand-typing coordinates. Field names are `from`/`to` (not
/// `from_year`/`to_year`, unlike `Era`) and stay flat (no nested `TimeRange`)
/// on purpose: the batch brief's own TOML example and wire contract both
/// name them `from`/`to` verbatim, and keeping the curated TOML, the compiled
/// JSON, and the wire response byte-identical in shape means zero rename
/// mapping anywhere in the pipeline -- "easily reversible/modifiable"
/// (the user's own words) starts with the data staying exactly what it looks
/// like on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolityEra {
    pub name: String,
    pub from: Year,
    pub to: Year,
    pub ref_note: String,
    pub rings: Vec<Vec<(f64, f64)>>,
    /// Batch M requirement 1 ("the DAG grows a node type: every era-boundary
    /// delta inside a polity becomes a first-class DELTA with a Scripture-
    /// mapped event"): the delta AT THIS ERA'S OWN START -- the change FROM
    /// the previous era of the same [`Polity`], or, for a polity's very
    /// first era, its own rise. Curated as a nested `[era.transition]` table
    /// under this era's own `[[era]]` (TOML's standard "subtable of the most
    /// recently opened array-of-tables element" shape -- no new top-level
    /// array, no id-matching needed). `None` is a HONEST, curator-visible
    /// choice, not a gap to paper over: "an uneventful boundary stays
    /// visible but gets the minimal popover" (the batch brief, verbatim) --
    /// see `atlas_etl::validate::run_polities`'s own delta checks for what
    /// "present" must satisfy (non-empty event/ref_note, every verse
    /// canonical AND real) and this crate's own `PolityDelta` doc comment
    /// for the citation-integrity rule governing what's ALLOWED to be
    /// `Some`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<PolityDelta>,
    /// Batch M requirement 1: the delta at this era's OWN END -- ONLY
    /// meaningful (and only ever curated) on a polity's FINAL era, i.e. the
    /// polity's own end/absorption/destruction. Curated as a nested
    /// `[era.fall]` table, same mechanism as `transition` above. A polity
    /// this app's own curated span (`[-4004,100]`) simply outlives (Rome,
    /// Parthia, ...) legitimately has no `fall` on its last era -- absence
    /// here does not imply an authoring gap the way an internal boundary's
    /// missing `transition` might.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fall: Option<PolityDelta>,
}

/// Batch M requirement 1: one Scripture-mapped historical delta -- the
/// event AT an era boundary (a polity's rise, an internal border/regime
/// change, or its fall), curated as `[era.transition]`/`[era.fall]` nested
/// under one `[[era]]` table (see `PolityEra::transition`/`PolityEra::fall`'s
/// own doc comments). Shape is deliberately flat and small, mirroring
/// `CatechismItem.ref_note`'s own "cite only what you actually consulted"
/// discipline:
/// - `event`: a short, sentence-cased prose description (e.g. "Assyria
///   carries Israel captive") -- house prose, never a bare citation.
/// - `verses`: individually-canonical verse refs (the SAME flat,
///   already-canonical-string convention `CatechismItem::verses`/
///   `Event::verses` already use -- no ranges, no re-expansion needed), each
///   independently explorable once on the wire (POLITY-DELTA's own THE
///   SCRIPTURES section, rendered through the shared passage-list
///   component). MAY be empty (an event grounded only in Church-traditional
///   history, with no single verse pinpointing it) -- the SCRIPTURES section
///   is then simply absent (conditional presence), never a fabricated ref.
/// - `ref_note`: names only the source(s) actually consulted -- 1911
///   Britannica (already the era ring's own grounding source in most
///   curated files), a specific scripture passage, or an explicit
///   "tradition only" disclosure when that's the honest state of the
///   evidence. Never invented, per this project's standing citation-
///   integrity rule (6+ prior incidents) -- see the batch report's own
///   delta-coverage table for exactly which boundaries got one and why the
///   rest were honestly omitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolityDelta {
    pub event: String,
    #[serde(default)]
    pub verses: Vec<String>,
    pub ref_note: String,
    /// Fix round 1 (I1): a curator-authored ECHO of the `from` year of the
    /// era this delta block is meant to belong to -- required precisely
    /// because TOML's array-of-tables rule silently attaches a nested
    /// `[era.transition]`/`[era.fall]` table to whichever `[[era]]` element
    /// was MOST RECENTLY OPENED, not whichever era a curator's own
    /// surrounding prose comment describes. That exact mismatch shipped
    /// live once (7 of this batch's own original 22 deltas, self-caught
    /// only via a live diagnostic, not this field) before this field
    /// existed. `atlas_etl::validate::run_polities` cross-checks this
    /// against the ACTUAL hosting era's own `from` -- a curator who writes
    /// the block in the wrong place now gets a loud, specific ETL error
    /// instead of a silently-misattached delta. Never read by the client
    /// (no wire/`PolityDeltaOut` field carries it) -- purely an authoring
    /// safety net.
    pub for_era_from: Year,
}

/// Batch R requirement 1 ("borders become part of the plate"): ONE named
/// region of the curated land mask (`data/curated/land-mask.toml`) -- used
/// ONLY to clip polity washes (map.js's `BorderLayer`) so they never spill
/// into open sea, never rendered as its own visible layer. `rings` reuses
/// `PolityEra::rings`'s own shape/convention exactly (`[lat, lon]`, one or
/// more closed simple polygons; a curator's own comment lives on `ref_note`,
/// same "cite what you actually consulted" discipline every other curated
/// geometry file in this app already follows). No time dimension (a land
/// mask doesn't change era to era) and no color -- this is geometry only,
/// consumed purely as a clip path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandMaskRegion {
    pub name: String,
    pub ref_note: String,
    pub rings: Vec<Vec<(f64, f64)>>,
}

/// Batch F ("the small catechism"): one item of one chief part of Luther's
/// Small Catechism (`data/curated/catechism.toml`, compiled to
/// `catechism.json`) -- id/name/text/explanation/where_written/verses per
/// the batch brief's own schema verbatim, PLUS `explanation_heading` and
/// `ref_note`, both well-justified additions (see `catechism.toml`'s own
/// header comment for the full reasoning, and this batch's report for the
/// short version):
/// - `text` is `Option<String>` -- present only for items whose own
///   commandment/creed-article/petition WORDING is a distinct prompt
///   separate from its "what does this mean" gloss (every Ten-Commandments/
///   Creed/Lord's-Prayer item); Baptism/Confession/Sacrament-of-the-Altar
///   items pose and answer their own bespoke question directly with no
///   separate prompt to quote first, so `text` is simply `None` there --
///   conditional presence, matching this app's own "no content, no
///   section" rule throughout, rather than a padded or duplicated value.
/// - `explanation_heading` is Luther's own VERBATIM heading for the
///   explanation that follows -- "What does this mean?" for the
///   overwhelming majority of items (commandments/creed articles/Lord's
///   Prayer petitions), but a distinct, real, bespoke question for
///   Baptism/Confession/Sacrament-of-the-Altar items (e.g. "What does
///   Baptism give or profit?"), where hardcoding the generic phrase would
///   have silently misquoted Luther's own actual wording at those specific
///   spots. Defaults to "What does this mean?" (`default_explanation_heading`)
///   so the curated TOML only spells it out where it actually differs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatechismItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default = "default_explanation_heading")]
    pub explanation_heading: String,
    pub explanation: String,
    #[serde(default)]
    pub where_written: Option<String>,
    #[serde(default)]
    pub verses: Vec<String>,
    #[serde(default)]
    pub ref_note: Option<String>,
    /// Batch F2 ("the user's own catechism verse mapping"): QUESTION-level
    /// citations -- distinct from `verses` above (Luther's OWN embedded
    /// citations, item-level, no question context) -- sourced from the
    /// brain-fuel/catechism repo's own per-topic YAML (question titles +
    /// refs) and, for the Ten Commandments, this project's own Deuteronomy 5
    /// parallel supplement (`data/curated/catechism-deut5.toml`, requirement
    /// 5b). Kept as a SEPARATE list rather than merged into `verses` -- see
    /// this batch's own report for why ("items keep their F-batch
    /// embedded-citation links too," requirement 3 verbatim -- two distinct
    /// granularities, not a parallel implementation of the same concept).
    /// `#[serde(default)]` so every catechism.json written before this batch
    /// (and any test fixture that doesn't care about it) keeps deserializing
    /// with an empty list, no schema migration needed.
    #[serde(default)]
    pub questions: Vec<CatechismQuestion>,
}

fn default_explanation_heading() -> String {
    "What does this mean?".to_string()
}

/// Batch F2: one QUESTION-level catechism citation, attached to a
/// `CatechismItem` via `CatechismItem::questions`. `verses` is a flat list
/// of individually-canonical verse refs (never a range string) -- the SAME
/// convention `CatechismItem::verses`/`curated::expand_verse_ref` already
/// use, produced here by `atlas_etl::catechism_map::canonicalize_ref`.
/// `source` is a plain provenance tag (`"brain-fuel/catechism"` or
/// `"deut5-parallel"` today) -- requirement 5b's own "keep it separate...
/// distinct source field so provenance stays clean," kept on the WIRE too
/// (not curator-only documentation) so a future UI could distinguish them
/// if ever useful, though this batch's own UI does not (see
/// `catechism-section-heading`/`popover-catechism-verse` -- the client
/// renders every question's own verses uniformly regardless of source).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatechismQuestion {
    pub title: String,
    pub verses: Vec<String>,
    pub source: String,
}

/// Batch F: one chief part of the Small Catechism (`[[part]]` in
/// `data/curated/catechism.toml`) -- id/title/items, the curated TOML's own
/// nesting kept all the way to the compiled/wire shape (same "keep the
/// curated shape the compiled shape" reasoning `Polity`/`PolityEra` already
/// follow for polities).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatechismPart {
    pub id: String,
    pub title: String,
    pub items: Vec<CatechismItem>,
}

/// One hand-authored polity, `data/curated/polities/{id}.toml` (one file per
/// polity, "easily reversible/modifiable... I'm expecting you to get this
/// wrong" per the user's own direction -- a wrong shape is ONE file, ONE
/// `[[era]]` table, plain coordinate lists, nothing derived). `color_key` is
/// computed ONCE at ETL compile time -- based on `id` alone, never the era
/// `name`, so a polity keeps its plate tint across a rename, e.g. Egypt
/// staying the same hue through "Ptolemaic Egypt"; this REFINES Batch C2's
/// own name-hash, which kept re-deriving the tint from whatever single name
/// a snapshot-year feature happened to carry that year -- an index into
/// map.js's own `POLITY_TINTS` palette (16 tints as of fix round 1, M1; 8
/// through Batch C2/B2). Fix round 1 (M1): assignment is no longer a plain
/// per-id hash computed by `curated::parse_polity` in isolation (that
/// collided too often -- 11 of the original batch's 14 polities shared a
/// hue) -- `curated::parse_polity` now leaves this field provisional (`0`),
/// and `atlas_etl::main::process_polities` overwrites every polity's real,
/// collision-free value in one pass, via `atlas_etl::polities::
/// assign_color_keys`, once the full sorted roster is available (see that
/// function's own doc comment for the linear-probing algorithm). Stored
/// here either way (not recomputed per-request); `/api/polities` just
/// copies it onto every emitted era row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polity {
    pub id: String,
    pub color_key: u8,
    pub eras: Vec<PolityEra>,
}

/// The whole compiled atlas: ETL builds one of these and calls `.finish()`
/// before writing it to disk; the server deserializes the file and calls
/// `.finish()` again to rebuild the derived indexes (they are `#[serde(skip)]`
/// and therefore empty immediately after deserialization).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AtlasData {
    pub canon: Canon,
    pub places: Vec<Place>,
    pub events: Vec<Event>,
    pub narratives: Vec<Narrative>,
    pub eras: Vec<Era>,
    pub books_meta: Vec<BookMeta>,
    pub verses: HashMap<String, String>,
    pub cross_refs: HashMap<String, Vec<CrossRef>>,

    /// Batch B2: hand-authored per-polity timerange borders (supersedes the
    /// snapshot-year GeoJSON model -- see `Polity`'s own doc comment).
    /// `#[serde(skip)]` because -- like `landmarks`/`place_history` below --
    /// this isn't populated via the struct-level derive: `AtlasData::load`
    /// fills it in from its own `polities.json`, a plain `Vec<Polity>` on
    /// disk (one entry per curated `data/curated/polities/{id}.toml` file).
    /// `demo_fixture()` leaves this empty (no server test needs real polity
    /// geometry); real data always has the full curated roster once
    /// atlas-etl has run.
    #[serde(skip)]
    pub polities: Vec<Polity>,
    /// Curated landmark labels (rivers/seas/mountains/regions). Same
    /// `#[serde(skip)]`-plus-bespoke-`load()` treatment as `borders` above,
    /// for the same reason (loaded from its own `landmarks.json`, not part
    /// of the original eight-file/eight-field pattern). `demo_fixture()`
    /// leaves this empty too.
    #[serde(skip)]
    pub landmarks: Vec<Landmark>,
    /// Batch E: curated place histories (period names, blurbs, established/
    /// destroyed dates), keyed by place id. Same `#[serde(skip)]`-plus-
    /// bespoke-`load()` treatment as `borders`/`landmarks` above -- loaded
    /// from its own `place-history.json` (a plain `Vec<PlaceHistory>` on
    /// disk, matching every sibling compiled file's array shape), indexed
    /// into a map here since every lookup site wants it by place id.
    /// `demo_fixture()` leaves this empty too (no scene test depends on it).
    #[serde(skip)]
    pub place_history: HashMap<String, PlaceHistory>,

    /// Batch E3: curated KJV display-name aliases, keyed by place id. Same
    /// `#[serde(skip)]`-plus-bespoke-`load()` treatment as `place_history`
    /// immediately above -- loaded from its own `place-names-kjv.json` (a
    /// plain `Vec<PlaceNameAlias>` on disk), indexed into a map here since
    /// every lookup site (`scene`/`handlers`) wants it by place id, same
    /// pattern. `demo_fixture()` leaves this empty too (no scene test
    /// depends on it).
    #[serde(skip)]
    pub place_name_aliases: HashMap<String, PlaceNameAlias>,

    /// Batch R requirement 1: the curated land mask (`data/curated/land-mask.toml`,
    /// compiled to `land-mask.json`) -- every region's own rings, flattened
    /// (region names/ref_notes are curator documentation only; the wire and
    /// the client never need them, only the raw ring geometry to clip
    /// against). Same `#[serde(skip)]`-plus-bespoke-`load()` treatment as
    /// `polities`/`landmarks`/`place_history` above. `demo_fixture()` leaves
    /// this empty (no server test needs real coastline geometry).
    #[serde(skip)]
    pub land_mask: Vec<Vec<(f64, f64)>>,

    /// Batch F ("the small catechism"): curated catechism parts/items
    /// (`data/curated/catechism.toml`, compiled to `catechism.json`), kept
    /// in their own curated PART nesting (not flattened) -- same
    /// `#[serde(skip)]`-plus-bespoke-`load()` treatment as `polities`/
    /// `landmarks`/`place_history`/`land_mask` above. `demo_fixture()`
    /// carries a small real entry (see below) so atlas-server's own
    /// integration tests can exercise the real verse<->catechism pipeline
    /// end to end, not just atlas_core's pure functions in isolation.
    #[serde(skip)]
    pub catechism: Vec<CatechismPart>,

    /// Batch HOTFIX-6 (graph-wide chronology audit): the curated chronology
    /// anchor table (`data/curated/chronology-anchors.toml`, compiled to
    /// `chronology-anchors.json`) -- same `#[serde(skip)]`-plus-bespoke-
    /// `load()` treatment as `polities`/`landmarks`/`place_history`/
    /// `land_mask`/`catechism` above. `demo_fixture()` leaves this empty (no
    /// scene/narrative-fixture test needs real anchor rows); the Amendment E
    /// property tests (E1/E3/E4) load it via `load_real_compiled_data()`,
    /// the SAME real-compiled-data helper the rest of `narrative.rs`'s own
    /// test suite already uses -- one compiled artifact, read by both the
    /// ETL-time validator and these tests, per the controller's own "single
    /// source, two enforcement layers" instruction.
    #[serde(skip)]
    pub chronology_anchors: Vec<ChronologyAnchor>,
    /// Batch HOTFIX-6: the curated per-book narration windows
    /// (`data/curated/book-narration-windows.toml`, compiled to
    /// `book-narration-windows.json`). Same `#[serde(skip)]`-plus-bespoke-
    /// `load()` treatment as `chronology_anchors` immediately above.
    #[serde(skip)]
    pub book_narration_windows: Vec<BookNarrationWindow>,

    /// Derived: place id -> index into `places`. Built by `finish()`.
    #[serde(skip)]
    place_index: HashMap<String, usize>,
    /// Derived: event id -> index into `events`. Built by `finish()`.
    #[serde(skip)]
    event_index: HashMap<String, usize>,
    /// Derived: canonical verse id -> event ids that reference it. Built by `finish()`.
    #[serde(skip)]
    verse_to_events: HashMap<String, Vec<String>>,
    /// Batch R requirement 5 (place-in-verse hover -> marker blink): derived,
    /// canonical verse id -> ids of every place whose OWN `verse_links`
    /// names it -- the reverse of `Place::verse_links` (which lists, per
    /// PLACE, every verse the geocoder attached to it). Built by `finish()`
    /// in one pass over `places`, same shape as `verse_to_events` just above.
    /// `handlers::chapter` uses this to populate each `VerseOut.places` --
    /// the client-side seam (a mini-reader rendering a chapter) needs to
    /// know, per verse, which curated places to offer as hoverable mentions;
    /// this is the ONLY data source for that (no separate "NER offsets"
    /// exist in this app -- the client itself does the plain-text substring
    /// match against each returned place's own name, see World.razor's/the
    /// mini-reader's own comment).
    #[serde(skip)]
    verse_to_places: HashMap<String, Vec<String>>,
    /// Batch E2 (the ever-present graph): "cities in our graph" per the
    /// user's own direction quoted in batch-e2-brief.md -- ids of every
    /// place touched by >=1 event, ANY window (206 in the real compiled
    /// data, but NEVER hardcoded -- always derived fresh from `events` by
    /// `finish()`, so a future data change is picked up automatically).
    /// `scene::quiet_places` subtracts a window's own lit set from this to
    /// get that window's quiet places.
    #[serde(skip)]
    event_bearing_place_ids: HashSet<String>,
    /// Batch E2: a place's ALL-TIME event count (every event touching it,
    /// in ANY window, not just the scene's own) -- the source of
    /// `QuietPlace::total_events`. Built alongside `event_bearing_place_ids`
    /// by `finish()` in the same single pass over `events`.
    #[serde(skip)]
    event_counts_by_place: HashMap<String, u32>,

    /// Batch F, extended Batch F2: derived, canonical verse id -> ordered
    /// `(item_id, question_title)` hits -- built in one pass over
    /// `catechism` by `finish()`, same shape as `verse_to_events`/
    /// `verse_to_places` above, but a PAIR per hit rather than a bare id:
    /// `question_title` is `None` for a hit from `CatechismItem::verses`
    /// (Luther's own item-level embedded citation, Batch F, unchanged) and
    /// `Some(title)` for a hit from one of `CatechismItem::questions[]`
    /// (Batch F2's own question-level citations). `catechism_items_for_span`
    /// (and, through it, `handlers::verse`'s own `catechism` field and
    /// `handlers::catechism_for_span`) is the only consumer.
    #[serde(skip)]
    verse_to_catechism: HashMap<String, Vec<(String, Option<String>)>>,
    /// Batch F: derived, catechism item id -> its own display `name` --
    /// built alongside `verse_to_catechism` by `finish()`, so resolving a
    /// cited item's display name never needs a second pass over `catechism`.
    #[serde(skip)]
    catechism_item_names: HashMap<String, String>,
    /// Batch F: derived, catechism item id -> (part index, item index) --
    /// built alongside `verse_to_catechism` by `finish()`; `catechism_item_by_id`
    /// is the one lookup that needs this (item fetch by id, requirement 3).
    #[serde(skip)]
    catechism_item_index: HashMap<String, (usize, usize)>,

    /// Batch T requirement 5: derived, canonical verse id -> the pericope
    /// heading (if any) that should render immediately above THAT verse in
    /// the reading flow. Built by `finish()` in one pass over `events` --
    /// see `heading_for_verse`'s own doc comment for exactly which events
    /// qualify ("heading-worthy") and how each one's own anchor verse is
    /// chosen (one entry per WITNESS, via `scene::witnesses_for`, so a
    /// multi-witness event gets one heading per Gospel book, each at that
    /// witness's own first verse -- never just one heading in one book).
    #[serde(skip)]
    verse_heading: HashMap<String, HeadingEntry>,

    /// Batch T2 (owner's own "within-layer anchor collision is a curation
    /// error your validation must catch" ruling): every case where TWO
    /// DIFFERENT real (layer-1 -- curated `witnesses` non-empty and/or
    /// `robertson_section` present) containers anchor the identical verse
    /// -- `(anchor_verse, first_event_id, second_event_id)`. Distinct from
    /// `verse_heading`'s own collision RESOLUTION (which decisively picks
    /// a winner for DISPLAY via `heading_precedence`, and is fine with a
    /// layer-1-vs-layer-0 collision, e.g. `pw_bethany`/`jm_bethany`): two
    /// real, independently-curated containers should never both claim the
    /// exact same anchor point in the first place -- curated sectioning
    /// (Robertson or otherwise) is supposed to PARTITION, not overlap, at
    /// its own boundary. Built alongside `verse_heading` in the SAME
    /// `finish()` pass (one iteration over `events`, not a second derived
    /// pass); `atlas_etl::validate::run` turns each entry here into a
    /// hard, fail-loud ETL error via `heading_anchor_collisions()` below.
    #[serde(skip)]
    heading_anchor_collisions: Vec<(String, String, String)>,

    /// Batch HOTFIX-4 requirement 1 ("whole-DAG chronological traversal --
    /// every dated event, not just narrative members"): every `kind ==
    /// "event"` (dated) event's own id, in GLOBAL CHRONOLOGICAL ORDER --
    /// the owner's own law verbatim, "the previous/next event is the one
    /// that is chronologically NEXT." Built by `finish()` via a STABLE sort
    /// by `(when.from_year, order_key)` ascending -- the SAME tuple
    /// `heading_precedence`'s own chronology tier and the narrative-leg
    /// validator both already use ("the T ordering"), `std::cmp::Reverse`
    /// nowhere near this (ascending needs no reversal at all -- the
    /// overflow bug fix-round-1 found was specific to a DESCENDING
    /// max-tuple comparison, not relevant here). Same-`(from_year,
    /// order_key)` runs are genuinely common (137 real groups in the
    /// compiled data -- `order_key` defaults to 0 outside Passion Week's
    /// own deliberate sub-sequencing) -- a STABLE sort resolves them by
    /// original array order, the IDENTICAL "equal on all explicit tiers
    /// keeps first-wins" precedent `heading_precedence` already establishes
    /// and CONTRACT.md already documents, not a new invented rule.
    /// General-kind events are excluded entirely (requirement 2: "NOT part
    /// of time traversal... fabricating one is forbidden" -- they have no
    /// real date to sort by). Built AFTER `event_merge::apply_event_merges`
    /// (this function's own first step) has already removed every
    /// duplicate identity, so the timeline never interleaves two scales for
    /// the same real-world event.
    #[serde(skip)]
    timeline_order: Vec<String>,
    /// Derived: event id -> index into `timeline_order`. Built alongside it.
    #[serde(skip)]
    timeline_index: HashMap<String, usize>,
}

/// Batch T requirement 5: one resolved pericope heading -- an EVENT-kind
/// PASSAGE's own id and title, anchored at one specific verse (the first
/// verse of one of its own witnesses). See `AtlasData::heading_for_verse`.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadingEntry {
    pub event_id: String,
    pub title: String,
    /// Batch HOTFIX-4 requirement 6 (AFFORDANCE HONESTY, owner's own law:
    /// "if something isn't traversable it shouldn't look like other things
    /// that are actually traversable"): this heading's own container's
    /// `Event::kind` ("event" | "general") -- carried so the CLIENT can
    /// tell, before ever clicking through, whether this heading leads to a
    /// dated (traversable) or general-kind (NOT part of time traversal,
    /// requirement 2) container, without a second fetch just to find out.
    pub kind: String,
}

/// Batch T: one anchor verse per witness of `e` -- GRACEFUL (never panics)
/// equivalent of `scene::witnesses_for`'s own "first verse of each witness"
/// concept, used ONLY by `AtlasData::finish()`'s own heading-index build
/// (see that call site's own comment for why gracefulness matters there
/// specifically). An unparseable verse is simply skipped, never a reason to
/// crash the whole ETL binary -- `atlas_etl::validate` is what turns a bad
/// witness verse into a loud, aggregated, curator-facing error; this
/// function's only job is to not stand in its way by panicking first.
fn heading_anchors_for(e: &Event) -> Vec<String> {
    if !e.witnesses.is_empty() {
        return e
            .witnesses
            .iter()
            .filter_map(|w| w.translations.get(crate::translation::DEFAULT_TRANSLATION))
            .filter_map(|verses| verses.iter().find(|v| crate::refs::VerseId::parse_canonical(v).is_ok()))
            .cloned()
            .collect();
    }

    // No explicit witnesses -- one anchor per book actually touched by
    // `e.verses`, first-seen-book order (mirrors `scene::witnesses_for`'s
    // own synthesis, minus the grouping/sorting this narrower need doesn't
    // require -- just each book's own FIRST verse in curated/imported order).
    let mut seen_books: Vec<String> = Vec::new();
    let mut anchors: Vec<String> = Vec::new();
    for v in &e.verses {
        let Ok(vid) = crate::refs::VerseId::parse_canonical(v) else { continue };
        let book = vid.book.code().to_string();
        if !seen_books.contains(&book) {
            seen_books.push(book);
            anchors.push(v.clone());
        }
    }
    anchors
}

/// Batch T fix-round-1 (Important-1, batch-t-review.md), amended by the
/// owner's own 2026-08-21 "decisive container" ruling: the collision
/// precedence `AtlasData::finish()`'s own `verse_heading` build uses when
/// two heading-worthy events anchor the identical verse -- see that call
/// site's own comment for the full rule and rationale. A 4-tuple, compared
/// lexicographically (Rust's own derived tuple `Ord`); a STRICTLY GREATER
/// tuple wins a collision, an EQUAL tuple keeps first-wins (unchanged from
/// before this fix -- not expected to ever actually occur for two distinct
/// real events, only reachable if every tier below ties exactly):
///
/// 1. LAYER (owner's own words: "T's existing-title freebie rule vs a real
///    container") -- `1` when `e` is a genuine curated container for THIS
///    grouping (`witnesses` non-empty and/or `robertson_section` present),
///    `0` when `e` is heading-worthy ONLY because it happens to be a
///    narrative leg riding its own pre-existing `Event::label` "freebie."
///    This is the PRIMARY axis and, in every real case in today's curated
///    data, decides the collision outright (e.g. `pw_bethany` real
///    container vs. `jm_bethany` freebie, both anchoring JHN.12.1).
/// 2. KIND -- `1` for `"event"`, `0` for `"general"`, a tiebreak that only
///    matters "if both colliders are genuinely same-layer containers"
///    (the owner's own words) -- LIVE as of Batch T2 (general-kind
///    passages are now real curated data), though still rare in practice:
///    curated sections are expected to PARTITION (see
///    `heading_anchor_collisions`), so two same-layer containers colliding
///    at all is uncommon; when it does happen (e.g. a general-kind
///    "preface" container and a spine-narrative freebie both touching one
///    verse -- a layer-0 freebie is not layer-1, so tier 1 already
///    resolves that case) this tier is what remains to decide it.
/// 3. CHRONOLOGY -- the EARLIER `(from_year, order_key)` wins, via
///    `std::cmp::Reverse` (NOT `i32::MAX - year` -- this atlas's own years
///    run deep negative, e.g. -4004 at the Ussher-consistent span floor,
///    and `i32::MAX - (a very negative year)` overflows i32 outright;
///    caught live by a debug-build panic across every fixture-based test
///    the moment this landed, fixed before it could ship) -- the SAME
///    `(from_year, order_key)` tuple `Narrative.legs`' own ETL-validated
///    ordering already uses elsewhere ("narrative-leg order," the owner's
///    own words). Only reached when two REAL containers of the same kind
///    collide, which does not occur anywhere in today's curated data.
fn heading_precedence(e: &Event) -> (u8, u8, std::cmp::Reverse<i32>, std::cmp::Reverse<i32>) {
    // Batch T2: `acts_section` counts the SAME as `robertson_section` here
    // -- Acts's own sibling provenance field (see `Event::acts_section`'s
    // own doc comment for why it's a separate field, not a reused one).
    // Batch W1: `atlas_section` (the whole-Bible sibling of both) counts
    // identically -- see `Event::atlas_section`'s own doc comment.
    // Batch W3: `kjv_superscription` (the KJV's own literal-citation
    // sibling of all three) counts identically -- see
    // `Event::kjv_superscription`'s own doc comment.
    let layer: u8 = if !e.witnesses.is_empty()
        || e.robertson_section.is_some()
        || e.acts_section.is_some()
        || e.atlas_section.is_some()
        || e.kjv_superscription.is_some()
    {
        1
    } else {
        0
    };
    let kind: u8 = if e.kind == "event" { 1 } else { 0 };
    (layer, kind, std::cmp::Reverse(e.when.from_year), std::cmp::Reverse(e.order_key))
}

impl AtlasData {
    /// Builds an `AtlasData` from its eight schema fields, leaving the derived
    /// indexes empty (call `.finish()` to populate them). This is the
    /// ergonomic, one-expression way to construct one from outside this
    /// module: the index fields are private, so a plain struct literal
    /// naming them (including via `..Default::default()`, which still
    /// requires visibility of every field it fills in) cannot be written
    /// from another module. It is not the *only* way — `AtlasData::default()`
    /// followed by per-field assignment on the public schema fields also
    /// works, since those fields are all `pub`, just less conveniently.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        canon: Canon,
        places: Vec<Place>,
        events: Vec<Event>,
        narratives: Vec<Narrative>,
        eras: Vec<Era>,
        books_meta: Vec<BookMeta>,
        verses: HashMap<String, String>,
        cross_refs: HashMap<String, Vec<CrossRef>>,
    ) -> Self {
        Self {
            canon,
            places,
            events,
            narratives,
            eras,
            books_meta,
            verses,
            cross_refs,
            ..Default::default()
        }
    }

    /// Sorts `events` chronologically by `when.from_year` and (re)builds the
    /// derived lookup indexes. Idempotent — safe to call more than once
    /// (e.g. once in ETL before writing, once in the server after reading).
    pub fn finish(mut self) -> Self {
        // Batch HOTFIX-2 (same-place dedupe): applied FIRST, before every
        // derived index below is built from `self.places`/`self.events` --
        // so `place_index`, `event_bearing_place_ids`, `verse_to_places`,
        // etc. all come out correct for free, already reflecting the merged
        // graph, with no separate index-fixup pass needed. See
        // `crate::merge`'s own doc comment for the full reasoning (why this
        // layer, why curated rather than automatic). Idempotent itself (a
        // pair already merged by an earlier `finish()` call is silently
        // skipped), so calling `finish()` twice -- ETL then the server, or
        // the server's own `AtlasData::load().finish()` -- never double-merges.
        crate::merge::apply_place_merges(&mut self.places, &mut self.events);

        // Batch HOTFIX-4 (duplicate event-identity rectification): applied
        // immediately after the place merge and, like it, BEFORE every
        // derived index below is built -- so `event_index`, `verse_to_events`,
        // `verse_heading`, and the global timeline index (below) all come out
        // correct for free, already reflecting the merged graph. See
        // `event_merge`'s own module doc comment for the full root-cause
        // chain and sweep methodology. Idempotent, same contract as
        // `apply_place_merges`.
        crate::event_merge::apply_event_merges(&mut self.events, &mut self.narratives);

        self.events.sort_by_key(|e| e.when.from_year);

        self.place_index = self
            .places
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.clone(), i))
            .collect();
        self.event_index = self
            .events
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.clone(), i))
            .collect();

        // Batch T requirement 3 ("verse popover: event membership"): a
        // REAL, live-caught bug, self-caught via a hanging Playwright test
        // before this shipped -- this reverse index used to be built from
        // `e.verses` ALONE, so a verse cited ONLY by a witness (e.g.
        // MAT.26.6, Matthew's own account of the Bethany anointing --
        // `pw_bethany`'s own top-level `verses` field is JHN.12.1-11 only)
        // never resolved to its own event here, even though the SAME verse
        // correctly anchors a reader heading (`heading_anchors_for`, which
        // already walked witnesses). That split meant a real user reading
        // Matthew 26:6 would see a reader heading naming the event, but the
        // verse's own popover would show NO "EVENT" section for it at all
        // -- confusingly inconsistent, and the exact bug the owner's own
        // requirement 3 exists to fix. Every witness's own verses are now
        // unioned in too, per event, deduped (a verse present in BOTH
        // `e.verses` and a witness must still yield this event id only
        // ONCE, never twice).
        let mut verse_to_events: HashMap<String, Vec<String>> = HashMap::new();
        for e in &self.events {
            let mut seen: HashSet<&str> = HashSet::new();
            for v in e.verses.iter().chain(
                e.witnesses.iter().filter_map(|w| w.translations.get(crate::translation::DEFAULT_TRANSLATION)).flatten(),
            ) {
                if seen.insert(v.as_str()) {
                    verse_to_events.entry(v.clone()).or_default().push(e.id.clone());
                }
            }
        }
        self.verse_to_events = verse_to_events;

        // Batch R requirement 5: one pass over every place's own
        // `verse_links` builds the reverse index -- see `verse_to_places`'s
        // own doc comment.
        let mut verse_to_places: HashMap<String, Vec<String>> = HashMap::new();
        for p in &self.places {
            for v in &p.verse_links {
                verse_to_places.entry(v.clone()).or_default().push(p.id.clone());
            }
        }
        self.verse_to_places = verse_to_places;

        // Batch E2: one pass over every event's `places` builds BOTH the
        // event-bearing id set and each one's all-time event count -- "cities
        // in our graph" is derived here, never hardcoded (see the two
        // fields' own doc comments).
        let mut event_counts_by_place: HashMap<String, u32> = HashMap::new();
        for e in &self.events {
            for pid in &e.places {
                *event_counts_by_place.entry(pid.clone()).or_insert(0) += 1;
            }
        }
        self.event_bearing_place_ids = event_counts_by_place.keys().cloned().collect();
        self.event_counts_by_place = event_counts_by_place;

        // Batch F, extended Batch F2: one pass over every catechism item
        // builds the reverse verse index, the id->name lookup, and the
        // id->(part,item) index together -- see the three fields' own doc
        // comments. Each item contributes TWO kinds of hit to the reverse
        // index: its own item-level `verses` (question `None`, Batch F,
        // unchanged order/behavior) THEN each of its `questions[]`, in
        // curated order, each contributing its OWN verses tagged with its
        // OWN question title (Batch F2) -- so a verse cited both ways for
        // the SAME item (rare, but not assumed impossible) sees the
        // item-level hit first, matching "items keep their F-batch
        // embedded-citation links too" reading naturally as the primary,
        // first-listed source.
        let mut verse_to_catechism: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
        let mut catechism_item_names: HashMap<String, String> = HashMap::new();
        let mut catechism_item_index: HashMap<String, (usize, usize)> = HashMap::new();
        for (pi, part) in self.catechism.iter().enumerate() {
            for (ii, item) in part.items.iter().enumerate() {
                catechism_item_names.insert(item.id.clone(), item.name.clone());
                catechism_item_index.insert(item.id.clone(), (pi, ii));
                for v in &item.verses {
                    verse_to_catechism.entry(v.clone()).or_default().push((item.id.clone(), None));
                }
                for q in &item.questions {
                    for v in &q.verses {
                        verse_to_catechism.entry(v.clone()).or_default().push((item.id.clone(), Some(q.title.clone())));
                    }
                }
            }
        }
        self.verse_to_catechism = verse_to_catechism;
        self.catechism_item_names = catechism_item_names;
        self.catechism_item_index = catechism_item_index;

        // Batch T requirement 5 ("pericope headings render in the reading
        // flow"): one pass over every EVENT-kind CONTAINER builds the
        // anchor-verse index -- see `heading_for_verse`'s own doc comment
        // for the heading-worthy rule AND the collision-precedence rule
        // below. Runs AFTER `self.events.sort_by_key` above.
        //
        // DECISIVE-CONTAINER MODEL (owner's own 2026-08-21 ruling, verbatim:
        // "we don't modify the verses because we lose composability... the
        // Bible data structure fundamentally changes where these titled
        // passages are containers for verses... identity is empty set. the
        // set may have n verses. there may be sets that have overlapping
        // verses as we grow the dag, which is fine, but we're decisive
        // about the titles of verse groupings that we display on the
        // reader"). `Event` IS a titled CONTAINER over a verse set, never
        // the other way around -- this index is built by ITERATING
        // containers and emitting anchors from their own content
        // (`heading_anchors_for`), never by writing heading data onto a
        // verse record (there is no such record to write onto -- verses
        // stay plain, immutable atoms; `verses-kjv.json` is untouched by
        // this or anything else in Batch T). Two containers legitimately
        // overlapping on the same verse is EXPECTED, not an error, as the
        // graph of containers grows (a real, live case already exists:
        // `jm_bethany`/`pw_bethany` both anchor JHN.12.1) -- overlap lives
        // in the DATA. The READER is what must be decisive: exactly one
        // title heads any displayed verse grouping, chosen by the
        // deterministic, principled precedence rule below
        // (`heading_precedence`), never by incidental iteration/file
        // order. The non-chosen container is NEVER unreachable -- a
        // verse's own EVENT membership section (`events_for_verse`, above)
        // lists every container touching that verse regardless of which
        // one won the single heading slot, so it stays one click away at
        // the identical verse.
        //
        // Fix-round-1 (Important-1, batch-t-review.md, then amended by the
        // ruling above): the OLD code here used plain first-wins
        // `entry().or_insert_with()`, with a comment claiming it
        // "deterministically keeps the chronologically EARLIER event's own
        // heading" -- never actually true for a same-year tie, since
        // `self.events.sort_by_key` sorts by `from_year` ALONE, so two
        // same-year events keep their pre-sort/file order under Rust's
        // stable sort. Live-reproduced by the review: `jm_bethany` (a bare
        // `jesus-ministry` leg, no witnesses/robertson_section -- heading-
        // worthy ONLY via the "existing-title freebie" narrative-leg rule)
        // sat earlier in `events-extra.toml` than `pw_bethany` (this
        // batch's own flagship 3-witness, Robertson-grounded passion-week
        // leg -- a REAL curated container for this exact grouping), so
        // `jm_bethany` silently won JHN.12.1's heading by bare file
        // position, dropping `pw_bethany`'s own John-witness heading
        // entirely -- no warning, no ETL error, no test catching it (every
        // existing heading test happened to exercise a collision-free
        // event). See `heading_collision_tests` below for the pinned
        // regression, and CONTRACT.md's own HEADING-WORTHY RULE paragraph
        // (kept in lockstep -- if `heading_precedence` ever changes, that
        // paragraph must change in the SAME commit).
        //
        // Every verse in the Bible ultimately belonging to at least one
        // titled container is the owner's own stated end-state; today only
        // Gospels+Acts+the-13-curated-narratives are covered (this batch's
        // own scoped coverage decision) -- the total migration is real
        // future work, not this fix.
        //
        // Deliberately does NOT call `scene::witnesses_for` here (a real,
        // live-caught bug, self-caught via TDD before this shipped): that
        // function -- like every other "verse groups on the wire" helper in
        // `scene.rs` -- calls `VerseId::parse_canonical(v).expect(..)`, an
        // invariant that holds everywhere else in this codebase because
        // every OTHER caller only ever runs on ALREADY-VALIDATED compiled
        // data (server-side, post-ETL). `finish()` is not that -- ETL's own
        // `main.rs` calls `.finish()` BEFORE `validate::run`, specifically
        // so validation can inspect the fully-derived `AtlasData`, which
        // means a curator's own malformed witness verse would reach this
        // loop BEFORE validation ever gets a chance to reject it loudly and
        // reports it as a clean, aggregated error -- panicking here instead
        // would crash the whole ETL binary on a single bad ref. `heading_anchors_for`
        // below is a narrower, GRACEFUL equivalent (skip, never panic) --
        // exactly the same "aggregate every violation, never fail fast on
        // curator input" discipline `atlas_etl::validate` itself follows.
        let narrative_leg_ids: HashSet<&str> =
            self.narratives.iter().flat_map(|n| n.legs.iter().map(|s| s.as_str())).collect();
        let mut verse_heading: HashMap<String, HeadingEntry> = HashMap::new();
        let mut verse_heading_precedence: HashMap<String, (u8, u8, std::cmp::Reverse<i32>, std::cmp::Reverse<i32>)> = HashMap::new();
        // Batch T2: tracks the FIRST real (layer-1) container to claim each
        // anchor, purely to detect a SECOND, different real container
        // claiming the identical anchor -- see `heading_anchor_collisions`'s
        // own doc comment for why this is a separate, additional concern
        // from `verse_heading_precedence`'s own decisive-winner resolution
        // above (that one is fine with a layer-1-vs-layer-0 collision; this
        // one only ever fires layer-1-vs-layer-1).
        let mut real_anchor_owner: HashMap<String, String> = HashMap::new();
        let mut heading_anchor_collisions: Vec<(String, String, String)> = Vec::new();
        for e in &self.events {
            let heading_worthy = narrative_leg_ids.contains(e.id.as_str())
                || !e.witnesses.is_empty()
                || e.robertson_section.is_some()
                || e.acts_section.is_some() // Batch T2: Acts's own sibling provenance field
                || e.atlas_section.is_some() // Batch W1: the whole-Bible sibling of both
                || e.kjv_superscription.is_some(); // Batch W3: the KJV's own literal-citation sibling
            if !heading_worthy {
                continue;
            }
            let precedence = heading_precedence(e);
            let is_real_container = precedence.0 == 1; // same LAYER bit heading_precedence itself uses
            for anchor in heading_anchors_for(e) {
                if is_real_container {
                    match real_anchor_owner.get(&anchor) {
                        Some(owner) if owner != &e.id => {
                            heading_anchor_collisions.push((anchor.clone(), owner.clone(), e.id.clone()));
                        }
                        Some(_) => {} // same event claims its own anchor more than once (e.g. two witnesses, same first verse) -- not a collision
                        None => {
                            real_anchor_owner.insert(anchor.clone(), e.id.clone());
                        }
                    }
                }
                let should_replace = match verse_heading_precedence.get(&anchor) {
                    None => true,
                    Some(&incumbent) => precedence > incumbent,
                };
                if should_replace {
                    verse_heading_precedence.insert(anchor.clone(), precedence);
                    verse_heading.insert(anchor, HeadingEntry { event_id: e.id.clone(), title: e.label.clone(), kind: e.kind.clone() });
                }
            }
        }
        self.verse_heading = verse_heading;
        self.heading_anchor_collisions = heading_anchor_collisions;

        // Batch HOTFIX-4 requirement 1: the global chronological timeline --
        // see `timeline_order`'s own field doc comment for the full ordering
        // rule. `self.events` is already `sort_by_key(|e| e.when.from_year)`-
        // sorted (above), so this stable secondary sort's own tie-breaking
        // (equal `(from_year, order_key)`) falls out to the ORIGINAL
        // pre-any-sort compiled-array order, matching `heading_precedence`'s
        // own documented "equal on all tiers keeps first-wins" precedent.
        let mut timeline_order: Vec<String> =
            self.events.iter().filter(|e| e.kind == "event").map(|e| e.id.clone()).collect();
        timeline_order.sort_by_key(|id| {
            let e = self.event_by_id(id).expect("id just collected from self.events");
            (e.when.from_year, e.order_key)
        });
        self.timeline_index = timeline_order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();
        self.timeline_order = timeline_order;

        self
    }

    pub fn event_by_id(&self, id: &str) -> Option<&Event> {
        self.event_index.get(id).map(|&i| &self.events[i])
    }

    pub fn place_by_id(&self, id: &str) -> Option<&Place> {
        self.place_index.get(id).map(|&i| &self.places[i])
    }

    /// Batch E: this place's curated history record, if any (most places
    /// have none -- `place_history` only covers the curated ~15-25).
    pub fn place_history_for(&self, id: &str) -> Option<&PlaceHistory> {
        self.place_history.get(id)
    }

    /// Batch E3: this place's curated KJV display-name alias, if any (most
    /// places have none -- `place_name_aliases` only covers the places a
    /// curator hand-verified against the actual KJV text).
    pub fn place_name_alias_for(&self, id: &str) -> Option<&PlaceNameAlias> {
        self.place_name_aliases.get(id)
    }

    /// Batch E2: ids of every event-bearing place ("cities in our graph",
    /// user direction 2026-08-19 quoted in batch-e2-brief.md) -- the fixed-
    /// cardinality set QUIET-1 checks `places` union `quiet_places` against
    /// for every time-mode window. Derived from `events` by `finish()`.
    pub fn event_bearing_place_ids(&self) -> &HashSet<String> {
        &self.event_bearing_place_ids
    }

    /// Batch E2: a place's ALL-TIME event count (every event touching it, in
    /// ANY window) -- `QuietPlace::total_events`'s source. 0 for a place
    /// with no events at all; such a place is never actually looked up by
    /// `scene::quiet_places` (it only iterates `event_bearing_place_ids`),
    /// but 0 is a safe, honest default rather than panicking if ever called
    /// directly.
    pub fn total_events_for(&self, id: &str) -> u32 {
        self.event_counts_by_place.get(id).copied().unwrap_or(0)
    }

    /// Event ids whose `verses` include the given canonical verse id.
    pub fn events_for_verse(&self, verse: &str) -> &[String] {
        self.verse_to_events.get(verse).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Batch T2: every `(anchor_verse, event_id, event_id)` pair where two
    /// DIFFERENT real (layer-1) curated containers anchor the identical
    /// verse -- see the `heading_anchor_collisions` field's own doc comment
    /// for what "real"/"layer-1" means and why this is checked separately
    /// from `heading_precedence`'s own (legitimate) collision resolution.
    /// Empty for well-formed curated data; `atlas_etl::validate::run` fails
    /// loud on every entry here.
    pub fn heading_anchor_collisions(&self) -> &[(String, String, String)] {
        &self.heading_anchor_collisions
    }

    /// Batch R requirement 5: place ids whose own `verse_links` include the
    /// given canonical verse id (the reverse of `Place::verse_links`) -- see
    /// `verse_to_places`'s own doc comment.
    pub fn places_for_verse(&self, verse: &str) -> &[String] {
        self.verse_to_places.get(verse).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Batch T requirement 5: the pericope heading (event id + title) that
    /// should render immediately above this verse in the reading flow, if
    /// any. A verse is a heading ANCHOR exactly when it is the FIRST verse
    /// of some "heading-worthy" event's own witness (`scene::witnesses_for`)
    /// -- "heading-worthy" means: a leg of one of the curated narratives
    /// (OT included -- every existing narrative event already carries a
    /// real title via `Event::label`, no new authoring needed for those),
    /// OR explicitly curated with parallel witnesses (`Event::witnesses`
    /// non-empty), OR explicitly Robertson-grounded (`Event::robertson_section`
    /// is `Some`) -- i.e. "Gospels + Acts (full coverage) + narrative-event
    /// passages elsewhere," per the owner's own coverage decision (verbatim:
    /// "General-passage titles outside these come later" -- a Theographic
    /// event this batch never touches, and that is a leg of no narrative,
    /// correctly produces NO heading here). A multi-witness event anchors
    /// ONE heading PER witness (one per Gospel book it's rendered in), never
    /// just once overall. Two CONTAINERS legitimately overlapping on one
    /// verse is expected, not an error (a real, live case -- `jm_bethany`/
    /// `pw_bethany` both anchor JHN.12.1) -- the READER is decisive:
    /// exactly one title heads that verse, chosen by `heading_precedence`'s
    /// own 3-tier rule (own doc comment has the full chain) -- fixed in
    /// fix-round-1, amended by the owner's own "decisive container" ruling,
    /// batch-t-review.md's Important-1. The non-chosen container is never
    /// unreachable -- see `events_for_verse`, which lists every container
    /// touching a verse regardless of which one won the heading.
    pub fn heading_for_verse(&self, verse: &str) -> Option<&HeadingEntry> {
        self.verse_heading.get(verse)
    }

    /// Batch HOTFIX-4 requirement 1: this event's own 0-based position in
    /// the GLOBAL chronological timeline (`timeline_order`'s own doc
    /// comment has the full ordering rule), if it's a dated (`kind ==
    /// "event"`) event at all. `None` for a general-kind or unknown id --
    /// requirement 2's own "NOT part of time traversal... fabricating a
    /// date is forbidden," resolved here by simple absence from the index
    /// this method reads, never a special-cased branch.
    pub fn timeline_position(&self, id: &str) -> Option<usize> {
        self.timeline_index.get(id).copied()
    }

    /// Batch HOTFIX-4 requirement 1: the dated event sitting at this
    /// 0-based position in the global timeline, if any. Every real caller
    /// derives `index` from `timeline_position` above (always in-bounds by
    /// construction); `None` here only guards a hypothetical out-of-range
    /// index rather than panicking.
    pub fn timeline_event_at(&self, index: usize) -> Option<&Event> {
        self.timeline_order.get(index).and_then(|id| self.event_by_id(id))
    }

    /// Batch F: every catechism item cited by any member verse of `span`
    /// (a single verse OR a same-chapter passage), in first-seen order, no
    /// duplicates -- the pure aggregation core lives in
    /// `crate::catechism::items_for_span` (mirrors
    /// `crate::xrefs::aggregate_span_xrefs`'s own split: pure function takes
    /// narrow borrowed inputs, this method is the thin "hand it this
    /// instance's own derived indexes" wrapper `atlas-server::handlers` calls
    /// directly, keeping business logic out of handlers.rs). Used for BOTH
    /// `GET /api/catechism/{sref}` (passages) and `GET /api/verse/{vref}`'s
    /// own embedded `catechism` field (a single-verse span) -- one function,
    /// no duplicated logic between the two.
    pub fn catechism_items_for_span(&self, span: &crate::refs::ScriptureRef) -> Vec<crate::catechism::CatechismRef> {
        crate::catechism::items_for_span(span, &self.verse_to_catechism, &self.catechism_item_names)
    }

    /// Batch F: one catechism item's own full record, plus the `CatechismPart`
    /// it belongs to (its `title` is part of the item-detail wire payload) --
    /// `GET /api/catechism/item/{id}`'s own lookup, O(1) via
    /// `catechism_item_index` rather than a linear scan of every part/item.
    pub fn catechism_item_by_id(&self, id: &str) -> Option<(&CatechismPart, &CatechismItem)> {
        let &(pi, ii) = self.catechism_item_index.get(id)?;
        Some((&self.catechism[pi], &self.catechism[pi].items[ii]))
    }

    /// Reads the SURVIVING compiled JSON files ETL writes under `dir`
    /// (exact filenames mirror `atlas-etl/src/main.rs`'s `write_json`
    /// calls: `canon.json`, `books-meta.json`, plus `polities.json`,
    /// `landmarks.json`, `place-history.json`, ...) and assembles an
    /// `AtlasData`. Does NOT call `.finish()` — the derived indexes are
    /// `#[serde(skip)]` and come back empty from a fresh deserialize (see
    /// the struct doc comment); callers must call `.finish()` themselves
    /// (or, on the server's own DEFAULT startup path, overlay the five
    /// fields below from the graph FIRST -- see `atlas_graph::legacy::
    /// atlas_data_overlay`'s own doc comment -- then call `.finish()`
    /// once).
    ///
    /// M-C2 DELETION EVENT (requirement 2, completing P1): `places.json`/
    /// `events.json`/`narratives.json`/`verses-kjv.json`/`cross-refs.json`
    /// retire, joining `eras.json`'s own M-C retirement -- every
    /// production reader of these five fields migrated onto the graph
    /// this batch (`handlers::place`/`event`/`verse`/`xrefs`/`narratives`
    /// directly; every NOT-yet-migrated surface via `atlas_graph::legacy::
    /// atlas_data_overlay`/`atlas_etl::compile::compile`, both graph/
    /// raw-sourced, never this loader -- see batch-mc2-report.md's own
    /// deletion inventory for the grep proof). The FIVE FIELDS stay on
    /// `AtlasData` (ETL-time validation, `.finish()`'s own derived-index
    /// web, and several test fixtures across this workspace still need
    /// them) -- only THIS loader's own reads retire; a fresh
    /// `AtlasData::load` now always starts with all five honestly empty,
    /// never stale or fabricated, exactly like `.eras` already does.
    pub fn load(dir: &std::path::Path) -> Result<Self, crate::CoreError> {
        let canon: Canon = read_json(dir, "canon.json")?;
        let places: Vec<Place> = Vec::new();
        let events: Vec<Event> = Vec::new();
        let narratives: Vec<Narrative> = Vec::new();
        let eras: Vec<Era> = Vec::new();
        let books_meta: Vec<BookMeta> = read_json(dir, "books-meta.json")?;
        let verses: HashMap<String, String> = HashMap::new();
        let cross_refs: HashMap<String, Vec<CrossRef>> = HashMap::new();

        let polities: Vec<Polity> = read_json(dir, "polities.json")?;
        let landmarks: Vec<Landmark> = read_json(dir, "landmarks.json")?;
        let place_history_list: Vec<PlaceHistory> = read_json(dir, "place-history.json")?;
        let place_history: HashMap<String, PlaceHistory> =
            place_history_list.into_iter().map(|h| (h.id.clone(), h)).collect();
        let place_name_alias_list: Vec<PlaceNameAlias> = read_json(dir, "place-names-kjv.json")?;
        let place_name_aliases: HashMap<String, PlaceNameAlias> =
            place_name_alias_list.into_iter().map(|a| (a.id.clone(), a)).collect();
        // Batch R requirement 1: `land-mask.json` is already the flattened
        // `Vec<Vec<(f64, f64)>>` shape (see `curated::parse_land_mask`'s own
        // doc comment for why region names/ref_notes never leave the ETL
        // step) -- a direct read, no per-region unwrapping needed here.
        let land_mask: Vec<Vec<(f64, f64)>> = read_json(dir, "land-mask.json")?;
        // Batch F: `catechism.json` is already the curated `Vec<CatechismPart>`
        // shape (see `CatechismPart`'s own doc comment) -- a direct read, no
        // per-part unwrapping needed here, same as `polities.json` above.
        let catechism: Vec<CatechismPart> = read_json(dir, "catechism.json")?;
        // Batch HOTFIX-6: `chronology-anchors.json`/`book-narration-windows.json`
        // are both already the curated `Vec<_>` shape (see each struct's own
        // doc comment) -- direct reads, same as `polities.json` above.
        let chronology_anchors: Vec<ChronologyAnchor> = read_json(dir, "chronology-anchors.json")?;
        let book_narration_windows: Vec<BookNarrationWindow> = read_json(dir, "book-narration-windows.json")?;

        let mut data = Self::new(canon, places, events, narratives, eras, books_meta, verses, cross_refs);
        data.polities = polities;
        data.place_history = place_history;
        data.place_name_aliases = place_name_aliases;
        data.landmarks = landmarks;
        data.land_mask = land_mask;
        data.catechism = catechism;
        data.chronology_anchors = chronology_anchors;
        data.book_narration_windows = book_narration_windows;
        Ok(data)
    }
}

/// Maps a signed calendar year (never zero) onto a contiguous integer line
/// with the zero-year gap removed: `..., -2, -1, 1, 2, ...` becomes
/// `..., -2, -1, 0, 1, ...` (AD years shift down by one; BC years are
/// unchanged, since they're already contiguous below zero). Used by
/// `nearest_border_year`'s midpoint math above and, via `pub(crate)`, by
/// `crate::history`'s own zero-aware window-midpoint math (same
/// straddle-BC/AD problem, same fix -- one source of truth for it).
pub(crate) fn year_index(y: Year) -> i64 {
    if y > 0 {
        (y - 1) as i64
    } else {
        y as i64
    }
}

fn read_json<T: serde::de::DeserializeOwned>(dir: &std::path::Path, file: &str) -> Result<T, crate::CoreError> {
    let path = dir.join(file);
    let text = std::fs::read_to_string(&path)
        .map_err(|source| crate::CoreError::Io { path: path.display().to_string(), source })?;
    serde_json::from_str(&text).map_err(|source| crate::CoreError::Json { path: path.display().to_string(), source })
}

/// A small, hand-built demo world. Originally
/// `atlas_core::scene::tests::fixture()` (Task 3, `#[cfg(test)]`-only);
/// promoted here as a normal `pub` function — not `#[cfg(test)]` — because
/// `atlas-server`'s integration tests live in a different crate and need it
/// at ordinary (non-test) compile time. `#[doc(hidden)]` keeps it out of
/// generated docs since it is not part of the intended public API surface.
///
/// The `places`/`events`/`narratives` below are byte-for-byte the Task 3
/// fixture: `scene.rs`'s own tests assert specific ids/links from this exact
/// shape (`jericho`, `gilgal`, `hebron`+`GEN.13.18`, `e1..e5`, `conquest`,
/// `patriarchs-demo`) and must keep passing unmodified. `canon`/`eras`/
/// `books_meta`/`verses`/`cross_refs` were empty placeholders in the Task 3
/// version (no scene test reads them); this promotion fills them with a
/// small consistent demo dataset for atlas-server's endpoint tests, whose
/// verses/cross-refs deliberately reuse ids already present above (e.g.
/// `GEN.13.18`, `JOS.6.20`) so the two halves of the fixture cross-check.
#[doc(hidden)]
pub fn demo_fixture() -> AtlasData {
    let places = vec![
        Place { id: "gilgal".into(), name: "Gilgal".into(), lat: 31.9000, lon: 35.4500, verse_links: vec![] },
        Place { id: "jericho".into(), name: "Jericho".into(), lat: 31.8703, lon: 35.4436, verse_links: vec![] },
        Place { id: "ai".into(), name: "Ai".into(), lat: 31.9339, lon: 35.2856, verse_links: vec![] },
        Place {
            id: "hebron".into(),
            name: "Hebron".into(),
            lat: 31.5326,
            lon: 35.0998,
            verse_links: vec!["GEN.13.18".into()],
        },
    ];
    let events = vec![
        Event {
            id: "e1".into(),
            label: "Camp at Gilgal".into(),
            when: TimeRange::new(-1406, -1406).unwrap(),
            places: vec!["gilgal".into()],
            verses: vec!["JOS.4.19".into(), "JOS.4.20".into()],
            ..Default::default()
        },
        Event {
            id: "e2".into(),
            label: "Jericho besieged".into(),
            when: TimeRange::new(-1406, -1406).unwrap(),
            places: vec!["jericho".into()],
            verses: vec!["JOS.6.1".into(), "JOS.6.2".into()],
            ..Default::default()
        },
        Event {
            id: "e3".into(),
            label: "Jericho falls".into(),
            when: TimeRange::new(-1405, -1405).unwrap(),
            places: vec!["jericho".into()],
            verses: vec!["JOS.6.20".into(), "JOS.6.21".into(), "JOS.6.24".into()],
            ..Default::default()
        },
        Event {
            id: "e4".into(),
            label: "Ai defeated".into(),
            when: TimeRange::new(-1405, -1405).unwrap(),
            places: vec!["ai".into()],
            verses: vec!["JOS.8.1".into(), "JOS.8.28".into()],
            ..Default::default()
        },
        Event {
            id: "e5".into(),
            label: "Sarah buried at Machpelah".into(),
            when: TimeRange::new(-2000, -2000).unwrap(),
            places: vec!["hebron".into()],
            verses: vec!["GEN.23.1".into(), "GEN.23.19".into()],
            ..Default::default()
        },
    ];
    let narratives = vec![
        Narrative {
            id: "conquest".into(),
            name: "The Conquest".into(),
            color: "#7C3AED".into(),
            legs: vec!["e1".into(), "e2".into(), "e3".into(), "e4".into()],
        },
        Narrative {
            id: "patriarchs-demo".into(),
            name: "Patriarchs (demo)".into(),
            color: "#D97706".into(),
            legs: vec!["e2".into()],
        },
    ];

    let canon = Canon {
        books: vec![
            // M-C2: widened from `vec![31]` (chapter 1 only) to also cover
            // chapters 13/23, the SAME "canon must cover every verse this
            // fixture actually has text/cross-refs for" fix as JOS below --
            // `xref_adapter::normalize`'s own "target's first verse must
            // exist" check (`atlas_etl::xrefs::filter_missing_first_verse`)
            // is checked against the GRAPH's own rebuilt verse map (that
            // module's own doc comment: "rebuilt by walking the
            // JUST-NORMALIZED graph nodes"), so a cross-ref TARGET outside
            // the declared canon (GEN.13.18, cited by JOS.6.20's own third
            // cross-ref below) was silently dropped at graph-build time
            // even though `data.verses`/`data.cross_refs` both carried it.
            // `/api/books` reads `data.canon` directly (unaffected by
            // which handler sources its OWN text from) so its own
            // `books[0]["chapters"]` wire value changes with this fix --
            // `health_books_eras_narratives_shapes`'s own assertion is
            // updated in the SAME commit, per this file's own citation-
            // integrity discipline (never let a wire assertion silently
            // drift from the fixture it's supposed to pin).
            CanonBook { code: "GEN".into(), name: "Genesis".into(), chapters: vec![31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 19] },
            // M-C2: widened from `vec![3]` (chapter 1 only) to cover every
            // JOS chapter this fixture's own `verses` map below actually
            // has text for (1/4/6/8) -- a real, self-found fixture bug:
            // `handlers::verse`/`handlers::chapter`'s own verse TEXT now
            // comes from the graph's TextUnit node table, which
            // `kjv_adapter::normalize` builds by walking `canon.books`'s
            // own declared chapter/verse counts (`ordered_verses_from_canon`),
            // NOT by scanning `verses` directly -- unlike the pre-M-C2
            // `data.verses.get(key)` read, which never consulted `canon` at
            // all. A `verses` entry for a chapter `canon` never declares
            // (JOS.4/6/8 against the old `chapters: vec![3]`) was silently
            // reachable through the old flat-map read but invisible to the
            // graph -- exactly the kind of drift `graph_fixture_for`'s own
            // doc comment above already warns this fixture must avoid.
            // Interior chapters this fixture has no verses for (2/3/5/7)
            // stay 0 (a lawful, honest "no verses this chapter" -- the
            // graph simply never materializes nodes for them, same as any
            // real book's own unwritten chapters would).
            CanonBook { code: "JOS".into(), name: "Joshua".into(), chapters: vec![3, 0, 0, 20, 0, 24, 0, 28] },
        ],
    };
    let eras = vec![
        Era { id: "patriarchs".into(), name: "Patriarchs".into(), from_year: -2166, to_year: -1877 },
        Era { id: "conquest-judges".into(), name: "Conquest & Judges".into(), from_year: -1406, to_year: -1051 },
    ];
    let books_meta = vec![
        BookMeta {
            book: "GEN".into(),
            author: "Moses".into(),
            write_place: None,
            write_from: Some(-1445),
            write_to: Some(-1405),
        },
        BookMeta {
            book: "JOS".into(),
            author: "Joshua".into(),
            write_place: Some("gilgal".into()),
            write_from: Some(-1400),
            write_to: Some(-1370),
        },
    ];

    let mut verses = HashMap::new();
    verses.insert("JOS.1.1".into(), "Now after the death of Moses it came to pass, that the LORD spake unto Joshua.".to_string());
    verses.insert("JOS.1.2".into(), "Moses my servant is dead; now therefore arise, go over this Jordan.".to_string());
    verses.insert("JOS.1.3".into(), "Every place that the sole of your foot shall tread upon, that have I given unto you.".to_string());
    verses.insert("JOS.4.19".into(), "And the people came up out of Jordan, and encamped in Gilgal.".to_string());
    verses.insert("JOS.4.20".into(), "And those twelve stones did Joshua pitch in Gilgal.".to_string());
    verses.insert("JOS.6.1".into(), "Now Jericho was straitly shut up because of the children of Israel.".to_string());
    verses.insert("JOS.6.2".into(), "And the LORD said unto Joshua, See, I have given into thine hand Jericho.".to_string());
    verses.insert(
        "JOS.6.20".into(),
        "So the people shouted, and the wall fell down flat, and they took the city.".to_string(),
    );
    verses.insert("JOS.6.21".into(), "And they utterly destroyed all that was in the city.".to_string());
    verses.insert("JOS.6.24".into(), "And they burnt the city with fire, and all that was therein.".to_string());
    verses.insert("JOS.8.1".into(), "And the LORD said unto Joshua, Fear not, neither be thou dismayed.".to_string());
    verses.insert("JOS.8.28".into(), "And Joshua burnt Ai, and made it an heap for ever.".to_string());
    verses.insert(
        "GEN.13.18".into(),
        "Then Abram removed his tent, and came and dwelt in the plain of Mamre, which is in Hebron.".to_string(),
    );
    verses.insert("GEN.23.1".into(), "And Sarah was an hundred and seven and twenty years old.".to_string());
    verses.insert(
        "GEN.23.19".into(),
        "And after this, Abraham buried Sarah his wife in the cave of the field of Machpelah.".to_string(),
    );

    let mut cross_refs = HashMap::new();
    cross_refs.insert(
        "JOS.6.20".into(),
        vec![
            CrossRef { target: "JOS.6.20-21".into(), votes: 9 }, // same-chapter span target
            CrossRef { target: "JOS.1.3".into(), votes: 5 },     // plain single-verse target
            CrossRef { target: "GEN.13.18".into(), votes: 2 },   // single-verse target, different book
        ],
    );
    // Batch G1 (GET /api/xrefs/{sref}): JOS.6.21's own list, added so
    // atlas-server's own endpoint test can exercise the REAL span-
    // aggregation handler end to end against the span JOS.6.20-21, not just
    // atlas_core::xrefs's pure unit/property tests. Deliberately overlaps
    // JOS.6.20's own JOS.1.3 target (5 + 4 = 9, proving votes are SUMMED
    // across member verses, not just taken from whichever verse is seen
    // first) and cites JOS.6.20 itself (a self-target once the span is
    // JOS.6.20-21, proving the drop rule) -- see
    // verse_chapter_place_and_404's own JOS.6.20 assertions above, which
    // this entry (a different map key) leaves completely untouched.
    cross_refs.insert(
        "JOS.6.21".into(),
        vec![
            CrossRef { target: "JOS.1.3".into(), votes: 4 },
            CrossRef { target: "JOS.6.20".into(), votes: 3 },
        ],
    );

    let mut data = AtlasData::new(canon, places, events, narratives, eras, books_meta, verses, cross_refs).finish();
    // Batch E: one small curated history record (hebron -- already present
    // above with a real event, e5, and its own GEN.23.* verses) so
    // atlas-server's own integration tests can exercise the REAL
    // `/api/place/{id}?from=&to=` resolution pipeline end to end, not just
    // atlas_core::history's pure functions in isolation. Values are
    // plausible, not independently curated the way data/curated/
    // place-history.toml's real content is -- this fixture exists only to
    // give the endpoint something to resolve.
    data.place_history.insert(
        "hebron".into(),
        PlaceHistory {
            id: "hebron".into(),
            names: vec![
                PlaceNameEntry { name: "Kirjath-arba".into(), when: TimeRange::new(-4004, -2001).unwrap(), verses: vec!["GEN.23.2".into()] },
            ],
            blurbs: vec![PlaceBlurbEntry {
                text: "Abraham buried Sarah in the cave of Machpelah here.".into(),
                when: TimeRange::new(-2166, -1877).unwrap(),
                breadth: "era".into(),
            }],
            established: Some(PlaceDateClaim {
                when: TimeRange::new(-2000, -2000).unwrap(),
                verses: vec!["GEN.23.19".into()],
                note: Some("traditional".into()),
            }),
            destroyed: None,
        },
    );

    // Batch F: one small curated catechism entry, citing JOS.6.20 -- a verse
    // already present above AND already carrying its own cross-refs (see the
    // `cross_refs` map), deliberately reused rather than inventing an
    // unrelated verse id: this proves a verse can carry cross-refs AND a
    // catechism citation at once without either system disturbing the
    // other, exactly the kind of layered content this batch's own wire
    // (`VerseDetailOut.catechism` alongside `.cross_refs`) is built to
    // support. `text` is `None` -- mirrors a REAL Baptism/Confession/
    // Sacrament-of-the-Altar item's own shape (see `CatechismItem`'s doc
    // comment), so this fixture exercises that conditional-presence branch
    // too, not just the more common `Some(...)` case.
    data.catechism = vec![CatechismPart {
        id: "demo-part".into(),
        title: "Demo Part".into(),
        items: vec![CatechismItem {
            id: "demo-item-1".into(),
            name: "Demo Catechism Item".into(),
            text: None,
            explanation_heading: "What does this mean?".into(),
            explanation: "Demo item explanation.".into(),
            where_written: Some("Demo where-written text.".into()),
            verses: vec!["JOS.6.20".into()],
            ref_note: None,
            // Batch F2: a QUESTION-level citation on the SAME item, citing a
            // DIFFERENT already-real verse (JOS.6.21, already present above)
            // -- exercises both citation granularities on one item, so
            // server integration tests can assert a verse's own catechism
            // hit carries the right `question` without a second fixture.
            questions: vec![CatechismQuestion {
                title: "Demo Question".into(),
                verses: vec!["JOS.6.21".into()],
                source: "brain-fuel/catechism".into(),
            }],
        }],
    }];
    // `finish()` is documented idempotent (safe to call more than once) --
    // re-run here, exactly as the place_history insert above relies on NOT
    // needing to, because unlike place_history, the catechism-derived
    // indexes (`verse_to_catechism`/`catechism_item_names`/
    // `catechism_item_index`) are only built INSIDE finish(), which already
    // ran (empty) before `data.catechism` was ever populated.
    data.finish()
}

#[cfg(test)]
mod verse_to_places_tests {
    use super::*;

    // Batch R requirement 5 (place-in-verse hover -> marker blink):
    // `places_for_verse` is the reverse of `Place::verse_links` -- proven
    // here against `demo_fixture()`'s own `hebron` place (GEN.13.18 is its
    // one curated verse_link) rather than in atlas-server's own endpoint
    // test, since exercising this through `GET /api/chapter/{cref}` would
    // require growing the fixture's deliberately tiny GEN canon entry (see
    // atlas-server's `verse_chapter_place_and_404`'s own comment on why it
    // doesn't).
    #[test]
    fn places_for_verse_resolves_the_reverse_of_verse_links() {
        let data = demo_fixture();
        assert_eq!(data.places_for_verse("GEN.13.18"), &["hebron".to_string()]);
    }

    #[test]
    fn places_for_verse_is_empty_for_an_unlinked_verse() {
        let data = demo_fixture();
        assert!(data.places_for_verse("JOS.1.1").is_empty());
    }
}

#[cfg(test)]
mod events_for_verse_witness_tests {
    use super::*;
    use std::collections::HashMap;

    // Batch T requirement 3: a REAL, live-caught bug (self-caught via a
    // hanging Playwright test before this shipped, not guessed) --
    // `events_for_verse` used to be built from `Event::verses` ALONE, so a
    // verse cited ONLY by a witness (never in the event's own top-level
    // `verses`) resolved to NO event at all, even though the identical
    // verse correctly anchors a reader heading (`heading_anchors_for`,
    // witness-aware from the start). Pinned here so it can never silently
    // regress: a multi-witness event whose own top-level `verses` names
    // only ONE book must still resolve via a DIFFERENT book's own
    // witness-only verse.
    fn multi_witness_fixture() -> AtlasData {
        let places = vec![Place { id: "golgotha".into(), name: "Golgotha".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![Event {
            id: "pw_golgotha".into(),
            label: "The crucifixion at Golgotha".into(),
            when: crate::time::TimeRange::new(33, 33).unwrap(),
            places: vec!["golgotha".into()],
            verses: vec!["JHN.19.17".into()], // top-level: JOHN only, matching this batch's own real pw_golgotha shape
            witnesses: vec![
                EventWitness {
                    book: "MAT".into(),
                    translations: HashMap::from([("kjv".to_string(), vec!["MAT.27.33".to_string()])]),
                    ref_note: None,
                    robertson_section: None,
                },
                EventWitness {
                    book: "MRK".into(),
                    translations: HashMap::from([("kjv".to_string(), vec!["MRK.15.22".to_string()])]),
                    ref_note: None,
                    robertson_section: None,
                },
            ],
            ..Default::default()
        }];
        AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish()
    }

    #[test]
    fn events_for_verse_resolves_a_witness_only_verse_not_in_the_top_level_verses_field() {
        let data = multi_witness_fixture();
        // MAT.27.33 is ONLY in the Matthew witness, never in Event::verses
        // (which names only JHN.19.17) -- this is exactly the bug: it must
        // still resolve to pw_golgotha.
        assert_eq!(data.events_for_verse("MAT.27.33"), &["pw_golgotha".to_string()]);
        assert_eq!(data.events_for_verse("MRK.15.22"), &["pw_golgotha".to_string()]);
    }

    #[test]
    fn events_for_verse_still_resolves_the_top_level_verses_field_too() {
        let data = multi_witness_fixture();
        assert_eq!(data.events_for_verse("JHN.19.17"), &["pw_golgotha".to_string()]);
    }

    #[test]
    fn events_for_verse_never_double_lists_an_event_for_one_verse() {
        // A verse present in BOTH Event::verses and a witness (an unusual
        // but not-impossible curated shape) must yield the event id ONCE,
        // never twice, for the same verse.
        let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![Event {
            id: "e1".into(),
            label: "E1".into(),
            when: crate::time::TimeRange::new(33, 33).unwrap(),
            places: vec!["p".into()],
            verses: vec!["MAT.1.1".into()],
            witnesses: vec![EventWitness {
                book: "MAT".into(),
                translations: HashMap::from([("kjv".to_string(), vec!["MAT.1.1".to_string()])]),
                ref_note: None,
                robertson_section: None,
            }],
            ..Default::default()
        }];
        let data = AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        assert_eq!(data.events_for_verse("MAT.1.1"), &["e1".to_string()]);
    }
}

#[cfg(test)]
mod heading_collision_tests {
    use super::*;
    use std::collections::HashMap;

    // Batch T fix-round-1 (Important-1, batch-t-review.md), amended by the
    // owner's own "decisive container" ruling: a real, live-reproduced
    // defect -- two heading-worthy CONTAINERS anchoring the IDENTICAL verse
    // (the real case: `jm_bethany`, a bare `jesus-ministry` leg with no
    // witnesses/robertson_section -- heading-worthy only via the
    // "existing-title freebie" narrative-leg rule -- and `pw_bethany`, a
    // REAL curated container for this exact grouping, this batch's own
    // flagship 3-witness passion-week leg, both anchor JHN.12.1) used to
    // resolve via plain first-wins `or_insert_with`, which -- since both
    // share one traditional year and the sort is merely stable -- meant
    // bare FILE ORDER silently decided which container's heading rendered,
    // dropping the other's entirely. Fixed: `heading_precedence`'s own
    // 3-tier rule (layer, then kind, then chronology -- see its own doc
    // comment for the full chain) now deterministically decides a
    // collision, regardless of vec/file order. `bare_leg`/`rich_leg` below
    // mirror `jm_bethany`/`pw_bethany`'s own real shape exactly (bare =
    // narrative-leg-only "freebie" layer; rich = a real container, via
    // witnesses) rather than using the real curated ids, so this test pins
    // the MECHANISM, independent of any future edit to the real Bethany
    // data.
    fn bare_leg() -> Event {
        Event {
            id: "bare_leg".into(),
            label: "A bare narrative-leg-only event".into(),
            when: crate::time::TimeRange::new(33, 33).unwrap(),
            verses: vec!["JHN.12.1".into()],
            ..Default::default()
        }
    }

    fn rich_leg() -> Event {
        Event {
            id: "rich_leg".into(),
            label: "A richer, witness-bearing event".into(),
            when: crate::time::TimeRange::new(33, 33).unwrap(),
            verses: vec!["JHN.12.1".into()], // ALSO cites the identical anchor verse
            witnesses: vec![EventWitness {
                book: "JHN".into(),
                translations: HashMap::from([("kjv".to_string(), vec!["JHN.12.1".to_string(), "JHN.12.2".to_string()])]),
                ref_note: None,
                robertson_section: Some("Robertson (1922) fixture section".into()),
            }],
            ..Default::default()
        }
    }

    // `bare_leg` needs a narrative leg to be heading-worthy at all (it has
    // no witnesses/robertson_section of its own) -- mirrors jm_bethany's
    // own real "leg of jesus-ministry" status exactly.
    fn narrative_for(leg_id: &str) -> Narrative {
        Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec![leg_id.to_string()] }
    }

    #[test]
    fn heading_collision_prefers_the_richer_event_when_bare_is_first_in_order() {
        // Mirrors the EXACT real bug: bare event first in the events vec
        // (jm_bethany's own file position), rich event second.
        let events = vec![bare_leg(), rich_leg()];
        let narratives = vec![narrative_for("bare_leg")];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "rich_leg", "the richer (witness-bearing) event must win the collision, not whichever happened to sort first");
    }

    #[test]
    fn heading_collision_prefers_the_richer_event_regardless_of_vec_order() {
        // Reverse of the above: rich event first this time -- proves the
        // rule is content-driven (richness), never incidentally order-driven.
        let events = vec![rich_leg(), bare_leg()];
        let narratives = vec![narrative_for("bare_leg")];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "rich_leg");
    }

    #[test]
    fn heading_collision_shadowed_event_stays_reachable_via_event_membership() {
        // Important-1's own reason this is Important, not Critical: the
        // LOSING event of a heading collision must remain fully reachable
        // through the verse's own EVENT membership section (events_for_verse),
        // completely independent of which one won the single heading slot.
        let events = vec![bare_leg(), rich_leg()];
        let narratives = vec![narrative_for("bare_leg")];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let mut ids = data.events_for_verse("JHN.12.1").to_vec();
        ids.sort();
        assert_eq!(ids, vec!["bare_leg".to_string(), "rich_leg".to_string()], "BOTH events must stay listed in the verse's own EVENT membership, win or lose the heading");
    }

    #[test]
    fn heading_collision_first_wins_when_richness_is_equal() {
        // Two EQUALLY bare (both narrative-leg-only, no witnesses/
        // robertson_section) events colliding is a narrower, lower-stakes
        // case this fix deliberately leaves alone -- confirms the OLD
        // stable-first-wins behavior is unchanged for ties, so this fix's
        // blast radius is exactly the rich-vs-bare case it targets, nothing
        // wider.
        let mut other_bare = bare_leg();
        other_bare.id = "other_bare_leg".into();
        other_bare.label = "Another bare event, same verse".into();
        let events = vec![bare_leg(), other_bare];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["bare_leg".into(), "other_bare_leg".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "bare_leg", "equal richness must keep first-wins (stable sort order), unchanged from before this fix");
    }

    #[test]
    fn heading_collision_tier3_earlier_chronology_wins_between_two_real_containers() {
        // Owner's own ruling: "if both colliders are genuinely same-layer
        // containers, tiebreak: event-kind over general, then narrative-leg
        // order." Two REAL containers (both with witnesses -- same layer,
        // same kind) colliding on the identical verse: the chronologically
        // EARLIER one (by (from_year, order_key)) must win, not whichever
        // sorts first in the vec. Currently vacuous in the real curated
        // data (no two real containers collide today), but the rule must
        // hold regardless.
        let mut earlier = rich_leg();
        earlier.id = "earlier_rich".into();
        earlier.when = crate::time::TimeRange::new(30, 30).unwrap(); // earlier year
        let mut later = rich_leg();
        later.id = "later_rich".into();
        later.when = crate::time::TimeRange::new(33, 33).unwrap(); // later year
        // `later` placed FIRST in the vec -- proves the win is chronology-
        // driven, not incidentally order-driven (mirrors the two order-
        // independence tests above for tier 1).
        let events = vec![later, earlier];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "earlier_rich", "between two real containers of equal layer/kind, the chronologically earlier one must win");
    }

    #[test]
    fn heading_collision_tier1_event_kind_beats_general_kind() {
        // Owner's own ruling's first tiebreak tier: "event-kind over
        // general." Currently vacuous in real curated data (no
        // kind="general" passage exists yet -- Batch T's own disclosed,
        // scoped limitation), but the rule must hold: two otherwise-
        // identical bare containers (same layer, same year/order_key),
        // one `kind="event"`, one `kind="general"`, colliding on the same
        // verse -- "event" must win.
        let mut event_kind = bare_leg();
        event_kind.id = "as_event".into();
        event_kind.kind = "event".into();
        let mut general_kind = bare_leg();
        general_kind.id = "as_general".into();
        general_kind.kind = "general".into();
        // `general_kind` placed FIRST -- proves the win is kind-driven, not
        // incidentally order-driven.
        let events = vec![general_kind, event_kind];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["as_event".into(), "as_general".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "as_event", "kind=\"event\" must beat kind=\"general\" between two same-layer containers");
    }

    #[test]
    fn heading_collision_acts_section_counts_as_a_real_layer1_container() {
        // Batch T2 (Acts provenance): `acts_section` is Acts's own sibling
        // to `robertson_section` -- a DIFFERENT field (per the owner's own
        // ambiguity ruling, "acts sections get their own provenance key,
        // NOT robertson_section"), but it must count the SAME way in
        // `heading_precedence`'s own LAYER tier: a bare freebie (neither
        // witnesses, robertson_section, NOR acts_section) must lose to a
        // real container carrying acts_section alone, same as the existing
        // robertson_section-only case already proven above.
        let mut with_acts_section = bare_leg();
        with_acts_section.id = "as_acts".into();
        with_acts_section.acts_section = Some("Acts pericope (this project's own sectioning)".into());
        let mut plain_bare = bare_leg();
        plain_bare.id = "as_bare".into();
        // `plain_bare` placed FIRST -- proves the win is layer-driven, not
        // incidentally order-driven.
        let events = vec![plain_bare, with_acts_section];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["as_bare".into(), "as_acts".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "as_acts", "acts_section alone must make a container real (layer 1), same as robertson_section");
    }

    #[test]
    fn heading_collision_atlas_section_counts_as_a_real_layer1_container() {
        // Batch W1 (whole-Bible titled verse containers): `atlas_section` is
        // the general, whole-Bible sibling of `robertson_section`/
        // `acts_section` (see `Event::atlas_section`'s own doc comment) --
        // same LAYER-1 treatment, proven the same way the two siblings
        // above already are: a bare freebie (none of the three provenance
        // fields, no witnesses) must lose to a real container carrying
        // atlas_section alone.
        let mut with_atlas_section = bare_leg();
        with_atlas_section.id = "as_atlas".into();
        with_atlas_section.atlas_section = Some("Atlas pericope (this project's own sectioning): GEN.1.1-2.3".into());
        let mut plain_bare = bare_leg();
        plain_bare.id = "as_bare2".into();
        // `plain_bare` placed FIRST -- proves the win is layer-driven, not
        // incidentally order-driven.
        let events = vec![plain_bare, with_atlas_section];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["as_bare2".into(), "as_atlas".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "as_atlas", "atlas_section alone must make a container real (layer 1), same as robertson_section/acts_section");
    }

    #[test]
    fn heading_collision_kjv_superscription_counts_as_a_real_layer1_container() {
        // Batch W3 (Psalms granularity): `kjv_superscription` is the KJV's
        // own literal-citation sibling of `robertson_section`/
        // `acts_section`/`atlas_section` (see `Event::kjv_superscription`'s
        // own doc comment) -- same LAYER-1 treatment, proven the same way
        // the three siblings above already are: a bare freebie (none of the
        // four provenance fields, no witnesses) must lose to a real
        // container carrying kjv_superscription alone.
        let mut with_kjv_superscription = bare_leg();
        with_kjv_superscription.id = "as_kjv".into();
        with_kjv_superscription.kjv_superscription =
            Some("A Psalm of David, when he fled from Absalom his son (PSA.3.1).".into());
        let mut plain_bare = bare_leg();
        plain_bare.id = "as_bare3".into();
        // `plain_bare` placed FIRST -- proves the win is layer-driven, not
        // incidentally order-driven.
        let events = vec![plain_bare, with_kjv_superscription];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["as_bare3".into(), "as_kjv".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "as_kjv", "kjv_superscription alone must make a container real (layer 1), same as robertson_section/acts_section/atlas_section");
    }
}

#[cfg(test)]
mod catechism_tests {
    use super::*;
    use crate::refs::ScriptureRef;

    // Batch F: `catechism_items_for_span`/`catechism_item_by_id` against
    // `demo_fixture()`'s own small catechism entry (citing JOS.6.20 -- see
    // that fixture's own comment for why that particular, already-busy
    // verse was deliberately reused).
    #[test]
    fn catechism_items_for_span_resolves_a_cited_verse() {
        let data = demo_fixture();
        let span = ScriptureRef::Verse(crate::refs::VerseId {
            book: crate::canon::resolve_alias("JOS").unwrap(),
            chapter: 6,
            verse: 20,
        });
        let out = data.catechism_items_for_span(&span);
        assert_eq!(
            out,
            vec![crate::catechism::CatechismRef { id: "demo-item-1".into(), name: "Demo Catechism Item".into(), question: None }]
        );
    }

    // Batch F2: JOS.6.21 is cited only via demo-item-1's own QUESTION-level
    // `questions` entry (not its item-level `verses`) -- the resulting hit
    // must carry that question's own title, proving the new citation
    // granularity flows end to end through AtlasData, not just through
    // atlas_core::catechism's own unit tests.
    #[test]
    fn catechism_items_for_span_resolves_a_question_level_citation() {
        let data = demo_fixture();
        let span = ScriptureRef::Verse(crate::refs::VerseId {
            book: crate::canon::resolve_alias("JOS").unwrap(),
            chapter: 6,
            verse: 21,
        });
        let out = data.catechism_items_for_span(&span);
        assert_eq!(
            out,
            vec![crate::catechism::CatechismRef {
                id: "demo-item-1".into(),
                name: "Demo Catechism Item".into(),
                question: Some("Demo Question".into()),
            }]
        );
    }

    #[test]
    fn catechism_items_for_span_is_empty_for_an_uncited_verse() {
        let data = demo_fixture();
        let span = ScriptureRef::Verse(crate::refs::VerseId {
            book: crate::canon::resolve_alias("JOS").unwrap(),
            chapter: 1,
            verse: 1,
        });
        assert!(data.catechism_items_for_span(&span).is_empty());
    }

    #[test]
    fn catechism_item_by_id_resolves_the_owning_part_and_item() {
        let data = demo_fixture();
        let (part, item) = data.catechism_item_by_id("demo-item-1").expect("demo-item-1 must resolve");
        assert_eq!(part.id, "demo-part");
        assert_eq!(part.title, "Demo Part");
        assert_eq!(item.name, "Demo Catechism Item");
        assert_eq!(item.text, None);
        assert_eq!(item.verses, vec!["JOS.6.20".to_string()]);
    }

    #[test]
    fn catechism_item_by_id_returns_none_for_an_unknown_id() {
        let data = demo_fixture();
        assert!(data.catechism_item_by_id("no-such-item").is_none());
    }
}

#[cfg(test)]
mod year_index_tests {
    use super::*;

    // Batch B2: `nearest_border_year` (the snapshot-year model it served)
    // and its own dedicated test module are gone -- see `Polity`'s doc
    // comment. `year_index` itself survives (still used by
    // `crate::history`'s zero-aware window-midpoint math), so its own unit
    // coverage survives with it, just no longer nested under a
    // border-flavored module name.
    #[test]
    fn year_index_is_contiguous_across_the_bc_ad_seam() {
        assert_eq!(year_index(-1), -1);
        assert_eq!(year_index(1), 0);
        assert_eq!(year_index(-2), -2);
        assert_eq!(year_index(2), 1);
        // -1 and 1 (adjacent calendar years, matching time::next_year) must
        // be exactly 1 apart in index space.
        assert_eq!(year_index(1) - year_index(-1), 1);
    }
}
