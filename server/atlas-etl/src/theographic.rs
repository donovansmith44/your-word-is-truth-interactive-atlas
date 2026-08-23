//! Joins Theographic Bible Metadata's Airtable-style `events.json`,
//! `places.json`, and `verses.json` (see `data/raw/README.md`) into compiled
//! `Event`s. Record cross-references are 14-char `rec...` Airtable ids.
//!
//! Adapted return shape: the parser contract in the task brief lists
//! `(Vec<Event>, TheoStats)`, but per its own instruction ("unmatched name
//! => create a new Place ... return those too") a third element is needed;
//! this module returns `(Vec<Event>, Vec<Place>, TheoStats)`.
//!
//! Dates: `fields.startDate` is documented as an *astronomical* year number
//! (`"-4003"` == `4004 BC`; negate and add 1 to get the BC year). We convert
//! straight to atlas-core's historical (no-year-zero) convention: astronomical
//! year `A` maps to historical year `A` when `A >= 1`, or `A - 1` when
//! `A <= 0`. Also accepts a literal `"NNNN BC"` suffix form (already a
//! calendar BC year, no astronomical shift) and the ISO-ish `"0030-05-01"`
//! form actually seen in the real data (extracts the leading year and applies
//! the same astronomical conversion). Empty/unparseable/all of these still
//! landing on year zero => the event is dropped into `TheoStats.undated`
//! (not fatal).

use std::collections::HashMap;

use anyhow::{Context, Result};
use atlas_core::data::{Event, Place};
use atlas_core::time::TimeRange;
use serde::Deserialize;

use crate::osis;

