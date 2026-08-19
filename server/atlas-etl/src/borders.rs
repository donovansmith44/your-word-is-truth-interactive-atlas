//! Parser + clip + simplify pipeline for `aourednik/historical-basemaps`
//! GeoJSON border snapshots (see `data/raw/README.md`'s Borders section for
//! the license, file list, and real structure this was written against).
//!
//! Hand-parsed via `serde_json::Value` rather than a typed `geojson` crate
//! dependency (sanctioned by the batch brief; the windows-gnu toolchain
//! constraint rules out crates needing cc/cmake/nasm, and this dataset's
//! `properties` schema isn't even uniform across files — see the README —
//! so an untyped pass-through is also simply the right fit, not just the
//! dependency-avoiding one).
//!
//! Pipeline: [`parse`] (raw GeoJSON text -> `Vec<RawFeature>`, normalizing
//! both `Polygon` and `MultiPolygon` geometries to the same
//! polygons-of-rings-of-points shape) -> [`clip`] (Sutherland-Hodgman
//! geometric clipping of every ring — exterior AND interior/hole rings
//! alike — against [`BIBLICAL_WORLD_BBOX`]) -> [`simplify_feature`]
//! (Douglas-Peucker per surviving ring, dropping rings that end up under 4
//! points, polygons left with zero rings, and features left with zero
//! polygons) -> [`to_geojson`] (reassemble into a compiled
//! `FeatureCollection`, always as `MultiPolygon`, coordinates rounded to 4
//! decimal places — about 11m, far finer than this app's world-scale
//! display needs, but a meaningful byte-size win over the source data's
//! full `f64` precision).
//!
//! [`clip`] is real ring-level geometric clipping: each ring is clipped in
//! turn against the bbox's 4 half-planes (west, east, south, north), which
//! is exact for a convex clip window like a bbox (Sutherland-Hodgman). It
//! runs BEFORE [`simplify_feature`] and never drops anything itself — a
//! ring clipped down to zero points, or a polygon left with zero rings, is
//! kept in place structurally and only actually removed by
//! `simplify_feature`'s existing size-based drop rules (ring < 4 points,
//! polygon with zero rings, feature with zero polygons), which already run
//! after simplification. That keeps exactly one "is this still real
//! geometry" check in the whole pipeline, applied after both clip and
//! simplify.
//!
//! Fix-round-1 correction: an earlier version of this module implemented
//! only feature-level bbox filtering (kept a whole feature, geometry
//! untouched, if any one of its rings merely overlapped the bbox) and
//! defended skipping real geometric clipping by attributing a specific
//! phrase to the batch brief that does not actually appear in any brief or
//! plan document in this repository — a fabricated quotation. That defense
//! is retracted; this module now performs the real ring-level clip
//! originally specified. See `.superpowers/sdd/2026-08-17-bible-atlas-m1/
//! batch-b-report.md`'s "Fix round 1" section for the correction, the new
//! per-snapshot clip statistics, and the measurement that motivated it
//! (real measurement found roughly 40% of compiled points landed outside
//! the render bbox as dead, unrenderable payload under the old
//! feature-level-only filter).

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde_json::Value;

/// The biblical-world clip/label bbox. Same constants
/// `client/wwwroot/js/map.js`'s `BIBLICAL_WORLD_BOUNDS` derives (documented
/// there: every compiled place's lat/lon extent + a flat 4-degree margin,
/// rounded to 1 decimal place) — reused here rather than independently
/// recomputed from `data/compiled/places.json` so the border data is
/// clipped to the EXACT box the map is locked to, not a second
/// independently-rounded approximation of it (a small drift between the
/// two would either clip borders the map could still pan to, or keep
/// borders the map can never show). Cross-reference: if `places.json`'s
/// extent ever changes enough to move that box, update both constants
/// together.
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

/// A parsed feature, geometry normalized to polygons-of-rings-of-(lon,lat)
/// regardless of whether the source was `Polygon` (wrapped as a
/// single-polygon list) or `MultiPolygon`. `properties` is passed through
/// verbatim (whatever object — or absence of one — the source feature had).
#[derive(Debug, Clone, PartialEq)]
pub struct RawFeature {
    pub properties: Value,
    pub polygons: Vec<Vec<Vec<(f64, f64)>>>,
}

