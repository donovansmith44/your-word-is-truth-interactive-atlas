//! Batch B2 ("borders v2, the cartographer's edition"): geometry helpers for
//! this project's own curated, hand-authored polity borders
//! (`data/curated/polities/{id}.toml`; see `atlas_core::data::Polity`'s own
//! doc comment for the data model and `LICENSES.md` for the CC0 dedication
//! and the public/general-knowledge sources consulted while drawing them).
//!
//! Replaces the Batch B/C2 snapshot-year GeoJSON pipeline (`borders.rs`,
//! deleted whole -- "the 12 snapshot-year files, their ETL path, the
//! nearest-snapshot logic... ALL RETIRE" per the batch brief). That module's
//! parse/clip/Douglas-Peucker pipeline existed to shrink a fetched/traced
//! GeoJSON dataset down to this app's display needs; a hand-authored ring is
//! already coarse by construction (the batch brief: "30-80 points per ring",
//! "do not try to get too fancy with mathematics") and needs neither
//! clipping (Leaflet simply never pans to where an out-of-frame vertex would
//! render, so an unclipped ring costs nothing but a few unused bytes) nor
//! simplification (there is nothing to simplify away from 30-80 points that
//! were placed by hand in the first place). Only two pieces of that module
//! survive, reused rather than re-derived:
//! - [`BIBLICAL_WORLD_BBOX`]: the exact same constant (same value, same
//!   provenance -- every compiled place's own lat/lon extent, padded 4
//!   degrees -- see `client/wwwroot/js/map.js`'s `BIBLICAL_WORLD_BOUNDS`,
//!   which this must keep matching). Curated polity coordinates are
//!   validated to fall inside it (`validate::run_polities`) the same way
//!   curated landmark coordinates already are (`validate::run_landmarks`),
//!   so a wildly-wrong vertex (a typo, a transposed lat/lon) fails ETL
//!   loudly instead of silently drawing a polygon nobody can ever pan to.
//! - The CCW-based segment-crossing "is this ring simple" test: the exact
//!   algorithm Batch L's own ring-simplicity checker used (a small,
//!   throwaway Node script at the time, never committed to this repo -- see
//!   `.superpowers/sdd/2026-08-17-bible-atlas-m1/batch-l-report.md`'s "Fix
//!   round 1" section for its own description: "a small Node script,
//!   CCW-based segment-crossing test over each ring's non-adjacent edges").
//!   [`ring_is_simple`] below is that same standard, textbook algorithm
//!   (Cormen et al.'s segment-intersection test via orientation/CCW), now
//!   written once in Rust so it can be a permanent, automatic ETL gate
//!   instead of a one-off manual script a curator has to remember to run --
//!   "REUSE... do not write new geometry math" per this batch's own brief.

use atlas_core::time::Year;

/// The biblical-world clip/label bbox. Same constants
/// `client/wwwroot/js/map.js`'s `BIBLICAL_WORLD_BOUNDS` derives (documented
/// there: every compiled place's lat/lon extent + a flat 4-degree margin,
/// rounded to 1 decimal place) -- carried over byte-for-byte from the
/// retired `borders.rs`'s own constant of the same name (see this module's
/// header comment). Cross-reference: if `places.json`'s extent ever changes
/// enough to move that box, update both constants together.
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

/// This app's atlas span -- same bound `validate::run`/`run_place_history`
/// already enforce elsewhere (there is no year zero; `-4004` is Ussher's
/// creation year, `100` is the latest curated event in this atlas).
pub const ATLAS_START_YEAR: Year = -4004;
pub const ATLAS_END_YEAR: Year = 100;

/// Number of hues in map.js's own `POLITY_TINTS` palette. Fix round 1 (M1
/// -- review finding B2: 11 of 14 polities collided into shared hash
/// buckets under the old 8-hue palette, including geographically/
/// temporally overlapping pairs -- Judah/Neo-Assyrian-Empire, Judah/
/// Achaemenid-Persia). Widened 8 -> 16 (within the design authority's own
/// "12-16" suggestion) -- see map.js's own `POLITY_TINTS` comment for the
/// actual added hex values. 16 stays `>=` today's 14 curated polities, with
/// two slots of headroom before [`assign_color_keys`] is forced to accept a
/// collision again.
pub const POLITY_TINT_COUNT: u32 = 16;

/// A polity's DETERMINISTIC STARTING probe position in the tint palette,
/// hashed from its stable `id` alone -- unchanged algorithm from this
/// crate's original (pre-fix-round) `color_key` function (sum of
/// normalized Unicode code-unit values, mod the palette length): a polity
/// id is already a clean, hand-authored kebab-case slug (`"egypt"`,
/// `"roman-empire"`) with no internal whitespace to collapse, so trim +
/// lowercase is the whole of what's needed before summing. `id.chars()`
/// (not `.bytes()`) mirrors the client's own (retired) `charCodeAt`-based
/// hash -- both sum Unicode code-unit values, and every id in this app's
/// curated set is plain ASCII, so the two only ever agree in practice;
/// `.chars()` keeps this correct in principle for a hypothetical non-ASCII
/// id too, not merely correct by accident of today's data.
///
/// This is a SEED, not the final answer -- see [`assign_color_keys`], which
/// resolves collisions between seeds computed from the same roster.
fn color_seed(id: &str) -> u8 {
    let normalized = id.trim().to_lowercase();
    let sum: u32 = normalized.chars().map(|c| c as u32).sum();
    (sum % POLITY_TINT_COUNT) as u8
}

