use serde::Serialize;

/// `GET /api/land-mask` (Batch R requirement 1, "borders become part of the
/// plate"): the curated land/coastline mask (`data/curated/land-mask.toml`)
/// used ONLY to clip polity washes so they never spill into open sea --
/// static geometry, no query params, no time dependence (unlike
/// `/api/polities`). `rings` is a flat `[[[lat,lon],...],...]` array -- every
/// region's own rings, already flattened at ETL time (see
/// `AtlasData::land_mask`'s own doc comment) -- the client never needs
/// region names/ref_notes, only the raw geometry to build an SVG clipPath
/// from.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LandMask {
    pub rings: Vec<Vec<(f64, f64)>>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Polities {
    pub polities: Vec<Polity>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Polity {
    pub id: String,
    pub name: String,
    pub from: i32,
    pub to: i32,
    pub rings: Vec<Vec<(f64, f64)>>,
    pub color_key: u8,
    /// Batch M requirement 1: this era's own delta metadata, OMITTED (not
    /// null) when absent -- "an uneventful boundary stays visible but gets
    /// the minimal popover," so the client must be able to tell "no
    /// transition curated" apart from "transition curated with zero
    /// verses" (the latter still shows the event/grounding-note sections,
    /// just no THE SCRIPTURES section — see `PolityDelta::verses`' own doc
    /// comment). Mirrors `NarrativePosition.prior`/`.following`'s own
    /// `skip_serializing_if` precedent exactly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<PolityDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fall: Option<PolityDelta>,
}

/// Batch B2 ("borders v2, the cartographer's edition"): one polity-era row
/// on the wire, `{ id, name, from, to, rings, color_key }` per the batch
/// brief verbatim -- field names deliberately match
/// `atlas_core::data::PolityEra`'s own (`from`/`to`, not `from_year`/
/// `to_year`; see that struct's doc comment for why) so this is a plain
/// copy, not a rename. `id` is the POLITY's id (constant across every era
/// row this same polity contributes to a response), `name`/`from`/`to`/
/// `rings` are this specific ERA's own fields, and `color_key` is the
/// polity's own precomputed hash (`Polity::color_key` -- copied here
/// unchanged, never rehashed per-request).
/// Batch M requirement 1: the wire shape of one `atlas_core::data::PolityDelta`
/// -- a plain field-for-field copy (`event`/`verses`/`ref_note`), same "no
/// rename, no reshaping" convention `Polity` itself already follows for
/// `PolityEra`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PolityDelta {
    pub event: String,
    pub verses: Vec<String>,
    pub ref_note: String,
}

impl From<&atlas_core::data::PolityDelta> for PolityDelta {
    fn from(d: &atlas_core::data::PolityDelta) -> Self {
        PolityDelta { event: d.event.clone(), verses: d.verses.clone(), ref_note: d.ref_note.clone() }
    }
}
