use serde::Serialize;

use atlas_core::time::TimeRange;
use atlas_core::wire::VerseGroup;

/// Batch HOTFIX-4 requirement 1: `GET /api/narrative/event/{id}`'s own
/// extended wire shape -- WAS a bare `Vec<NarrativePosition>` (Batch N);
/// NOW an object, `narrative` carrying EXACTLY that same array (unchanged
/// shape, unchanged rows, every existing narrative-scoped consumer keeps
/// reading it unmodified) alongside the NEW `timeline` field. `timeline` is
/// OMITTED (not `null`) entirely for a general-kind or unknown event id
/// (requirement 2: "general-kind containers... NOT part of time traversal"),
/// present otherwise with `prior`/`following` each independently omitted
/// only at the atlas's own true first/last dated event (conditional
/// presence, matching every other optional field on this wire). Every
/// consumer of the OLD bare-array shape is migrated in this same commit:
/// `AtlasClient.NarrativeEventPositions` (client), `EventNode`/
/// `INarrativeAware` (client), `PlaceCard.LoadNarrativePositions` (client,
/// reads `.Narrative` — TRAVERSAL-1 logic unchanged), and the Playwright
/// helper call sites in `world-pin.spec.ts`/`popover-sections.spec.ts`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NarrativeEventPositions {
    pub narrative: Vec<NarrativePosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<TimelinePosition>,
}

/// Batch N: one (narrative, event) position a queried verse or event
/// touches -- `prior`/`following` omitted (not null) exactly at a
/// narrative's own first/last leg, same conditional-presence wire
/// convention `History.blurb`/`CatechismRef.question` etc. already
/// use throughout `wire/`. `event_id`/`event_label` are carried (map-
/// focus-sync + disambiguating two positions sharing one `narrative_id`)
/// even though they restate something the CALLER usually already knows.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NarrativePosition {
    pub narrative_id: String,
    pub narrative_name: String,
    pub event_id: String,
    pub event_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior: Option<NarrativeAdjacentEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<NarrativeAdjacentEvent>,
}

// M-D3 (owner ruling R5): `impl From<atlas_core::narrative::NarrativePosition>
// for NarrativePosition` retired -- genuinely orphaned (grep-proven: no
// call site). `events::narrative_event_positions` has built
// `NarrativePosition` directly, from the graph's own succession-edge
// topology, since M-B; this conversion's OWN source type
// (`atlas_core::narrative::NarrativePosition`, produced only by the
// now-retired `positions_for_events`) has had no live producer since.
// Recoverable from git history at the commit immediately preceding this
// one.

/// Batch N: one event ADJACENT to a `NarrativePosition` (its own PRIOR or
/// FOLLOWING leg) -- id/label/places/verse_groups, per requirement 1
/// verbatim ("each adjacent event carrying its id, label, place(s), and
/// verse groups"). `verse_groups` is built by
/// `atlas_core::scene::to_scene_event` -- the SAME function every other
/// "an event's own verses on the wire" case in this crate already calls
/// (`reading::verse`'s own `VerseEvent` construction, `places::place`'s own
/// event list) -- so this
/// is provably the same data a map arrow's own endpoint would show for the
/// identical event id, not a parallel re-derivation.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NarrativeAdjacentEvent {
    pub id: String,
    pub label: String,
    pub places: Vec<String>,
    pub verse_groups: Vec<VerseGroup>,
}

impl From<atlas_core::narrative::NarrativeAdjacentEvent> for NarrativeAdjacentEvent {
    fn from(e: atlas_core::narrative::NarrativeAdjacentEvent) -> Self {
        NarrativeAdjacentEvent { id: e.id, label: e.label, places: e.places, verse_groups: e.verse_groups }
    }
}

/// Batch HOTFIX-4 requirement 1: the GLOBAL chronological PRIOR/FOLLOWING
/// for one event id, independent of narrative membership -- see
/// `atlas_core::narrative::TimelinePosition`'s own doc comment for the full
/// ordering rule. Reuses `NarrativeAdjacentEvent` (same shape, same
/// "id/label/places/verse_groups" the narrative rows already send) -- one
/// computation, one wire type, two consumers.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TimelinePosition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior: Option<NarrativeAdjacentEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<NarrativeAdjacentEvent>,
}

