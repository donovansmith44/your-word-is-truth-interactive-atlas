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
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
