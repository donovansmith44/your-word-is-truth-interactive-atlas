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
//! polygons-of-rings-of-points shape) -> [`clip`] (drop features with no
//! ring overlapping the bbox at all) -> [`simplify_feature`] (Douglas-
//! Peucker per surviving ring, dropping rings that end up under 4 points)
//! -> [`to_geojson`] (reassemble into a compiled `FeatureCollection`,
//! always as `MultiPolygon`, coordinates rounded to 4 decimal places —
//! about 11m, far finer than this app's world-scale display needs, but a
//! meaningful byte-size win over the source data's full `f64` precision).
//!
//! "CLIP" here is feature-level bbox filtering (drop a feature only if
//! EVERY ring of it misses the bbox entirely), not ring-level geometric
//! clipping (cutting a surviving polygon's own edge at the bbox boundary,
//! e.g. Sutherland-Hodgman). Deliberate: (1) the brief's own wording is
//! "dropping features entirely outside", which is exactly a feature-level
//! filter; (2) the client's Leaflet map is ALREADY hard-locked to the same
//! bbox via `maxBounds` + `overflow:hidden` (see `map.js`), so any surviving
//! feature's out-of-bbox extent is invisibly cropped by the browser anyway
//! — ring-level clipping would save bytes but change nothing on screen;
//! (3) real measurement (see the batch report) showed kept-but-unclipped
//! features already compile to a very reasonable ~70-90KB per snapshot
//! after simplification + rounding, so the extra ~100-150 line
//! polygon-clipping implementation (correctly handling holes/multi-ring
//! features) wasn't worth its complexity for a local sketch app.

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

fn ring_overlaps_bbox(ring: &[(f64, f64)], bbox: &Bbox) -> bool {
    let (mut min_lon, mut max_lon) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_lat, mut max_lat) = (f64::INFINITY, f64::NEG_INFINITY);
    for &(lon, lat) in ring {
        min_lon = min_lon.min(lon);
        max_lon = max_lon.max(lon);
        min_lat = min_lat.min(lat);
        max_lat = max_lat.max(lat);
    }
    min_lon <= bbox.east && max_lon >= bbox.west && min_lat <= bbox.north && max_lat >= bbox.south
}

/// Keeps a feature iff at least one ring of at least one of its polygons
/// overlaps `bbox` — see the module doc comment for why this is
/// feature-level filtering, not ring-level geometric clipping.
pub fn clip(features: Vec<RawFeature>, bbox: &Bbox) -> Vec<RawFeature> {
    features.into_iter().filter(|f| f.polygons.iter().any(|poly| poly.iter().any(|ring| ring_overlaps_bbox(ring, bbox)))).collect()
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
/// up `parse`'s error) or on zero features surviving `clip` (a bbox bug —
/// this dataset's biblical-world-bbox overlap was verified nonzero, 18-37
/// features, on all 12 chosen real snapshots; see `data/raw/README.md`).
pub fn process_snapshot(input: &str, year: i32, bbox: &Bbox, epsilon: f64) -> Result<(Value, SnapshotStats)> {
    let parsed = parse(input)?;
    let features_in = parsed.len();

    let clipped = clip(parsed, bbox);
    if clipped.is_empty() {
        bail!("snapshot year {year}: zero features overlap the biblical-world bbox after clipping (bbox bug — every real chosen snapshot has 18+ overlapping features; see data/raw/README.md)");
    }

    let points_before_simplify: usize =
        clipped.iter().flat_map(|f| f.polygons.iter()).flat_map(|poly| poly.iter()).map(|ring| ring.len()).sum();

    let simplified: Vec<SimplifiedFeature> = clipped.into_iter().filter_map(|f| simplify_feature(f, epsilon)).collect();
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
    fn clip_drops_fully_outside_keeps_inside_and_straddling() {
        let features = parse(FIXTURE).unwrap();
        let bbox = Bbox { south: 7.6, north: 48.9, west: -10.9, east: 71.4 };
        let kept = clip(features, &bbox);
        // The fully-outside feature (a box far in the Pacific) is dropped;
        // the inside, straddling, and null-named features survive.
        assert_eq!(kept.len(), 3, "{kept:#?}");
        assert!(kept.iter().all(|f| f.properties.get("NAME").and_then(Value::as_str) != Some("Fully Outside")));
    }

    #[test]
    fn clip_all_outside_yields_empty() {
        let features = parse(FIXTURE).unwrap();
        // A bbox nowhere near any fixture feature.
        let bbox = Bbox { south: -80.0, north: -70.0, west: -170.0, east: -160.0 };
        assert!(clip(features, &bbox).is_empty());
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