/// Fix round 1 (M1 -- review finding B2: "assign deterministically and
/// collision-free: extend the palette... assign by SORTED polity id with
/// linear probing computed from the full polity list at load -- same data,
/// same colors, every run; no two polities share a tint while count <=
/// palette size"). REFINES Batch C2's own client-side `polityFillColor`
/// (retired) and this crate's own original `color_key` (retired as a public
/// function, survives internally as [`color_seed`]) -- both hashed a SINGLE
/// id in isolation, with no visibility into what else was in play, so
/// collisions between ANY two ids sharing a bucket were silently accepted.
/// This function sees every id in the roster AT ONCE and actively avoids
/// re-using a bucket another id in the SAME call already claimed.
///
/// `sorted_ids` MUST already be sorted by the caller (this function does
/// not re-sort -- `process_polities`, its sole caller in
/// `server/atlas-etl/src/main.rs`, already reads its curated directory in
/// filename/id order for its own report, so re-sorting here would just be a
/// second, possibly-inconsistent source of truth for the same ordering).
/// Determinism follows directly from that fixed order plus a purely
/// deterministic probe: hashing the same id set, in the same order, always
/// walks the same sequence of "is this bucket free yet" checks and lands on
/// the same answer -- no `HashMap` iteration order, no randomness, nothing
/// time-of-day-dependent ("same data, same colors, every run").
///
/// Algorithm: for each id, in the given order, start at that id's own
/// [`color_seed`] and LINEAR-PROBE forward (seed, seed+1, seed+2, ...,
/// wrapping mod [`POLITY_TINT_COUNT`]) for the first bucket no EARLIER id
/// in this same call has already claimed; mark it claimed; move on. Because
/// a full sweep of all `POLITY_TINT_COUNT` buckets can only fail to find a
/// free one once every bucket is already claimed -- which requires at
/// least `POLITY_TINT_COUNT` ids to have gone before -- every id is
/// guaranteed its own distinct bucket as long as `sorted_ids.len() <=
/// POLITY_TINT_COUNT` (true for today's 14 curated polities against a
/// 16-slot palette). Past that point (not true today, but this function
/// must degrade rather than panic or spin forever if it ever becomes true)
/// a probe that finds no free bucket after a full sweep just falls back to
/// its own plain seed, accepting a collision -- the same graceful
/// degradation the palette was always documented to have once id count
/// exceeds palette size (see the old `color_key_differs_for_most_distinct_ids`
/// test's own comment, kept below under its fix-round name).
pub fn assign_color_keys(sorted_ids: &[&str]) -> Vec<u8> {
    let mut used = vec![false; POLITY_TINT_COUNT as usize];
    let mut out = Vec::with_capacity(sorted_ids.len());
    for id in sorted_ids {
        let seed = color_seed(id);
        let mut assigned = seed; // fallback if every bucket is already taken (see this function's own doc comment)
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
    // A small epsilon (not exact zero) absorbs floating-point noise from
    // hand-typed decimal coordinates without so loosening the test that a
    // real, visually-obvious crossing reads as "merely collinear" -- the
    // rings this validates are country-scale (degrees, not survey-grade
    // precision), so 1e-9 is many orders of magnitude below anything a
    // curator could type as a deliberate near-miss.
    if val.abs() < 1e-9 {
        0
    } else if val > 0.0 {
        1
    } else {
        2
    }
}

/// Given `p`, `q`, `r` already known collinear (`orientation(p, q, r) == 0`),
/// is `q` within `p`'s and `r`'s own bounding box -- i.e. does `q` sit ON
/// segment `p`-`r`, not merely on the infinite line through it.
fn on_segment(p: (f64, f64), q: (f64, f64), r: (f64, f64)) -> bool {
    q.0 <= p.0.max(r.0) && q.0 >= p.0.min(r.0) && q.1 <= p.1.max(r.1) && q.1 >= p.1.min(r.1)
}

/// The standard CCW/orientation-based segment-intersection test (Cormen et
/// al., *Introduction to Algorithms*; see this module's header comment for
/// why this is a REUSE of Batch L's own checker, not new geometry math):
/// true when open segments `p1`-`p2` and `p3`-`p4` cross, including the
/// degenerate collinear-overlap cases.
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

/// Is `ring` a simple polygon (no two of its own non-adjacent edges cross)?
/// `ring` may be given either CLOSED (first point repeated as the last, this
/// app's own curated convention -- see `atlas_core::data::PolityEra::rings`'s
/// doc comment) or open; the repeated closing point, if present, is stripped
/// before the edge walk so it is never mistaken for a real, separate vertex.
/// A ring with fewer than 3 distinct points is vacuously simple (nothing to
/// cross) -- `validate::run_polities` separately hard-fails a ring that
/// small for not being closed/a real polygon in the first place, so this
/// function itself never needs to reject it.
///
/// O(n^2) over the ring's own edges -- entirely adequate at this app's own
/// scale (30-80 points per the brief's own recognizability band; the
/// largest curated ring in this batch's own roster is under 90), and exactly
/// matches what a script re-run by hand after every edit would have done.
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
            // Edges i and j are adjacent (share exactly one endpoint, which
            // is expected and never a "crossing") when edge j starts where
            // edge i ends (j == i+1), or -- the one wraparound case a
            // j-in-(i+1)..n loop can still reach -- when edge i is the ring's
            // OWN closing edge back to edge j's start (i == 0 && j == n-1).
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
        // Not a claim of no collisions among raw SEEDS (16 buckets, and
        // nothing stops two different ids hashing to the same one) -- just
        // a smoke check that the hash isn't degenerately constant across a
        // handful of real ids from this batch's own roster. Collision-
        // FREEDOM among real assignments is assign_color_keys's own job,
        // tested below -- this test is about color_seed alone.
        let ids = ["egypt", "assyria", "babylon", "judah", "israel", "roman-empire"];
        let keys: std::collections::HashSet<u8> = ids.iter().map(|id| color_seed(id)).collect();
        assert!(keys.len() > 1, "expected some variety across {ids:?}, got all-same");
    }

    // --- assign_color_keys (fix round 1, M1) --------------------------------

    /// The real 14 curated polity ids, already sorted -- matches
    /// `data/curated/polities/*.toml`'s own filenames (and therefore the
    /// exact order `process_polities`, main.rs, reads and feeds this
    /// function in production) at the time of this fix round. If a future
    /// batch adds/removes a polity file, this list -- and the "collision-
    /// free" assertion below -- should be updated to match.
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
        // General property, not just today's real roster: ANY set of
        // POLITY_TINT_COUNT distinct synthetic ids, sorted, gets
        // POLITY_TINT_COUNT distinct tints -- the exact guarantee the fix
        // round's own brief asks for ("no two polities share a tint while
        // count <= palette size").
        let mut ids: Vec<String> = (0..POLITY_TINT_COUNT).map(|i| format!("synthetic-polity-{i}")).collect();
        ids.sort();
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let keys = assign_color_keys(&id_refs);
        let distinct: std::collections::HashSet<u8> = keys.iter().copied().collect();
        assert_eq!(distinct.len(), POLITY_TINT_COUNT as usize, "expected every one of {} synthetic ids to get a distinct tint; got {keys:?}", POLITY_TINT_COUNT);
    }

    #[test]
    fn assign_color_keys_degrades_to_a_collision_rather_than_panicking_past_palette_size() {
        // Past the palette size, collisions become unavoidable -- this must
        // degrade gracefully (no panic, no infinite loop), not a claim of
        // continued collision-freedom.
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

    // --- ring_is_simple ----------------------------------------------------

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
        // A classic "bowtie": (0,0)->(1,1)->(1,0)->(0,1)->(0,0). The first
        // edge (0,0)-(1,1) and the third edge (1,0)-(0,1) cross in the
        // middle -- neither adjacent nor sharing an endpoint.
        let ring = vec![(0.0, 0.0), (1.0, 1.0), (1.0, 0.0), (0.0, 1.0), (0.0, 0.0)];
        assert!(!ring_is_simple(&ring), "bowtie must be detected as self-intersecting");
    }

    #[test]
    fn concave_but_simple_polygon_is_simple() {
        // A plain "arrow"/chevron shape -- concave (a reflex vertex) but
        // never crosses itself. Ring simplicity must not be confused with
        // convexity.
        let ring = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (1.0, 1.0), (0.0, 2.0), (0.0, 0.0)];
        assert!(ring_is_simple(&ring));
    }

    #[test]
    fn a_vertex_touching_a_non_adjacent_edge_is_not_simple() {
        // A "figure-6"/lollipop shape where the closing edge passes exactly
        // through an earlier, non-adjacent vertex -- a degenerate but real
        // self-intersection (the collinear on_segment branch).
        let ring = vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (2.0, 0.0), (0.0, 4.0), (0.0, 0.0)];
        assert!(!ring_is_simple(&ring), "closing edge crosses through vertex (2,0): {ring:?}");
    }

    #[test]
    fn real_curated_egypt_style_ring_is_simple() {
        // A coarse Nile-valley-plus-delta shape in the SAME spirit as this
        // batch's own curated egypt.toml (hugs a river/coast rather than
        // being a blob) -- a real-shape-shaped smoke test, not just tiny
        // synthetic squares.
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
