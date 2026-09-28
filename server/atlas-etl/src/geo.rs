//! Parser for the OpenBible geocoding bundle's JSON Lines file. A coordinate is a `"lon,lat"` string --
//! longitude FIRST, reversed from the usual order -- and the int score used for ranking lives in
//! `modern_associations{modern_id}.score`, not in `identifications[].score`, which is a stats object.

use std::collections::HashMap;

use anyhow::{Context, Result};
use atlas_core::data::Place;
use serde::Deserialize;

use crate::osis;

#[derive(Deserialize)]
struct RawAncient {
    friendly_id: String,
    #[serde(default)]
    identifications: Vec<RawIdentification>,
    #[serde(default)]
    modern_associations: HashMap<String, RawModernAssoc>,
    #[serde(default)]
    verses: Vec<RawVerseLink>,
}

#[derive(Deserialize)]
struct RawIdentification {
    #[serde(default)]
    resolutions: Vec<RawResolution>,
}

#[derive(Deserialize)]
struct RawResolution {
    // Both optional: a resolution can be a `"special": "not_a_place"` dead-end marker that carries
    // neither field at all.
    #[serde(default)]
    lonlat: Option<String>,
    #[serde(default)]
    modern_basis_id: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawModernAssoc {
    #[serde(default)]
    score: i64,
}

#[derive(Deserialize)]
struct RawVerseLink {
    osis: String,
    #[serde(default)]
    translations: Vec<String>,
}

/// Lowercase-kebab-case: runs of non-alphanumeric characters become a single `-`, with no leading or
/// trailing dash.
fn kebab(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}

/// One `(lon, lat)` per place: across every resolution, the one whose `modern_basis_id` has the highest
/// association score, unknown or missing associations counting as 0, ties keeping the first encountered.
/// `None` when the record has no resolvable coordinate at all.
fn best_lonlat(raw: &RawAncient) -> Option<(f64, f64)> {
    let mut best: Option<(i64, &str)> = None;
    for ident in &raw.identifications {
        for res in &ident.resolutions {
            let (Some(lonlat), Some(modern_basis_id)) = (res.lonlat.as_deref(), res.modern_basis_id.as_deref()) else {
                continue;
            };
            let score = raw.modern_associations.get(modern_basis_id).map(|m| m.score).unwrap_or(0);
            let better = match best {
                None => true,
                Some((best_score, _)) => score > best_score,
            };
            if better {
                best = Some((score, lonlat));
            }
        }
    }
    let (_, lonlat) = best?;
    let (lon_s, lat_s) = lonlat.split_once(',')?;
    let lon: f64 = lon_s.trim().parse().ok()?;
    let lat: f64 = lat_s.trim().parse().ok()?;
    Some((lon, lat))
}

/// A record with no resolvable coordinate is skipped rather than failing: geocoding coverage is
/// inherently partial, and the report surfaces the gap as a percentage. Slugs are our own kebab-case of
/// `friendly_id`, not the upstream slug, so same-named places collide and take `-2`, `-3` suffixes in
/// encounter order.
pub fn parse(input: &str) -> Result<Vec<Place>> {
    let mut places = Vec::new();
    let mut slug_counts: HashMap<String, u32> = HashMap::new();

    for (i, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let raw: RawAncient =
            serde_json::from_str(line).with_context(|| format!("geo/ancient.jsonl line {} is not valid JSON", i + 1))?;

        let Some((lon, lat)) = best_lonlat(&raw) else {
            continue;
        };

        let base = kebab(&raw.friendly_id);
        let n = slug_counts.entry(base.clone()).or_insert(0);
        *n += 1;
        let id = if *n == 1 { base } else { format!("{base}-{n}") };

        let mut verse_links = Vec::new();
        for v in &raw.verses {
            if !v.translations.iter().any(|t| t == "kjv") {
                continue;
            }
            if let Some(vid) = osis::parse_verse(&v.osis) {
                let canon = osis::canonical(&vid);
                if !verse_links.contains(&canon) {
                    verse_links.push(canon);
                }
            }
        }

        places.push(Place { id, name: raw.friendly_id, lat, lon, verse_links });
    }

    Ok(places)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_case_examples() {
        assert_eq!(kebab("Antioch of Pisidia"), "antioch-of-pisidia");
        assert_eq!(kebab("Aroer (in Ammon)"), "aroer-in-ammon");
        assert_eq!(kebab("Abana"), "abana");
    }
}