/// Parses a GeoJSON `FeatureCollection` of `Polygon`/`MultiPolygon`
/// features. Fatal (per the batch brief: "unparseable snapshot = fatal")
/// on anything structurally wrong: invalid JSON, missing `features`,
/// missing/malformed geometry, or a geometry type other than
/// `Polygon`/`MultiPolygon`.
pub fn parse(input: &str) -> Result<Vec<RawFeature>> {
    let root: Value = serde_json::from_str(input).context("invalid JSON")?;
    let features = root.get("features").and_then(Value::as_array).context("missing top-level \"features\" array")?;

    let mut out = Vec::with_capacity(features.len());
    for (i, f) in features.iter().enumerate() {
        let properties = f.get("properties").cloned().unwrap_or(Value::Null);
        let geometry = f.get("geometry").with_context(|| format!("feature {i} missing \"geometry\""))?;
        let gtype =
            geometry.get("type").and_then(Value::as_str).with_context(|| format!("feature {i} geometry missing \"type\""))?;
        let coordinates = geometry.get("coordinates").with_context(|| format!("feature {i} geometry missing \"coordinates\""))?;

        let polygons = match gtype {
            "Polygon" => vec![parse_polygon(coordinates).with_context(|| format!("feature {i} Polygon"))?],
            "MultiPolygon" => parse_multipolygon(coordinates).with_context(|| format!("feature {i} MultiPolygon"))?,
            other => bail!("feature {i} has unsupported geometry type '{other}' (expected Polygon or MultiPolygon)"),
        };
        out.push(RawFeature { properties, polygons });
    }
    Ok(out)
}

fn parse_ring(v: &Value) -> Result<Vec<(f64, f64)>> {
    let arr = v.as_array().context("ring is not a JSON array")?;
    let mut ring = Vec::with_capacity(arr.len());
    for (i, pt) in arr.iter().enumerate() {
        let pair = pt.as_array().with_context(|| format!("point {i} is not a JSON array"))?;
        let lon = pair.first().and_then(Value::as_f64).with_context(|| format!("point {i} missing numeric lon"))?;
        let lat = pair.get(1).and_then(Value::as_f64).with_context(|| format!("point {i} missing numeric lat"))?;
        ring.push((lon, lat));
    }
    Ok(ring)
}

fn parse_polygon(v: &Value) -> Result<Vec<Vec<(f64, f64)>>> {
    let arr = v.as_array().context("polygon is not a JSON array of rings")?;
    arr.iter().enumerate().map(|(i, ring)| parse_ring(ring).with_context(|| format!("ring {i}"))).collect()
}

fn parse_multipolygon(v: &Value) -> Result<Vec<Vec<Vec<(f64, f64)>>>> {
    let arr = v.as_array().context("multipolygon is not a JSON array of polygons")?;
    arr.iter().enumerate().map(|(i, poly)| parse_polygon(poly).with_context(|| format!("polygon {i}"))).collect()
}

/// Point where segment `a`->`b` crosses the vertical line `x = at`. Only
/// ever invoked by [`clip_half_plane`] when `a`/`b` are on opposite sides
/// of that line (one inside, one outside), so `b.0 - a.0` is never zero in
/// practice; the `dx == 0.0` guard is defensive only (e.g. two
/// back-to-back duplicate points in messy source geometry) and falls back
/// to `b` rather than dividing by zero.
fn lerp_x(a: (f64, f64), b: (f64, f64), at: f64) -> (f64, f64) {
    let dx = b.0 - a.0;
    if dx == 0.0 {
        return b;
    }
    let t = (at - a.0) / dx;
    (at, a.1 + t * (b.1 - a.1))
}

