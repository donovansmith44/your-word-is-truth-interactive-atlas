use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{time, CoreError};

const EN_DASH: &str = " – ";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields, try_from = "LabelledYear")]
#[schema(description = "A year, negative for BC, with the label a reader sees for it, such as `1447 BC` or `AD 30`.")]
pub struct Year {
    pub value: i32,
    pub label: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelledYear {
    value: i32,
    #[serde(rename = "label")]
    _label: String,
}

impl TryFrom<LabelledYear> for Year {
    type Error = CoreError;

    fn try_from(arriving: LabelledYear) -> Result<Year, CoreError> {
        Year::of(arriving.value)
    }
}

impl Year {
    pub fn of(value: i32) -> Result<Year, CoreError> {
        match value {
            0 => Err(CoreError::ZeroYear),
            _ => Ok(Year::labelled(value)),
        }
    }

    pub(crate) fn labelled(value: i32) -> Year {
        let label = match Era::of(value) {
            Era::BeforeChrist => format!("{} BC", value.unsigned_abs()),
            Era::AnnoDomini => format!("AD {value}"),
        };
        Year { value, label }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields, try_from = "LabelledRange")]
#[schema(description = "A span of years, both ends included, with the label a reader sees for it: an era both ends share is named once, as in `1450 – 1400 BC`, `5 BC – AD 30` or, for a single year, `AD 33`.")]
pub struct TimeRange {
    pub from: Year,
    pub to: Year,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelledRange {
    from: Year,
    to: Year,
    #[serde(rename = "label")]
    _label: String,
}

impl TryFrom<LabelledRange> for TimeRange {
    type Error = CoreError;

    fn try_from(arriving: LabelledRange) -> Result<TimeRange, CoreError> {
        time::TimeRange::new(arriving.from.value, arriving.to.value).map(TimeRange::of)
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
