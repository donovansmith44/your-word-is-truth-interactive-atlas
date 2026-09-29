//! Years as a reader reads them. Every label the atlas shows for a year or a span
//! of years is written here, once.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{time, CoreError};

const EN_DASH: &str = " – ";

/// A year, negative for BC, with the label a reader sees for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Year {
    pub value: i32,
    /// Such as `1447 BC` or `AD 30`.
    pub label: String,
}

impl Year {
    pub fn of(value: i32) -> Result<Year, CoreError> {
        match value {
            0 => Err(CoreError::ZeroYear),
            _ => Ok(Year::labelled(value)),
        }
    }

    // Only for a value read off a validated range, which holds no year zero; any other
    // value goes through `of`, which refuses it.
    pub(crate) fn labelled(value: i32) -> Year {
        let label = match Era::of(value) {
            Era::BeforeChrist => format!("{} BC", value.unsigned_abs()),
            Era::AnnoDomini => format!("AD {value}"),
        };
        Year { value, label }
    }
}

/// A span of years, both ends included, with the label a reader sees for it: an
/// era both ends share is named once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TimeRange {
    pub from: Year,
    pub to: Year,
    /// Such as `1450 – 1400 BC`, `5 BC – AD 30` or, for a single year, `AD 33`.
    pub label: String,
}

impl TimeRange {
    pub fn of(range: time::TimeRange) -> TimeRange {
        let (from, to) = (Year::labelled(range.from_year), Year::labelled(range.to_year));
        let label = match (Era::of(from.value), Era::of(to.value)) {
            _ if from == to => from.label.clone(),
            (Era::BeforeChrist, Era::BeforeChrist) => format!("{}{EN_DASH}{}", from.value.unsigned_abs(), to.label),
            (Era::AnnoDomini, Era::AnnoDomini) => format!("{}{EN_DASH}{}", from.label, to.value),
            _ => format!("{}{EN_DASH}{}", from.label, to.label),
        };
        TimeRange { from, to, label }
    }
}

enum Era {
    BeforeChrist,
    AnnoDomini,
}

impl Era {
    fn of(value: i32) -> Era {
        if value < 0 {
            Era::BeforeChrist
        } else {
            Era::AnnoDomini
        }
    }
}
