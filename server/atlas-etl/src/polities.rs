//! Geometry helpers for the curated, hand-authored polity borders: the biblical-world box every curated
//! coordinate must fall inside, the deterministic tint assignment, and the ring-simplicity gate.

use atlas_core::time::Year;

/// The biblical-world clip/label box: every compiled place's lat/lon extent plus a flat 4-degree margin,
/// rounded to one decimal. The client derives the same constants, so if the places' extent ever moves this
/// box, both must move together.
pub const BIBLICAL_WORLD_BBOX: Bbox = Bbox { south: 7.6, north: 48.9, west: -10.9, east: 71.4 };

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bbox {
    pub south: f64,
    pub north: f64,
    pub west: f64,
    pub east: f64,
}

impl Bbox {
    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        lat >= self.south && lat <= self.north && lon >= self.west && lon <= self.east
    }
}

/// This atlas's span. There is no year zero; `-4004` is Ussher's creation year and `100` the latest curated
/// event.
pub const ATLAS_START_YEAR: Year = -4004;
pub const ATLAS_END_YEAR: Year = 100;

/// The number of hues in the client's tint palette. It stays at or above the curated polity count, which is
/// what lets every polity claim its own distinct tint.
pub const POLITY_TINT_COUNT: u32 = 16;

/// A polity's deterministic STARTING probe position in the palette, hashed from its stable id alone: the sum
/// of its lowercased characters' code values, mod the palette length, mirroring the client's own former hash.
/// A seed, not the final answer -- collisions between seeds are resolved when the roster is assigned.
fn color_seed(id: &str) -> u8 {
    let normalized = id.trim().to_lowercase();
    let sum: u32 = normalized.chars().map(|c| c as u32).sum();
    (sum % POLITY_TINT_COUNT) as u8
}

/// Assigns one distinct tint per id by starting at each id's seed and linear-probing forward for a bucket no EARLIER
/// id in this call claimed. `sorted_ids` MUST already be sorted by the caller; that fixed order plus a deterministic
/// probe is what makes the result reproducible. Past palette size a probe falls back to its seed, accepting a collision.
pub fn assign_color_keys(sorted_ids: &[&str]) -> Vec<u8> {
    let mut used = vec![false; POLITY_TINT_COUNT as usize];
    let mut out = Vec::with_capacity(sorted_ids.len());
    for id in sorted_ids {
        let seed = color_seed(id);
        let mut assigned = seed;
        for step in 0..POLITY_TINT_COUNT {
            let candidate = ((seed as u32 + step) % POLITY_TINT_COUNT) as u8;
            if !used[candidate as usize] {
                assigned = candidate;
                break;
            }
        }
        used[assigned as usize] = true;
        out.push(assigned);
    }
    out
}

fn orientation(p: (f64, f64), q: (f64, f64), r: (f64, f64)) -> i32 {
    let val = (q.1 - p.1) * (r.0 - q.0) - (q.0 - p.0) * (r.1 - q.1);
    // A small epsilon rather than exact zero absorbs floating-point noise from hand-typed decimal coordinates
    // without letting a real, visually obvious crossing read as merely collinear: these rings are
    // country-scale, so 1e-9 is far below anything a curator could type as a deliberate near-miss.
    if val.abs() < 1e-9 {
        0
    } else if val > 0.0 {
        1
    } else {
        2
    }
}

/// Given `p`, `q`, `r` already known to be collinear, is `q` inside `p` and `r`'s bounding box -- that is,
/// does `q` sit ON the segment rather than merely on the infinite line through it.
fn on_segment(p: (f64, f64), q: (f64, f64), r: (f64, f64)) -> bool {
    q.0 <= p.0.max(r.0) && q.0 >= p.0.min(r.0) && q.1 <= p.1.max(r.1) && q.1 >= p.1.min(r.1)
}

