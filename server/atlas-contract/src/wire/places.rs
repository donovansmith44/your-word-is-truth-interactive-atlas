use serde::Serialize;

use atlas_core::time::TimeRange;
use atlas_core::wire::SceneEvent;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceDetail {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub events: Vec<SceneEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<History>,
    /// Batch E3 (requirement 2's quiet provenance note): the bare, stripped,
    /// un-aliased, un-period-resolved canonical name. ALWAYS COMPUTED --
    /// unlike `history`, which stays absent for a place with no curated
    /// `PlaceHistory` record at all (e.g. `cush-2`), this field's own
    /// resolution never depends on `history` existing -- but only ever
    /// SERIALIZED (`Some`) when it differs from whatever name is actually
    /// showing this request (a curated period name OR a curated KJV alias
    /// resolved to something else). Omitted (`None`, not a repeat of the
    /// title) whenever this place's displayed name already IS its canonical
    /// name, so the client's own popover renders NO provenance note rather
    /// than a vacuous "known elsewhere as X" that just repeats the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    /// ENT-1a (owner order: "we actually want meaningful information about
    /// who or what someone is, having that be backed by scripture"):
    /// Easton's Bible Dictionary (1897, public domain) prose, source-
    /// attested, `None` until a match exists -- never fabricated. ADDITIVE
    /// JSON (batch-ent1a-brief.md controller decision 3): the current
    /// client ignores unknown/absent fields; the EntityProfile presentation
    /// half that renders this is HELD for the frontend-elegance brainstorm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Batch E: `/api/place/{id}`'s optional `history` payload, present only
/// when this place has a curated `PlaceHistory` record at all (`when
/// curated`, per the brief). `display_name` and `blurb` are resolved
/// against the request's `?from=&to=` window when given (else `display_name`
/// falls back to the place's own default `name` and `blurb` is omitted --
/// see `history::resolve_display_name`/`resolve_blurb`'s own doc comments);
/// `established`/`destroyed` are window-independent static facts, always
/// included verbatim whenever curated.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blurb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub established: Option<DateClaim>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destroyed: Option<DateClaim>,
}

/// Batch E: one curated established/destroyed date claim, as served by
/// `/api/place/{id}`. `when` reuses `TimeRange`'s own wire shape
/// (`from_year`/`to_year`) rather than a separate "year" field -- the
/// client's `YearText.FormatRange` already collapses equal endpoints to a
/// single-year display, so a genuine year and a range need no separate flag.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DateClaim {
    pub when: TimeRange,
    pub verses: Vec<String>,
    pub note: Option<String>,
}