/// Point where segment `a`->`b` crosses the horizontal line `y = at`. See
/// [`lerp_x`] (same reasoning, transposed).
fn lerp_y(a: (f64, f64), b: (f64, f64), at: f64) -> (f64, f64) {
    let dy = b.1 - a.1;
    if dy == 0.0 {
        return b;
    }
    let t = (at - a.1) / dy;
    (a.0 + t * (b.0 - a.0), at)
}

/// One Sutherland-Hodgman pass: clips a closed polygon (given as an OPEN
/// vertex cycle — no duplicated closing point; the edge from the last
/// vertex back to the first is implicit) against a single half-plane.
/// `inside` decides which side of that half-plane a point is on;
/// `intersect` computes where an edge crossing it lands. Standard
/// algorithm: walk the vertices, and for each edge (prev -> curr) emit the
/// boundary-crossing point whenever the edge crosses between outside and
/// inside, plus `curr` itself whenever `curr` is inside.
fn clip_half_plane(
    points: &[(f64, f64)],
    inside: impl Fn((f64, f64)) -> bool,
    intersect: impl Fn((f64, f64), (f64, f64)) -> (f64, f64),
) -> Vec<(f64, f64)> {
    if points.is_empty() {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(points.len() + 1);
    let mut prev = points[points.len() - 1];
    let mut prev_inside = inside(prev);
    for &curr in points {
        let curr_inside = inside(curr);
        if curr_inside {
            if !prev_inside {
                output.push(intersect(prev, curr));
            }
            output.push(curr);
        } else if prev_inside {
            output.push(intersect(prev, curr));
        }
        prev = curr;
        prev_inside = curr_inside;
    }
    output
}

/// Clips an open vertex cycle against `bbox` by running [`clip_half_plane`]
/// against each of its 4 edges in turn (west, east, south, north), each
/// pass consuming the previous pass's output. Correct because a bbox is
/// convex: clipping against the intersection of 4 half-planes one at a
/// time is equivalent to clipping against all 4 simultaneously at once.
/// Returns an open vertex list (possibly empty); callers close the ring
/// themselves.
fn sutherland_hodgman(points: &[(f64, f64)], bbox: &Bbox) -> Vec<(f64, f64)> {
    let west = clip_half_plane(points, |p| p.0 >= bbox.west, |a, b| lerp_x(a, b, bbox.west));
    let east = clip_half_plane(&west, |p| p.0 <= bbox.east, |a, b| lerp_x(a, b, bbox.east));
    let south = clip_half_plane(&east, |p| p.1 >= bbox.south, |a, b| lerp_y(a, b, bbox.south));
    clip_half_plane(&south, |p| p.1 <= bbox.north, |a, b| lerp_y(a, b, bbox.north))
}

/// Clips one ring — exterior or interior/hole, Sutherland-Hodgman treats
/// them identically — against `bbox`. GeoJSON rings repeat their first
/// point as their last (closed ring); that duplicate is stripped before
/// clipping (the algorithm's wraparound edge already implies the closure,
/// so keeping it would just add a harmless zero-length edge) and the
/// surviving output is re-closed (first point repeated as the last) before
/// returning. Returns an empty `Vec` — not a 1/2/3-point degenerate ring —
/// when the ring doesn't overlap `bbox` at all.
pub fn clip_ring(ring: &[(f64, f64)], bbox: &Bbox) -> Vec<(f64, f64)> {
    let open = if ring.len() >= 2 && ring[0] == ring[ring.len() - 1] { &ring[..ring.len() - 1] } else { ring };

    let clipped = sutherland_hodgman(open, bbox);
    if clipped.is_empty() {
        return Vec::new();
    }

    let mut closed = clipped;
    closed.push(closed[0]);
    closed
}

/// Clips every ring of every polygon in `feature` against `bbox` via
/// [`clip_ring`]. Purely geometric: nothing is dropped here. A ring/
/// polygon left with no surviving geometry is kept as an empty ring —
/// the ONE place that decides "is this still real geometry" is
/// [`simplify_feature`]'s existing drop rules, applied after simplify too
/// (see the module doc comment).
fn clip_feature(feature: RawFeature, bbox: &Bbox) -> RawFeature {
    let polygons =
        feature.polygons.into_iter().map(|poly| poly.into_iter().map(|ring| clip_ring(&ring, bbox)).collect()).collect();
    RawFeature { properties: feature.properties, polygons }
}

/// Clips every feature's geometry against `bbox` — see [`clip_feature`].
/// Feature count is always preserved (nothing is dropped at this stage);
/// see the module doc comment for where drops actually happen.
pub fn clip(features: Vec<RawFeature>, bbox: &Bbox) -> Vec<RawFeature> {
    features.into_iter().map(|f| clip_feature(f, bbox)).collect()
}

fn perp_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    if dx == 0.0 && dy == 0.0 {
        return ((p.0 - a.0).powi(2) + (p.1 - a.1).powi(2)).sqrt();
    }
    let t = ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy);
    let (px, py) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - px).powi(2) + (p.1 - py).powi(2)).sqrt()
}

