use serde::Serialize;

use atlas_core::narrative::{NarrativeAdjacentEvent, TimelinePosition};
use atlas_core::scene::EventWitness;
use atlas_core::time::TimeRange;

use super::reading::PlaceRef;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "Where one event sits in time: in each narrative it belongs to, and in the atlas's whole chronology.")]
pub struct NarrativeEventPositions {
    pub narrative: Vec<NarrativePosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<TimelinePosition>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One event's place in one narrative, with the legs on either side of it.")]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One event, or one titled passage, in full.")]
pub struct EventPage {
    pub id: String,
    pub title: String,
    pub kind: atlas_core::data::EventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<TimeRange>,
    pub places: Vec<PlaceRef>,
    pub witnesses: Vec<EventWitness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robertson_section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acts_section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas_section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kjv_superscription: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_note: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mentioned_in: Vec<atlas_core::refs::VerseId>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub analogues: Vec<EventAnalogue>,
    pub provenance: super::Provenance,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub witnesses_provenance: Vec<super::Provenance>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mentions_provenance: Vec<super::Provenance>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One event whose account resembles another's -- similar in form or in content, never a second account of the same happening.")]
pub struct EventAnalogue {
    pub id: String,
    pub title: String,
    pub provenance: super::Provenance,
}
