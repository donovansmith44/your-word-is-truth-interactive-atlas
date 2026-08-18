//! Parser for the OpenBible.info geocoding bundle's `ancient.jsonl` (JSON
//! Lines, one place per line; see `data/raw/README.md`). This is the only
//! one of the 5 geo files M1 needs: it carries place name, resolvable
//! coordinates (nested under `identifications[].resolutions[].lonlat`, a
//! `"lon,lat"` string — longitude first, reversed from the usual order),
//! and KJV verse links (`verses[].osis`, filtered to entries whose
//! `translations` include `"kjv"`).
//!
//! Divergence from `data/raw/README.md`'s worked example: the README's
//! prose describes `identifications[].score` as a plain int, but the real
//! file has it as a nested stats object (`{vote_average, vote_count, ...}`).
//! The per-modern-id int score genuinely used for ranking lives in
//! `modern_associations{modern_id}.score` instead (confirmed against the
//! real `ancient.jsonl`), which is what this parser ranks by; see the
//! correction added to `data/raw/README.md`.

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
    // Both optional: a resolution can be a `"special": "not_a_place"` dead-end
    // marker (e.g. real ancient.jsonl id `aaee94d` "Addar") carrying neither
    // field at all — see the correction note in data/raw/README.md.
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

/// Lowercase-kebab-case of `s`: runs of non-alphanumeric characters become a
/// single `-`, with no leading/trailing dash. `"Antioch of Pisidia"` ->
/// `"antioch-of-pisidia"`.
fn kebab(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = true; // suppress a leading dash
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

/// Picks one `(lon, lat)` per place: across all `identifications[].resolutions[]`,
/// the one whose `modern_basis_id` has the highest `modern_associations[..].score`
/// (defaulting unknown/missing associations to score 0); ties keep the first
/// encountered. Returns `None` if the record has no resolvable coordinate at all.
fn best_lonlat(raw: &RawAncient) -> Option<(f64, f64)> {
    let mut best: Option<(i64, &str)> = None; // (score, lonlat)
    for ident in &raw.identifications {
        for res in &ident.resolutions {
            // "special": "not_a_place" dead-end resolutions carry neither field: skip.
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

/// Parses `ancient.jsonl` into `Vec<Place>`. Records with no resolvable
/// coordinate are skipped (not a hard error — geocoding coverage is
/// inherently partial; `report.rs` surfaces gaps as a percentage instead).
/// Slugs are our own kebab-case of `friendly_id`, not the upstream
/// `url_slug`, so that same-named places collide and get `-2`, `-3` suffixes
/// in encounter order (independent of whatever slug OpenBible assigned).
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
            continue; // unresolvable place: no coordinate to plot
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