/// Classic recursive Douglas-Peucker line simplification: keeps the two
/// endpoints always, recursively keeps whichever interior point is
/// furthest from the endpoint-to-endpoint line if that distance exceeds
/// `epsilon`, and otherwise collapses the whole segment to just its two
/// endpoints. Degrees-based `epsilon` (no projection) is appropriate here —
/// this app renders at world/regional scale, never surveying precision.
pub fn douglas_peucker(points: &[(f64, f64)], epsilon: f64) -> Vec<(f64, f64)> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let (a, b) = (points[0], points[points.len() - 1]);
    let mut max_dist = -1.0;
    let mut split = 0;
    for (i, &p) in points.iter().enumerate().take(points.len() - 1).skip(1) {
        let d = perp_distance(p, a, b);
        if d > max_dist {
            max_dist = d;
            split = i;
        }
    }

    if max_dist > epsilon {
        let mut left = douglas_peucker(&points[..=split], epsilon);
        let right = douglas_peucker(&points[split..], epsilon);
        left.pop(); // shared point at `split`; keep exactly one copy
        left.extend(right);
        left
    } else {
        vec![a, b]
    }
}

fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedFeature {
    pub properties: Value,
    pub polygons: Vec<Vec<Vec<(f64, f64)>>>,
}

/// Simplifies every ring of `feature` with [`douglas_peucker`], drops rings
/// that end up with fewer than 4 points (not a valid closed polygon ring —
/// a triangle needs 3 distinct vertices plus the closing repeat of the
/// first), drops polygons left with zero rings, and drops the whole
/// feature (returns `None`) if every polygon was dropped. Coordinates are
/// rounded to 4 decimal places (~11m) on the way out.
pub fn simplify_feature(feature: RawFeature, epsilon: f64) -> Option<SimplifiedFeature> {
    let mut polygons = Vec::new();
    for poly in feature.polygons {
        let mut rings = Vec::new();
        for ring in poly {
            let simplified = douglas_peucker(&ring, epsilon);
            if simplified.len() < 4 {
                continue;
            }
            rings.push(simplified.into_iter().map(|(lon, lat)| (round4(lon), round4(lat))).collect());
        }
        if !rings.is_empty() {
            polygons.push(rings);
        }
    }
    if polygons.is_empty() {
        None
    } else {
        Some(SimplifiedFeature { properties: feature.properties, polygons })
    }
}

/// Reassembles simplified features into a compiled GeoJSON
/// `FeatureCollection` `Value`. Always writes `MultiPolygon` geometry
/// regardless of the source's original `Polygon`/`MultiPolygon` shape —
/// both are valid, equivalent representations, and always emitting one
/// shape keeps the compiled schema uniform for the client to consume.
pub fn to_geojson(features: &[SimplifiedFeature]) -> Value {
    let features_json: Vec<Value> = features
        .iter()
        .map(|f| {
            let coordinates: Vec<Vec<Vec<[f64; 2]>>> = f
                .polygons
                .iter()
                .map(|poly| poly.iter().map(|ring| ring.iter().map(|&(lon, lat)| [lon, lat]).collect()).collect())
                .collect();
            serde_json::json!({
                "type": "Feature",
                "properties": f.properties,
                "geometry": { "type": "MultiPolygon", "coordinates": coordinates },
            })
        })
        .collect();
    serde_json::json!({ "type": "FeatureCollection", "features": features_json })
}

