//! Joins Theographic's event, place and verse records into compiled events; their cross-references are
//! 14-char Airtable record ids. A `startDate` is an ASTRONOMICAL year, so `-4003` is 4004 BC: it is
//! converted to the no-year-zero convention, and an unparseable or year-zero date drops the event rather
//! than failing.

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
    /// The source omits the key entirely rather than writing an empty array, hence the default.
    #[serde(default)]
    locations: Vec<String>,
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

/// `pub(crate)` because the people parser reuses it verbatim for birth and death years: Theographic uses
/// the same astronomical convention for both tables, so a second copy could only drift.
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

    // A plain astronomical integer or an ISO-ish date: the leading signed integer is the astronomical
    // year.
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

/// The Airtable record id -> atlas event id map, spelled ONCE and by the same rule `parse_events` mints
/// ids with, so the people parser's timeline resolution cannot disagree with the events'.
pub fn event_ids_by_record(events_json: &str) -> Result<HashMap<String, String>> {
    let events: Vec<Record<EventFields>> = serde_json::from_str(events_json).context("theographic events.json is not valid JSON")?;
    Ok(events
        .iter()
        .map(|rec| {
            let id = match rec.fields.event_id {
                Some(n) => format!("theo-{n}"),
                None => format!("theo-{}", rec.id),
            };
            (rec.id.clone(), id)
        })
        .collect())
}

/// Joins events to places by case-insensitive name match against `place_slug_by_name`, whose keys must
/// already be lowercased. A place name with no geo match gets a `Place` synthesized from Theographic's own
/// latitude/longitude -- in that order, unlike the geocoding bundle's -- and those places are returned
/// alongside the events so the caller can merge them.
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
                continue;
            };
            let name = pf.display_title.clone().or_else(|| pf.kjv_name.clone());
            let matched = name.as_ref().and_then(|n| place_slug_by_name.get(&n.to_lowercase()).cloned());

            let our_id = match matched {
                Some(id) => id,
                None => {
                    let (Some(lat_s), Some(lon_s)) = (pf.latitude.as_deref(), pf.longitude.as_deref()) else {
                        continue;
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
