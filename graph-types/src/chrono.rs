//! Chronology: day-capable time, single-feed placements, total order.
//! No event carries a date literal; every ordering commitment names its
//! basis; adopted tradition carries its justification inside the hash.

use std::cmp::Ordering;

use crate::edge::Justification;
use crate::id::{AnchorId, EraId, EventId};

/// Signed year, no year zero — the invariant lives in the constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Year(i32);

impl Year {
    pub fn new(y: i32) -> Result<Self, YearError> {
        if y == 0 {
            Err(YearError::YearZero)
        } else {
            Ok(Year(y))
        }
    }
    pub fn get(self) -> i32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YearError {
    YearZero,
}

/// Time at the precision the sources give it — down to the day.
/// Ordering convention for missing precision (documented, refined at
/// first day-precision authoring): None sorts before Some within the
/// same coarser unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimePoint {
    pub year: Year,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

impl TimePoint {
    pub fn new(year: Year, month: Option<u8>, day: Option<u8>) -> Result<Self, TimeError> {
        if day.is_some() && month.is_none() {
            return Err(TimeError::DayWithoutMonth);
        }
        Ok(TimePoint { year, month, day })
    }
    pub fn year_only(year: Year) -> Self {
        TimePoint { year, month: None, day: None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeError {
    DayWithoutMonth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResolvedDate {
    pub from: TimePoint,
    pub to: TimePoint, // invariant: from <= to
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
    pub years: i32,
    pub months: u8,
    pub days: u8,
}

impl Duration {
    pub const fn years(y: i32) -> Self {
        Duration { years: y, months: 0, days: 0 }
    }
}

/// Single-feed placement: dates resolve from the anchor table; sequence
/// placement supplies order within equal TimePoints — where adopted
/// traditional chronology lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatePlacement {
    AnchorBinding { anchor: AnchorId, offset: Duration },
    ReignYear { reign: AnchorId, year_of_reign: u8 },
    SequenceAfter { prior: EventId, spacing: Duration },
    EraOnly { era: EraId },
}

/// The heterogeneous target a dated-by edge points at (sweep F5).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChronoTarget {
    Anchor(AnchorId),
    Prior(EventId),
    Era(EraId),
}

impl DatePlacement {
    pub fn target(&self) -> ChronoTarget {
        match self {
            DatePlacement::AnchorBinding { anchor, .. } => ChronoTarget::Anchor(anchor.clone()),
            DatePlacement::ReignYear { reign, .. } => ChronoTarget::Anchor(reign.clone()),
            DatePlacement::SequenceAfter { prior, .. } => ChronoTarget::Prior(prior.clone()),
            DatePlacement::EraOnly { era } => ChronoTarget::Era(era.clone()),
        }
    }
}

/// What kind of ground an ordering commitment stands on. The why of a
/// Traditional placement lives in the row's justification (one carrier),
/// inside the content-addressed identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlacementBasis {
    /// The text itself fixes the time.
    Textual,
    /// Confessionally acceptable traditional chronology.
    Traditional,
}

/// The dated-by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatedBy {
    pub event: EventId,
    pub placement: DatePlacement,
    pub basis: PlacementBasis,
    pub justification: Justification,
    pub provenance: crate::ingest::ProvenanceId,
}

/// Sequence key within equal TimePoints, resolved from SequenceAfter
/// chains — total by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeqKey(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResolvedPlacement {
    pub date: ResolvedDate,
    pub seq: SeqKey,
    pub basis: PlacementBasis,
}

/// TOTAL. Precision first, then the traditional-sequence key. No
/// cluster of uncertainty ever reaches a surface.
pub fn temporal_order(a: &ResolvedPlacement, b: &ResolvedPlacement) -> Ordering {
    a.date
        .from
        .cmp(&b.date.from)
        .then_with(|| a.date.to.cmp(&b.date.to))
        .then_with(|| a.seq.cmp(&b.seq))
}