/// Parses the signed year a snapshot filename stem encodes: `"world_bc323"`
/// -> `-323`, `"world_100"` -> `100`. Fatal on anything else (not shaped
/// like `world_...`, a non-numeric year, or a zero year — the repo's own
/// file list never has `world_0.geojson`/`world_bc0.geojson`, so seeing one
/// would mean a naming assumption broke, not a value to silently coerce).
pub fn parse_snapshot_year(filename_stem: &str) -> Result<i32> {
    let rest = filename_stem
        .strip_prefix("world_")
        .with_context(|| format!("snapshot filename '{filename_stem}' does not start with 'world_'"))?;
    let (magnitude_str, sign) = match rest.strip_prefix("bc") {
        Some(digits) => (digits, -1),
        None => (rest, 1),
    };
    let magnitude: i32 = magnitude_str
        .parse()
        .with_context(|| format!("snapshot filename '{filename_stem}': '{magnitude_str}' is not a valid year number"))?;
    if magnitude == 0 {
        bail!("snapshot filename '{filename_stem}': year cannot be zero");
    }
    Ok(sign * magnitude)
}

/// Per-snapshot stats for `atlas-etl`'s report (feature counts through the
/// pipeline + point counts before/after simplification, so the report can
/// show a total point-reduction percentage).
#[derive(Debug, Clone, Default)]
pub struct SnapshotStats {
    pub year: i32,
    pub features_in: usize,
    pub features_kept: usize,
    pub points_before_simplify: usize,
    pub points_after_simplify: usize,
}

/// Runs the full parse -> clip -> simplify -> reassemble pipeline for one
/// snapshot's raw GeoJSON text. Fatal on an unparseable snapshot (bubbles
/// up `parse`'s error) or on zero features surviving clip+simplify (a bbox
/// bug — this dataset's biblical-world-bbox overlap was verified nonzero,
/// 18-37 features, on all 12 chosen real snapshots; see
/// `data/raw/README.md`). The check runs AFTER simplify, not right after
/// `clip`, because `clip` is now real ring-level geometric clipping that
/// never drops a feature itself (see the module doc comment) — whether any
/// geometry actually survived is only known once `simplify_feature` has
/// applied its drop rules.
pub fn process_snapshot(input: &str, year: i32, bbox: &Bbox, epsilon: f64) -> Result<(Value, SnapshotStats)> {
    let parsed = parse(input)?;
    let features_in = parsed.len();

    let clipped = clip(parsed, bbox);
    let points_before_simplify: usize =
        clipped.iter().flat_map(|f| f.polygons.iter()).flat_map(|poly| poly.iter()).map(|ring| ring.len()).sum();

    let simplified: Vec<SimplifiedFeature> = clipped.into_iter().filter_map(|f| simplify_feature(f, epsilon)).collect();
    if simplified.is_empty() {
        bail!("snapshot year {year}: zero features overlap the biblical-world bbox after clipping (bbox bug — every real chosen snapshot has 18+ overlapping features; see data/raw/README.md)");
    }

    let points_after_simplify: usize =
        simplified.iter().flat_map(|f| f.polygons.iter()).flat_map(|poly| poly.iter()).map(|ring| ring.len()).sum();

    let stats = SnapshotStats {
        year,
        features_in,
        features_kept: simplified.len(),
        points_before_simplify,
        points_after_simplify,
    };
    Ok((to_geojson(&simplified), stats))
}