impl From<atlas_core::narrative::TimelinePosition> for TimelinePosition {
    fn from(p: atlas_core::narrative::TimelinePosition) -> Self {
        TimelinePosition { prior: p.prior.map(Into::into), following: p.following.map(Into::into) }
    }
}

/// `GET /api/event/{id}`'s own wire shape (Batch T requirement 4, "EVENT
/// node popover"): `title` (this event's own `Event::label`), `kind`
/// (Batch T2: `"event"` | `"general"`, ALWAYS present -- the client's own
/// signal for which sections apply), `when`, every resolved place, every
/// resolved witness (ALWAYS >=1 -- see `scene::witnesses_for`'s own doc
/// comment for the single-implicit-witness synthesis; requirement 4's
/// "single-witness events show the one passage, no parallel framing when
/// n=1" is a CLIENT-side rendering decision keyed off `witnesses.len()`,
/// not a server-side omission), and provenance (`robertson_section`/
/// `ref_note`, each omitted, not null, when this event's own date/grouping
/// needed no note beyond the other).
///
/// Batch T2: `when` is OMITTED (not null) for a `kind == "general"`
/// passage -- the internal `Event::when` still holds
/// `TimeRange::undated()` (a structurally-required field, see that
/// function's own doc comment), but a general-kind passage has no
/// defensible date, so nothing here may ever present that sentinel to a
/// reader as a real claim. `places` stays an always-present, possibly-
/// empty array (unchanged pattern -- a general-kind passage's own
/// `Event::places` is always empty by construction, so this needs no
/// separate gating).
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventDetail {
    pub id: String,
    pub title: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<TimeRange>,
    pub places: Vec<EventPlace>,
    pub witnesses: Vec<EventWitness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robertson_section: Option<String>,
    /// Batch T2 (Acts provenance): Acts's own sibling provenance field to
    /// `robertson_section` above -- see `atlas_core::data::Event::
    /// acts_section`'s own doc comment for why it's separate, not reused.
    /// Omitted (not null) when absent, same convention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acts_section: Option<String>,
    /// Batch W1 (whole-Bible titled verse containers): the general,
    /// whole-Bible sibling of `acts_section` above -- see
    /// `atlas_core::data::Event::atlas_section`'s own doc comment. Omitted
    /// (not null) when absent, same convention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas_section: Option<String>,
    /// Batch W3: the KJV's own literal-citation sibling of `robertson_section`/
    /// `acts_section`/`atlas_section` above -- see `atlas_core::data::Event::
    /// kjv_superscription`'s own doc comment. Omitted (not null) when absent,
    /// same convention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kjv_superscription: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_note: Option<String>,
    /// ATTEST-1 (L3, "Mentioned in"): the verses that MENTION this event
    /// without narrating it -- canonical verse ids, ascending. Distinct
    /// from `witnesses` ON PURPOSE, and that distinction IS the batch: an
    /// account NARRATES the event and belongs under PARALLEL ACCOUNTS; a
    /// mention merely REFERENCES it while narrating something else, and
    /// rendering one as the other is exactly the "fundamental error" the
    /// owner reported. An event whose whole scriptural basis is mentions
    /// (the Espousal of Mary) serves an EMPTY `witnesses` and a non-empty
    /// list here -- a real node with a real frontier, never a fabricated
    /// parallel-accounts section.
    ///
    /// OMITTED (not `[]`) when empty, the same convention every optional
    /// field above follows -- so an event with no mentions serves
    /// byte-identically to before this batch.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mentioned_in: Vec<String>,
    /// ATTEST-1 (L4, "Similar Accounts"): events joined to this one by an
    /// `Analogue` row -- "distinct events whose accounts are similar in
    /// form or content, NEVER two accounts of one event." Deliberately a
    /// SEPARATE field from `witnesses` rather than a flag on it: the
    /// owner's own report was that a similar-but-distinct story was being
    /// rendered as a parallel account, and one field cannot carry two
    /// claims. Omitted when empty, same convention.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub analogues: Vec<EventAnalogue>,
    /// Batch PROV-1 (owner order 1): THIS EVENT's own source -- the Event
    /// node's `provenance`, off `Node::provenance`
    /// (`event_world::event_provenance`: `"theographic"` for an imported
    /// event, `"curated"` for one this project authored). The focus card's
    /// own attribution, and the TOTAL-CAPTURE HONESTY case in one field: a
    /// hand-authored event says `curated`, which resolves to the registry's
    /// own "Our Own Curated Work" category, and therefore cannot silently
    /// wear Theographic's clothes at the reader.
    pub provenance: String,
    /// Batch PROV-1: every distinct provenance id behind THIS EVENT's
    /// PARALLEL ACCOUNTS section -- the `Attests` rows for this event
    /// specifically, not the family average.
    ///
    /// A list, and per-EVENT rather than per-family, even though the real
    /// `attests` table is single-sourced TODAY (`{event-witnesses}`,
    /// measured by `the_per_family_provenance_map_of_the_real_artifact_
    /// is_pinned`; ATTEST-1's `attestation-corrections` rows land on
    /// `mentions`/`analogue`, not here -- an earlier version of this
    /// comment said otherwise and was wrong). Per-event is what keeps this
    /// TRUE if a second source ever lands in the table: a family average
    /// would start lying the moment it did, and THE LEPER LESSON is exactly
    /// that a hand-authored row must never be attributed to an importer.
    ///
    /// Omitted (not `[]`) when empty -- an event with no accounts (the
    /// Espousal of Mary) renders no PARALLEL ACCOUNTS section at all, so it
    /// needs no attribution for one; same convention as `mentioned_in`
    /// above, so such an event serves byte-identically to before this
    /// batch but for the two unconditional fields.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub witnesses_provenance: Vec<String>,
    /// Batch PROV-1: the same, for the "Mentioned in" section -- the
    /// `Mentions` rows naming THIS event.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mentions_provenance: Vec<String>,
}

