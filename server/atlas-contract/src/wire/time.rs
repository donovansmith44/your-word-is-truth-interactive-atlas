use serde::Serialize;
use utoipa::ToSchema;

pub use atlas_core::label::{TimeRange, Year};

use super::{NodeRef, TextSpan};

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A date something is claimed to have happened at, the verses the claim rests on, the event it rests on where it names one, and the note that qualifies it where the date is not Scripture's own; the label is marked `c. ` where the claim carries a note.")]
pub struct DateClaim {
    pub when: TimeRange,
    pub label: String,
    pub verses: Vec<TextSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<NodeRef>,
}

impl DateClaim {
    pub fn of(when: TimeRange, verses: Vec<TextSpan>, note: Option<String>, event: Option<NodeRef>) -> DateClaim {
        let label = match note {
            Some(_) => format!("c. {}", when.label),
            None => when.label.clone(),
        };
        DateClaim { when, label, verses, note, event }
    }
}
