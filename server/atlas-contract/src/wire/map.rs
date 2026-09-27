use atlas_core::data::PolityDelta;
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

/// One polity-era row: `id` is the POLITY's (constant across every era row
/// the same polity contributes to a response) and `color_key` its own
/// precomputed hash, while `name`/`from`/`to`/`rings` are this specific ERA's.
/// The field names deliberately match `atlas_core::data::PolityEra`'s own
/// (`from`/`to`, not `from_year`/`to_year` -- see that struct's doc comment
/// for why), so the era half is a plain copy, not a rename.
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