/// Batch T requirement 4: one EVENT-kind PASSAGE's own resolved place --
/// id (to open a `PlaceNode`/target the map) + display name (so the client
/// never needs a second lookup just to label an explorable place row).
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventPlace {
    pub id: String,
    pub name: String,
}

/// Batch T requirement 4: one resolved witness -- "book, verse-range,
/// translation-mapped," the owner's own words verbatim, ALREADY resolved to
/// this app's one compiled translation (`atlas_core::translation::resolve`,
/// real fail-loud lookup, not a silent default) and grouped via the SAME
/// `verse_groups_for` every other verse list on this wire already uses
/// (`atlas_core::scene::witnesses_for` -- one function, so a heading's own
/// anchor verse and this section's own witness list can never disagree).
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventWitness {
    pub book: String,
    pub verse_groups: Vec<VerseGroup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robertson_section: Option<String>,
}

impl From<atlas_core::scene::ResolvedWitness> for EventWitness {
    fn from(w: atlas_core::scene::ResolvedWitness) -> Self {
        EventWitness { book: w.book, verse_groups: w.verse_groups, ref_note: w.ref_note, robertson_section: w.robertson_section }
    }
}

/// ATTEST-1: one end of an `Analogue` -- enough to render and to explore
/// (`/api/event/{id}` takes this `id` straight back).
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventAnalogue {
    pub id: String,
    pub title: String,
    /// Batch PROV-1: genuinely PER-ROW -- the `Analogue` row joining these
    /// two events carries its own provenance, and this is that value, not
    /// either end's node provenance and not the family's. An `Analogue` is
    /// a curatorial CLAIM about two events ("similar in form or content,
    /// never two accounts of one event"); attributing it to whoever
    /// supplied the events would name the wrong asserter.
    pub provenance: String,
}