/// Builds `borders-index.json`'s content (sorted signed years) from the
/// compiled map's keys.
pub fn sorted_years(borders: &BTreeMap<i32, Value>) -> Vec<i32> {
    borders.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/borders-sample.geojson");

    #[test]
    fn parse_reads_polygon_and_multipolygon() {
        let features = parse(FIXTURE).unwrap();
        // 4 features in the fixture: one MultiPolygon fully inside the
        // bbox, one Polygon straddling the bbox edge, one fully outside,
        // one MultiPolygon with a null `NAME` property.
        assert_eq!(features.len(), 4);
        assert_eq!(features[1].polygons.len(), 1, "a bare Polygon normalizes to one polygon");
        assert_eq!(features[0].polygons.len(), 1);
    }

    #[test]
    fn parse_rejects_unsupported_geometry_type() {
        let bad = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"Point","coordinates":[1,2]}}]}"#;
        let err = parse(bad).unwrap_err();
        assert!(err.to_string().contains("unsupported geometry type"), "{err}");
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(parse("not json").is_err());
    }

    #[test]
    fn clip_geometrically_clips_rings_rather_than_dropping_whole_features() {
        let features = parse(FIXTURE).unwrap();
        let bbox = Bbox { south: 7.6, north: 48.9, west: -10.9, east: 71.4 };
        let clipped = clip(features, &bbox);

        // clip() never drops features -- it clips geometry in place, so
        // the count is unchanged from parse (4); "Fully Outside" survives
        // as a feature with only empty rings, only removed later by
        // simplify_feature (see the module doc comment).
        assert_eq!(clipped.len(), 4, "{clipped:#?}");

        let by_name = |name: &str| {
            clipped.iter().find(|f| f.properties.get("NAME").and_then(Value::as_str) == Some(name)).unwrap()
        };

        let inside = by_name("Inside Land");
        assert_eq!(inside.polygons[0][0].len(), 6, "fully-inside ring is untouched by clipping: {:?}", inside.polygons[0][0]);

        let straddler = by_name("Straddler");
        let ring = &straddler.polygons[0][0];
        assert_eq!(ring.first(), ring.last(), "clipped ring must be closed: {ring:?}");
        assert!(
            ring.iter().any(|&(lon, _)| (lon - bbox.west).abs() < 1e-9),
            "straddling ring must gain a point on the west clip edge: {ring:?}"
        );

        let outside = by_name("Fully Outside");
        assert!(outside.polygons[0][0].is_empty(), "fully-outside ring clips to nothing: {:?}", outside.polygons[0][0]);
    }

    #[test]
    fn clip_bbox_nowhere_near_any_feature_yields_all_empty_rings() {
        let features = parse(FIXTURE).unwrap();
        // A bbox nowhere near any fixture feature.
        let bbox = Bbox { south: -80.0, north: -70.0, west: -170.0, east: -160.0 };
        let clipped = clip(features, &bbox);
        assert_eq!(clipped.len(), 4, "clip() never drops features, only clips geometry");
        for f in &clipped {
            for poly in &f.polygons {
                for ring in poly {
                    assert!(ring.is_empty(), "{f:#?}");
                }
            }
        }
    }

    // --- Fix round 1: Sutherland-Hodgman ring-level clip tests -----------

    #[test]
    fn clip_ring_straddling_edge_yields_closed_ring_with_points_on_the_boundary() {
        // A rectangle (lon -15..-5, lat 20..25) straddling the bbox's west
        // edge (west = -10.9) -- the bbox's other 3 edges don't touch it.
        let ring = vec![(-15.0, 20.0), (-15.0, 25.0), (-5.0, 25.0), (-5.0, 20.0), (-15.0, 20.0)];
        let clipped = clip_ring(&ring, &BIBLICAL_WORLD_BBOX);

        assert!(clipped.len() >= 4, "{clipped:?}");
        assert_eq!(clipped.first(), clipped.last(), "clipped ring must be closed");
        assert!(
            clipped.iter().any(|&(lon, _)| (lon - BIBLICAL_WORLD_BBOX.west).abs() < 1e-9),
            "expected a point exactly on the west clip edge: {clipped:?}"
        );
        assert!(clipped.iter().all(|&(lon, _)| lon >= BIBLICAL_WORLD_BBOX.west - 1e-9), "{clipped:?}");
    }

    #[test]
    fn clip_ring_fully_containing_bbox_yields_the_bbox_rectangle() {
        // A giant rectangle that fully contains BIBLICAL_WORLD_BBOX on every side.
        let ring = vec![(-90.0, -40.0), (-90.0, 80.0), (150.0, 80.0), (150.0, -40.0), (-90.0, -40.0)];
        let clipped = clip_ring(&ring, &BIBLICAL_WORLD_BBOX);

        assert_eq!(clipped.first(), clipped.last(), "clipped ring must be closed: {clipped:?}");
        let bbox = &BIBLICAL_WORLD_BBOX;
        let mut corners: Vec<(f64, f64)> = clipped[..clipped.len() - 1].to_vec();
        corners.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut expected =
            vec![(bbox.west, bbox.south), (bbox.west, bbox.north), (bbox.east, bbox.south), (bbox.east, bbox.north)];
        expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(corners, expected, "expected exactly the bbox's 4 corners: {clipped:?}");
    }

    #[test]
    fn clip_ring_fully_outside_drops_to_empty() {
        let ring = vec![(-150.0, -10.0), (-150.0, 0.0), (-140.0, 0.0), (-140.0, -10.0), (-150.0, -10.0)];
        assert!(clip_ring(&ring, &BIBLICAL_WORLD_BBOX).is_empty());
    }

    #[test]
    fn clip_feature_keeps_a_straddling_interior_ring_as_a_valid_hole() {
        // Exterior ring comfortably covers the bbox; interior ring (hole)
        // straddles the west edge, same rectangle as the straddling-edge
        // test above -- the hole must survive clipping as its own valid
        // ring, not be silently dropped or merged into the exterior.
        let exterior = vec![(-20.0, 0.0), (-20.0, 50.0), (80.0, 50.0), (80.0, 0.0), (-20.0, 0.0)];
        let hole = vec![(-15.0, 20.0), (-15.0, 25.0), (-5.0, 25.0), (-5.0, 20.0), (-15.0, 20.0)];
        let feature = RawFeature { properties: Value::Null, polygons: vec![vec![exterior, hole]] };

        let clipped = clip_feature(feature, &BIBLICAL_WORLD_BBOX);

        assert_eq!(clipped.polygons.len(), 1);
        assert_eq!(clipped.polygons[0].len(), 2, "exterior + hole both survive clipping: {:#?}", clipped.polygons[0]);

        let hole_ring = &clipped.polygons[0][1];
        assert!(hole_ring.len() >= 4, "{hole_ring:?}");
        assert_eq!(hole_ring.first(), hole_ring.last(), "clipped hole ring must be closed: {hole_ring:?}");
        assert!(
            hole_ring.iter().any(|&(lon, _)| (lon - BIBLICAL_WORLD_BBOX.west).abs() < 1e-9),
            "expected the clipped hole to touch the west edge: {hole_ring:?}"
        );
    }

    #[test]
    fn douglas_peucker_collapses_a_near_straight_line() {
        // A gently bowed line: the middle point is only ~0.001 deg off the
        // straight line from end to end, well under a 0.02 epsilon.
        let points = vec![(0.0, 0.0), (1.0, 0.0005), (2.0, 0.0)];
        let simplified = douglas_peucker(&points, 0.02);
        assert_eq!(simplified, vec![(0.0, 0.0), (2.0, 0.0)]);
    }

    #[test]
    fn douglas_peucker_keeps_a_real_corner() {
        // A sharp right-angle corner: the middle point is 1 full degree off
        // the straight line, far over a 0.02 epsilon -- must be kept.
        let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)];
        let simplified = douglas_peucker(&points, 0.02);
        assert_eq!(simplified, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)]);
    }

    #[test]
    fn douglas_peucker_short_input_passes_through() {
        assert_eq!(douglas_peucker(&[], 0.02), Vec::<(f64, f64)>::new());
        assert_eq!(douglas_peucker(&[(1.0, 2.0)], 0.02), vec![(1.0, 2.0)]);
        assert_eq!(douglas_peucker(&[(1.0, 2.0), (3.0, 4.0)], 0.02), vec![(1.0, 2.0), (3.0, 4.0)]);
    }

    #[test]
    fn simplify_feature_drops_degenerate_rings_but_keeps_the_feature_if_another_ring_survives() {
        // One ring collapses to a straight 2-point line (degenerate, < 4
        // points) under simplification; the OTHER ring in the same
        // MultiPolygon is a real square and survives -- the feature as a
        // whole must survive with just the square ring.
        let degenerate_ring = vec![(0.0, 0.0), (1.0, 0.0001), (2.0, 0.0002), (3.0, 0.0)]; // near-straight
        let square_ring = vec![(10.0, 10.0), (10.0, 11.0), (11.0, 11.0), (11.0, 10.0), (10.0, 10.0)];
        let feature = RawFeature { properties: Value::Null, polygons: vec![vec![degenerate_ring], vec![square_ring]] };

        let simplified = simplify_feature(feature, 0.02).expect("feature should survive via its second polygon");
        assert_eq!(simplified.polygons.len(), 1, "the degenerate ring's whole polygon must be dropped");
    }

    #[test]
    fn simplify_feature_drops_entirely_when_every_ring_degenerates() {
        let straight = vec![(0.0, 0.0), (1.0, 0.00001), (2.0, 0.0)];
        let feature = RawFeature { properties: Value::Null, polygons: vec![vec![straight]] };
        assert!(simplify_feature(feature, 0.02).is_none());
    }

    #[test]
    fn round_trip_reassembles_valid_geojson() {
        let square_ring = vec![(10.0, 10.0), (10.0, 11.0), (11.0, 11.0), (11.0, 10.0), (10.0, 10.0)];
        let feature =
            SimplifiedFeature { properties: serde_json::json!({"NAME": "Testland"}), polygons: vec![vec![square_ring]] };
        let value = to_geojson(&[feature]);
        assert_eq!(value["type"], "FeatureCollection");
        assert_eq!(value["features"][0]["type"], "Feature");
        assert_eq!(value["features"][0]["properties"]["NAME"], "Testland");
        assert_eq!(value["features"][0]["geometry"]["type"], "MultiPolygon");
        assert_eq!(value["features"][0]["geometry"]["coordinates"][0][0][0], serde_json::json!([10.0, 10.0]));
    }

    #[test]
    fn parse_snapshot_year_handles_bc_and_ad() {
        assert_eq!(parse_snapshot_year("world_bc323").unwrap(), -323);
        assert_eq!(parse_snapshot_year("world_bc4000").unwrap(), -4000);
        assert_eq!(parse_snapshot_year("world_100").unwrap(), 100);
        assert_eq!(parse_snapshot_year("world_1").unwrap(), 1);
    }

    #[test]
    fn parse_snapshot_year_rejects_bad_shapes() {
        assert!(parse_snapshot_year("nope_100").is_err());
        assert!(parse_snapshot_year("world_bc0").is_err());
        assert!(parse_snapshot_year("world_0").is_err());
        assert!(parse_snapshot_year("world_abc").is_err());
    }

    #[test]
    fn process_snapshot_end_to_end_on_the_fixture() {
        let bbox = Bbox { south: 7.6, north: 48.9, west: -10.9, east: 71.4 };
        let (geojson, stats) = process_snapshot(FIXTURE, -323, &bbox, 0.02).unwrap();
        assert_eq!(stats.year, -323);
        assert_eq!(stats.features_in, 4);
        assert_eq!(stats.features_kept, 3);
        assert!(stats.points_after_simplify <= stats.points_before_simplify);
        assert_eq!(geojson["type"], "FeatureCollection");
        assert_eq!(geojson["features"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn process_snapshot_zero_features_after_clip_is_fatal() {
        let bbox = Bbox { south: -80.0, north: -70.0, west: -170.0, east: -160.0 };
        let err = process_snapshot(FIXTURE, -323, &bbox, 0.02).unwrap_err();
        assert!(err.to_string().contains("zero features"), "{err}");
    }

    #[test]
    fn process_snapshot_unparseable_is_fatal() {
        assert!(process_snapshot("not json", -323, &BIBLICAL_WORLD_BBOX, 0.02).is_err());
    }
}