#[derive(Deserialize)]
struct Record<F> {
    id: String,
    fields: F,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PlaceFields {
    #[serde(default)]
    display_title: Option<String>,
    #[serde(default)]
    kjv_name: Option<String>,
    #[serde(default)]
    latitude: Option<String>,
    #[serde(default)]
    longitude: Option<String>,
    #[serde(default)]
    slug: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct VerseFields {
    #[serde(default)]
    osis_ref: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct EventFields {
    #[serde(default)]
    title: String,
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    locations: Vec<String>, // missing key -> empty (per README, absent not [])
    #[serde(default)]
    verses: Vec<String>,
    #[serde(default, rename = "eventID")]
    event_id: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct TheoStats {
    pub total: usize,
    pub dated: usize,
    pub undated: usize,
    /// Dated events left with zero resolved places (kept, just unanchored).
    pub no_place: usize,
    /// Synthesized `Place`s created from Theographic's own lat/lon because
    /// the linked place name had no match in the geo-derived place set.
    pub new_places: usize,
}

/// Converts a Theographic `startDate` string into atlas-core's historical
/// (no-year-zero) year convention. Returns `None` for empty/unparseable
/// input or the (should-be-impossible) case of landing on year zero.
///
/// `pub(crate)` (Batch P): `atlas_etl::people::parse_people` reuses this
/// verbatim for `birthYear`/`deathYear` -- the SAME astronomical-year
/// convention Theographic uses for both its Events and People tables, so a
/// second, independently-authored copy would only risk drifting from this
/// one, never add real independence (unlike `fidelity.rs`'s OWN
/// deliberately-independent re-derivation, which exists specifically to
/// catch bugs IN shared code -- this is not that kind of law).
pub(crate) fn parse_theo_year(raw: &str) -> Option<i32> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    // Literal "NNNN BC" form: already a calendar BC year, no astronomical shift.
    let upper = s.to_ascii_uppercase();
    if let Some(prefix) = upper.strip_suffix("BC") {
        let mag: i32 = prefix.trim().parse().ok()?;
        return if mag == 0 { None } else { Some(-mag) };
    }

    // Plain astronomical integer ("-4003") or ISO-ish ("0030-05-01" /
    // "-1446-04-01"): take the leading signed integer as the astronomical year.
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let digits_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if digits_end == 0 {
        return None;
    }
    let mag: i32 = rest[..digits_end].parse().ok()?;
    let astro = if neg { -mag } else { mag };
    let historical = if astro <= 0 { astro - 1 } else { astro };
    if historical == 0 {
        None
    } else {
        Some(historical)
    }
}

/// Joins events -> places -> our place slugs (by case-insensitive name match
/// against `place_slug_by_name`, whose keys must already be lowercased) and
/// events -> verses -> canonical verse ids. An event place name with no geo
/// match gets a brand-new `Place` synthesized from Theographic's own
/// `latitude`/`longitude` (same lat/lon order as normal, unlike the
/// openbible `lon,lat` convention) — those new places are returned
/// alongside the events so the caller can merge them into the compiled set.
pub fn parse_events(
    places_json: &str,
    verses_json: &str,
    events_json: &str,
    place_slug_by_name: &HashMap<String, String>,
) -> Result<(Vec<Event>, Vec<Place>, TheoStats)> {
    let places: Vec<Record<PlaceFields>> =
        serde_json::from_str(places_json).context("theographic places.json is not valid JSON")?;
    let verses: Vec<Record<VerseFields>> =
        serde_json::from_str(verses_json).context("theographic verses.json is not valid JSON")?;
    let events: Vec<Record<EventFields>> =
        serde_json::from_str(events_json).context("theographic events.json is not valid JSON")?;

    let place_by_id: HashMap<&str, &PlaceFields> = places.iter().map(|r| (r.id.as_str(), &r.fields)).collect();
    let verse_osis_by_id: HashMap<&str, &str> =
        verses.iter().filter_map(|r| r.fields.osis_ref.as_deref().map(|o| (r.id.as_str(), o))).collect();

    // theographic place record id -> our compiled place id (geo match or newly synthesized).
    let mut resolved_place_cache: HashMap<String, String> = HashMap::new();
    let mut new_places: Vec<Place> = Vec::new();
    let mut new_place_ids_used: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut out_events = Vec::new();
    let mut stats = TheoStats::default();

    for rec in &events {
        stats.total += 1;
        let f = &rec.fields;

        let Some(year) = f.start_date.as_deref().and_then(parse_theo_year) else {
            stats.undated += 1;
            continue;
        };
        stats.dated += 1;

        let mut event_places: Vec<String> = Vec::new();
        for loc_id in &f.locations {
            if let Some(our_id) = resolved_place_cache.get(loc_id) {
                if !event_places.contains(our_id) {
                    event_places.push(our_id.clone());
                }
                continue;
            }
            let Some(pf) = place_by_id.get(loc_id.as_str()) else {
                continue; // dangling record id in this trimmed/fixture data: skip
            };
            let name = pf.display_title.clone().or_else(|| pf.kjv_name.clone());
            let matched = name.as_ref().and_then(|n| place_slug_by_name.get(&n.to_lowercase()).cloned());

            let our_id = match matched {
                Some(id) => id,
                None => {
                    let (Some(lat_s), Some(lon_s)) = (pf.latitude.as_deref(), pf.longitude.as_deref()) else {
                        continue; // no coordinate to synthesize from either: skip this location
                    };
                    let (Ok(lat), Ok(lon)) = (lat_s.parse::<f64>(), lon_s.parse::<f64>()) else {
                        continue;
                    };
                    let new_id = pf.slug.clone().unwrap_or_else(|| loc_id.clone());
                    let new_name = name.clone().unwrap_or_else(|| new_id.clone());
                    if new_place_ids_used.insert(new_id.clone()) {
                        new_places.push(Place { id: new_id.clone(), name: new_name, lat, lon, verse_links: vec![] });
                        stats.new_places += 1;
                    }
                    new_id
                }
            };
            resolved_place_cache.insert(loc_id.clone(), our_id.clone());
            if !event_places.contains(&our_id) {
                event_places.push(our_id);
            }
        }
        if event_places.is_empty() {
            stats.no_place += 1;
        }

        let mut event_verses: Vec<String> = Vec::new();
        for verse_rec_id in &f.verses {
            let Some(osis_ref) = verse_osis_by_id.get(verse_rec_id.as_str()) else { continue };
            if let Some(vid) = osis::parse_verse(osis_ref) {
                let canon = osis::canonical(&vid);
                if !event_verses.contains(&canon) {
                    event_verses.push(canon);
                }
            }
        }

        let id = match f.event_id {
            Some(n) => format!("theo-{n}"),
            None => format!("theo-{}", rec.id),
        };

        let when = TimeRange::new(year, year).context("theographic event landed on an impossible (zero) year")?;
        out_events.push(Event { id, label: f.title.clone(), when, places: event_places, verses: event_verses, ..Default::default() });
    }

    Ok((out_events, new_places, stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_event_matches_readmes_worked_example() {
        // README: "-4003" (astronomical) == "4004 BC" == our historical -4004.
        assert_eq!(parse_theo_year("-4003"), Some(-4004));
    }

    #[test]
    fn iso_ish_and_bc_suffix_and_blank() {
        assert_eq!(parse_theo_year("0030-05-01"), Some(30));
        assert_eq!(parse_theo_year("1446 BC"), Some(-1446));
        assert_eq!(parse_theo_year(""), None);
        assert_eq!(parse_theo_year("   "), None);
    }

    #[test]
    fn positive_astronomical_year_is_unshifted() {
        assert_eq!(parse_theo_year("46"), Some(46));
        assert_eq!(parse_theo_year("1"), Some(1));
    }
}
