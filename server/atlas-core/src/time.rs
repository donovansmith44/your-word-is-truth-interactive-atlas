use serde::{Deserialize, Serialize};

pub type Year = i32;

pub fn next_year(y: Year) -> Year {
    if y == -1 {
        1
    } else {
        y + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    pub from_year: Year,
    pub to_year: Year,
}

impl TimeRange {
    pub fn new(from_year: Year, to_year: Year) -> Result<Self, crate::CoreError> {
        if from_year == 0 || to_year == 0 {
            return Err(crate::CoreError::ZeroYear);
        }
        if from_year > to_year {
            return Err(crate::CoreError::InvertedRange);
        }
        Ok(Self { from_year, to_year })
    }
    pub fn intersects(&self, o: &TimeRange) -> bool {
        self.from_year <= o.to_year && o.from_year <= self.to_year
    }
    pub fn contains_year(&self, y: Year) -> bool {
        self.from_year <= y && y <= self.to_year
    }

    /// Batch T2 (general-kind PASSAGEs, `atlas_core::data::Event::kind ==
    /// "general"`): a structurally-required `TimeRange` with no real
    /// chronological claim. `Event::when` deliberately stays a required
    /// (non-`Option`) `TimeRange` -- see batch-t2-report.md for the
    /// disclosed reasoning (an `Option<TimeRange>` migration would have
    /// touched every existing map/scene/test call site across the whole
    /// workspace for a field that, for a general-kind passage, is never
    /// read anyway: `places` stays empty too, so `scene::lit_places`'s own
    /// per-place grouping never even looks at it). This is the SAME
    /// "I need some TimeRange value but have no real date" idiom
    /// `scene.rs`'s own scripture-mode `mention-*` pseudo-events and
    /// arrow/legend fallback already use (`TimeRange::new(-4004,
    /// 100).unwrap()`, hand-written at each call site) -- centralized here
    /// under a self-documenting name so a general-kind event's own `when`
    /// reads as "deliberately undated," not as a mystery pair of numbers.
    /// Exactly the atlas's own curated span bounds, so it always passes
    /// `atlas_etl::validate::run`'s `[-4004,100]` bound check with no
    /// special case, and is immediately recognizable (spans the WHOLE
    /// atlas) rather than resembling a plausible specific year -- the
    /// structural half of "do not fabricate a date": nothing here is ever
    /// curator-typed, and the server's own `GET /api/event/{id}` handler
    /// omits `when` entirely from the wire for a general-kind passage (see
    /// `atlas-server/src/handlers.rs::event`), so this value never reaches
    /// a reader as a claim in the first place.
    pub const fn undated() -> Self {
        TimeRange { from_year: -4004, to_year: 100 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn undated_is_the_whole_atlas_span() {
        // Batch T2 (general-kind PASSAGEs): `Event::when` stays a required
        // `TimeRange` structurally (no Option<TimeRange> migration -- see
        // batch-t2-report.md for why), so a kind="general" passage (no
        // defensible date) needs SOME TimeRange value that (a) always
        // passes atlas_etl::validate's own [-4004,100] bound check with no
        // special case, and (b) is instantly recognizable as "no real
        // claim" rather than resembling a plausible specific year. The
        // whole-atlas-span bounds are exactly that -- the same idiom
        // scene.rs's own scripture-mode `mention-*` pseudo-events and
        // arrow/legend fallback already use for "I need a TimeRange here
        // but have no real date."
        let u = TimeRange::undated();
        assert_eq!(u.from_year, -4004);
        assert_eq!(u.to_year, 100);
    }

    #[test]
    fn zero_year_rejected() {
        assert!(TimeRange::new(0, 5).is_err());
        assert!(TimeRange::new(-5, 0).is_err());
        assert!(TimeRange::new(5, -5).is_err()); // from > to
    }
    #[test]
    fn bc_ad_adjacency() {
        assert_eq!(next_year(-1), 1);
        assert_eq!(next_year(-2), -1);
        assert_eq!(next_year(1), 2);
    }
    #[test]
    fn contains_year_boundaries() {
        let bc = TimeRange::new(-1450, -1400).unwrap();
        assert!(bc.contains_year(-1450)); // lower boundary, in
        assert!(bc.contains_year(-1400)); // upper boundary, in
        assert!(!bc.contains_year(-1451)); // just below lower, out
        assert!(!bc.contains_year(-1399)); // just above upper, out

        let ad = TimeRange::new(1, 100).unwrap();
        assert!(ad.contains_year(1)); // lower boundary, in
        assert!(ad.contains_year(100)); // upper boundary, in
        assert!(!ad.contains_year(101)); // just above upper, out
    }
    #[test]
    fn intersect_examples() {
        let a = TimeRange::new(-1450, -1400).unwrap();
        assert!(a.intersects(&TimeRange::new(-1400, -1300).unwrap())); // touching
        assert!(!a.intersects(&TimeRange::new(-1399, -1300).unwrap()));
    }
    proptest! {
        #[test]
        fn intersects_symmetric(a in range_strategy(), b in range_strategy()) {
            prop_assert_eq!(a.intersects(&b), b.intersects(&a));
        }
        #[test]
        fn contains_implies_intersects(a in range_strategy(), b in range_strategy()) {
            if b.from_year >= a.from_year && b.to_year <= a.to_year {
                prop_assert!(a.intersects(&b));
            }
        }
    }
    fn range_strategy() -> impl Strategy<Value = TimeRange> {
        (-4004i32..=100, -4004i32..=100)
            .prop_filter("no zero", |(a, b)| *a != 0 && *b != 0)
            .prop_map(|(a, b)| TimeRange::new(a.min(b), a.max(b)).unwrap())
    }
}
