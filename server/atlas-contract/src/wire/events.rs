use serde::Serialize;

use atlas_core::narrative::{NarrativeAdjacentEvent, TimelinePosition};
use atlas_core::scene::EventWitness;
use atlas_core::time::TimeRange;

use super::reading::PlaceRef;

/// Where one event sits in time: in each narrative it belongs to, and in the
/// atlas's whole chronology.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NarrativeEventPositions {
    pub narrative: Vec<NarrativePosition>,
    /// The event's place in the whole chronology; absent for an event with no date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<TimelinePosition>,
}

/// One event's place in one narrative, with the legs on either side of it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NarrativePosition {
    pub narrative_id: String,
    pub narrative_name: String,
    pub event_id: String,
    pub event_label: String,
    /// The leg before this one; absent at the narrative's first leg.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior: Option<NarrativeAdjacentEvent>,
    /// The leg after this one; absent at the narrative's last leg.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<NarrativeAdjacentEvent>,
}

/// One event, or one titled passage, in full.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventDetail {
    pub id: String,
    pub title: String,
    pub kind: atlas_core::data::EventKind,
    /// The years the event spans; absent for a titled passage that has no date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<TimeRange>,
    pub places: Vec<PlaceRef>,
    /// The passages that narrate this event, one per book. Always at least one.
    pub witnesses: Vec<EventWitness>,
    /// The section of Robertson's Harmony of the Gospels this event falls in, absent
    /// when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robertson_section: Option<String>,
    /// The section of the outline of Acts this event falls in, absent when it has
    /// none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acts_section: Option<String>,
    /// The titled section of this atlas's own outline of Scripture, absent when it
    /// has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas_section: Option<String>,
    /// The superscription the King James Version prints over this passage, absent
    /// when it prints none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kjv_superscription: Option<String>,
    /// A note on how this event's date and grouping were arrived at, absent when
    /// none was needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_note: Option<String>,
    /// The verses that mention this event without narrating it, in canonical order.
    /// Omitted when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mentioned_in: Vec<String>,
    /// Events whose accounts resemble this one without being accounts of it.
    /// Omitted when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub analogues: Vec<EventAnalogue>,
    /// The id of the source that asserts this event; `/api/sources` names it.
    pub provenance: String,
    /// The sources behind the passages that narrate this event, for this event
    /// rather than for the corpus. Omitted when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub witnesses_provenance: Vec<String>,
    /// The sources behind the verses that mention this event. Omitted when there are
    /// none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mentions_provenance: Vec<String>,
}

/// One event whose account resembles another's -- similar in form or in content,
/// never a second account of the same happening.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventAnalogue {
    pub id: String,
    pub title: String,
    /// The id of the source that asserts the resemblance. A resemblance is a claim
    /// of its own, so this is neither event's own source.
    pub provenance: String,
}