/// The standard CCW/orientation-based segment-intersection test (Cormen et al., Introduction to Algorithms):
/// true when the open segments cross, the degenerate collinear-overlap cases included.
fn segments_intersect(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), p4: (f64, f64)) -> bool {
    let o1 = orientation(p1, p2, p3);
    let o2 = orientation(p1, p2, p4);
    let o3 = orientation(p3, p4, p1);
    let o4 = orientation(p3, p4, p2);

    if o1 != o2 && o3 != o4 {
        return true;
    }
    if o1 == 0 && on_segment(p1, p3, p2) {
        return true;
    }
    if o2 == 0 && on_segment(p1, p4, p2) {
        return true;
    }
    if o3 == 0 && on_segment(p3, p1, p4) {
        return true;
    }
    if o4 == 0 && on_segment(p3, p2, p4) {
        return true;
    }
    false
}

/// Is `ring` a simple polygon -- no two non-adjacent edges crossing? It may arrive closed, with its first point
/// repeated last as the curated convention has it, or open; a repeated closing point is stripped so it is never
/// mistaken for a separate vertex. Fewer than three distinct points is vacuously simple.
pub fn ring_is_simple(ring: &[(f64, f64)]) -> bool {
    let open: &[(f64, f64)] = if ring.len() >= 2 && ring[0] == ring[ring.len() - 1] { &ring[..ring.len() - 1] } else { ring };
    let n = open.len();
    if n < 3 {
        return true;
    }

    for i in 0..n {
        let a1 = open[i];
        let a2 = open[(i + 1) % n];
        for j in (i + 1)..n {
            // Two edges are adjacent -- sharing one endpoint, which is never a crossing -- when edge j starts
            // where edge i ends, or, the one wraparound this loop can still reach, when edge i is the ring's
            // own closing edge back to edge j's start.
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            if adjacent {
                continue;
            }
            let b1 = open[j];
            let b2 = open[(j + 1) % n];
            if segments_intersect(a1, a2, b1, b2) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_contains_checks_all_four_edges() {
        let bbox = BIBLICAL_WORLD_BBOX;
        assert!(bbox.contains(31.5, 35.0), "Jerusalem-ish point should be inside");
        assert!(!bbox.contains(90.0, 35.0), "north pole is outside");
        assert!(!bbox.contains(31.5, 200.0), "lon way outside");
    }

    #[test]
    fn color_seed_is_deterministic_and_in_range() {
        for id in ["egypt", "roman-empire", "judah", ""] {
            let a = color_seed(id);
            let b = color_seed(id);
            assert_eq!(a, b, "color_seed must be a pure function of id");
            assert!((a as u32) < POLITY_TINT_COUNT, "{a} out of range for {id:?}");
        }
    }

    #[test]
    fn color_seed_ignores_case_and_surrounding_whitespace() {
        assert_eq!(color_seed("Egypt"), color_seed("egypt"));
        assert_eq!(color_seed(" egypt "), color_seed("egypt"));
    }

    #[test]
    fn color_seed_differs_for_most_distinct_ids() {
        let ids = ["egypt", "assyria", "babylon", "judah", "israel", "roman-empire"];
        let keys: std::collections::HashSet<u8> = ids.iter().map(|id| color_seed(id)).collect();
        assert!(keys.len() > 1, "expected some variety across {ids:?}, got all-same");
    }

    const REAL_CURATED_POLITY_IDS_SORTED: [&str; 14] = [
        "alexander-empire", "assyria", "babylon", "egypt", "elam", "hittites", "israel", "judah",
        "parthian-empire", "persia", "phoenicia", "roman-empire", "seleucid-empire", "sumer",
    ];

    #[test]
    fn assign_color_keys_is_collision_free_for_the_real_curated_roster() {
        let keys = assign_color_keys(&REAL_CURATED_POLITY_IDS_SORTED);
        assert_eq!(keys.len(), REAL_CURATED_POLITY_IDS_SORTED.len());
        for &k in &keys {
            assert!((k as u32) < POLITY_TINT_COUNT, "{k} out of range");
        }
        let distinct: std::collections::HashSet<u8> = keys.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            REAL_CURATED_POLITY_IDS_SORTED.len(),
            "expected all {} real curated polities to get DISTINCT tints (16-slot palette, 14 polities); got {keys:?}",
            REAL_CURATED_POLITY_IDS_SORTED.len()
        );
    }

    #[test]
    fn assign_color_keys_is_deterministic_same_data_same_colors_every_run() {
        let a = assign_color_keys(&REAL_CURATED_POLITY_IDS_SORTED);
        let b = assign_color_keys(&REAL_CURATED_POLITY_IDS_SORTED);
        assert_eq!(a, b, "same sorted id list must produce the exact same assignment every call");
    }

    #[test]
    fn assign_color_keys_is_collision_free_for_any_roster_up_to_palette_size() {
        let mut ids: Vec<String> = (0..POLITY_TINT_COUNT).map(|i| format!("synthetic-polity-{i}")).collect();
        ids.sort();
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let keys = assign_color_keys(&id_refs);
        let distinct: std::collections::HashSet<u8> = keys.iter().copied().collect();
        assert_eq!(distinct.len(), POLITY_TINT_COUNT as usize, "expected every one of {} synthetic ids to get a distinct tint; got {keys:?}", POLITY_TINT_COUNT);
    }

    #[test]
    fn assign_color_keys_degrades_to_a_collision_rather_than_panicking_past_palette_size() {
        let mut ids: Vec<String> = (0..POLITY_TINT_COUNT + 3).map(|i| format!("synthetic-polity-{i}")).collect();
        ids.sort();
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let keys = assign_color_keys(&id_refs);
        assert_eq!(keys.len(), id_refs.len());
        for &k in &keys {
            assert!((k as u32) < POLITY_TINT_COUNT, "{k} out of range even past palette size");
        }
    }

    #[test]
    fn assign_color_keys_handles_empty_input() {
        let keys = assign_color_keys(&[]);
        assert!(keys.is_empty());
    }

    #[test]
    fn simple_square_is_simple() {
        let ring = vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)];
        assert!(ring_is_simple(&ring));
    }

    #[test]
    fn open_ring_without_closing_repeat_is_still_handled() {
        let ring = vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)];
        assert!(ring_is_simple(&ring));
    }

    #[test]
    fn bowtie_self_intersects() {
        let ring = vec![(0.0, 0.0), (1.0, 1.0), (1.0, 0.0), (0.0, 1.0), (0.0, 0.0)];
        assert!(!ring_is_simple(&ring), "bowtie must be detected as self-intersecting");
    }

    #[test]
    fn concave_but_simple_polygon_is_simple() {
        let ring = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (1.0, 1.0), (0.0, 2.0), (0.0, 0.0)];
        assert!(ring_is_simple(&ring));
    }

    #[test]
    fn a_vertex_touching_a_non_adjacent_edge_is_not_simple() {
        let ring = vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (2.0, 0.0), (0.0, 4.0), (0.0, 0.0)];
        assert!(!ring_is_simple(&ring), "closing edge crosses through vertex (2,0): {ring:?}");
    }

    #[test]
    fn real_curated_egypt_style_ring_is_simple() {
        let ring = vec![
            (31.6, 32.3), (31.5, 32.9), (31.3, 33.5), (31.0, 33.9), (30.9, 32.9), (31.1, 32.0),
            (30.6, 31.9), (30.0, 31.5), (29.5, 31.3), (29.0, 31.2), (28.5, 30.9), (28.0, 30.8),
            (27.5, 30.7), (27.0, 30.9), (26.5, 31.5), (26.0, 32.0), (25.5, 32.5), (24.5, 32.9),
            (24.0, 32.9), (23.9, 32.6), (24.5, 32.4), (25.0, 32.0), (25.5, 31.5), (26.0, 31.0),
            (26.5, 30.5), (27.0, 30.3), (27.5, 30.0), (28.0, 30.4), (28.5, 30.5), (29.0, 30.8),
            (29.5, 30.9), (30.0, 31.0), (30.5, 31.1), (31.0, 31.2), (31.6, 32.3),
        ];
        assert_eq!(ring.first(), ring.last(), "test fixture itself must be closed");
        assert!(ring_is_simple(&ring), "{ring:?}");
    }
}
