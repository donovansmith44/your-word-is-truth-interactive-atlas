//! Batch HOTFIX-7 (single-feed chronology migration, owner directive
//! 2026-08-22, near-verbatim: "do all our narratives/dates feed from the
//! same table? if not, they should."): the RESOLUTION layer HOTFIX-6's own
//! forward-compatibility note anticipated (`chronology-anchors.toml`'s own
//! header: "a future HOTFIX-7 migrates event date AUTHORING to resolve from
//! this table, never re-shaping it"). Before this batch, every curated
//! event AUTHORED its own literal `from_year`/`to_year`; the anchor table
//! only VALIDATED them after the fact (two independent facts that could
//! silently drift apart). After this batch, no curated event carries a
//! year literal at all -- every date is authored as exactly one of four
//! RESOLUTION FORMS (brief requirement 1) and COMPUTED here, at ETL time,
//! from the canonical table. `atlas_etl::curated::parse_events_extra` is
//! the authoring-guard half of this feature (rejects an inline literal,
//! requires exactly one resolution field) -- this module is the resolving
//! half.
//!
//! THE FOUR FORMS:
//! a. ANCHOR BINDING -- the event IS a table row (`chronology-
//!    anchors.toml`'s own worked example: `nehemiah-wall` ->
//!    `ret_jerusalem_wall`). Authored as `anchor = "<id>"` with no
//!    `offset`/`to_offset` (both default 0) -- NOT a separate enum
//!    variant from (b) below; see `ResolutionForm`'s own doc comment for
//!    why classifying it as the offset-zero special case of `Anchor` is
//!    deliberate, not a shortcut.
//! b. ANCHOR-RELATIVE PLACEMENT -- `anchor = "<id>"` plus a signed
//!    `offset` (and optional `to_offset` for a range; defaults to
//!    `offset`, i.e. a point). `from_year = table[anchor_id].year +
//!    offset`. A "reign_year N" claim (the table's reign-start anchors --
//!    `solomon-crowned`, `saul-anointed`, `kingdom-divided`, ... -- carry
//!    real start years) is authored as the direct arithmetic equivalent,
//!    `offset = N - 1` against that reign's own start anchor -- this
//!    module does not carry a SEPARATE "reign row" concept alongside the
//!    anchor table; HOTFIX-6's own schema doc comment already anticipated
//!    exactly this ("a future reign/sequence table can express 'N years
//!    after anchor X' arithmetically... without this file ever being
//!    reshaped" -- no new file needed, the arithmetic lives directly in
//!    the offset a curator writes).
//! c. SEQUENCE PLACEMENT -- `after = "<event id>"` plus `offset`/
//!    `to_offset` (same shape as (b), against a different kind of base).
//!    `from_year = resolve(after).from_year + offset` -- the referenced
//!    event's own resolved `from_year` is ALWAYS the base, even for a
//!    range event, so a curator can independently tune a range's own two
//!    ends against the SAME reference (see `resolve_curated_dates`'s own
//!    module-level worked example below). `after` may name EITHER another
//!    curated event (resolved recursively, memoized, cycle-checked) or an
//!    already-dated Theographic import event (a fixed base case -- see
//!    the `base_dates` parameter).
//! d. ERA PLACEMENT -- `era = "<id>"` (a `data/curated/eras.toml` row),
//!    no offset. `(from_year, to_year) = (table[era_id].from_year,
//!    table[era_id].to_year)` directly -- an era window IS the claim, not
//!    a point inside it, so unlike (b)/(c) there is nothing to offset.
//!    Rare by design (brief requirement 1d: "only where precision is
//!    genuinely era-level... each use justified inline") -- most curated
//!    events carry enough textual precision for (a)/(b)/(c) instead.
//!
//! WORKED EXAMPLE (a genuinely mixed-provenance range, real curated data):
//! `2ki_flight_and_jehoiachin_released` (2 Kings 25:26-30) opens with the
//! flight to Egypt, immediately after Jerusalem's own fall (the
//! `jerusalem-falls` anchor), and closes on Jehoiachin's OWN separately
//! textually-dated release 37-ish years later -- two different textual
//! bases for one event's two ends. Authored as `after = "exl_jerusalem"`,
//! `offset = 0` (the flight begins the SAME year Jerusalem falls -- moves
//! WITH that anchor if it is ever corrected again), `to_offset = 28`
//! (chosen so `to_year` lands on its own independently-dated -560
//! regardless of what `exl_jerusalem` resolves to -- Jehoiachin's release
//! does not move if Jerusalem's own fall year is later corrected again).
//! Both offsets are ordinary arithmetic against the identical `after`
//! reference; nothing here requires a second, independent reference field.
//!
//! WHY THIS LIVES HERE, NOT IN `atlas_etl::curated`: resolution needs the
//! FULL anchor/era table plus the FULL cross-file curated event pool (an
//! `after=` reference routinely crosses book/file boundaries -- e.g. a
//! Daniel event citing `ret_babylon`, an Ezra event citing a Nehemiah
//! anchor) -- neither is available to `parse_events_extra`, which parses
//! ONE file in isolation (see that function's own doc comment for the
//! established reason every OTHER cross-file curated fact, e.g.
//! `event-witnesses.toml`, is merged the same way, after every file's own
//! pure parse). `parse_events_extra` instead returns each dated event's
//! raw `DateResolution` ALONGSIDE an `Event` whose own `when` is a
//! placeholder (`TimeRange::new(1, 1)`, never read) -- `atlas_etl::main`
//! combines every curated file's own output into one pool and calls
//! `resolve_curated_dates` ONCE, after `chronology-anchors.toml`/
//! `eras.toml` are parsed and after the Theographic import's own
//! calibration (`nt_calibration::apply_nt_calibration` +
//! `chronology::apply_theo_date_overrides`) has run on it -- the SAME
//! ETL-only, called-exactly-once discipline those two modules already
//! established for the identical reason (idempotency: nothing downstream
//! ever re-parses curated TOML the way `AtlasData::finish()` re-runs
//! across the ETL/server boundary, so this module's own single call site
//! is naturally exactly-once, but the ORDERING relative to Theographic
//! calibration still matters -- `base_dates` must already reflect the
//! CALIBRATED Theographic scale before any curated `after=` reference
//! reads it, or a curated event chained off a `theo-*` id would silently
//! resolve against the pre-calibration Theographic clock instead).

use std::collections::HashMap;
use std::fmt;

use crate::data::{ChronologyAnchor, Era, Event};
use crate::time::{TimeRange, Year};

/// One curated event's own raw date-resolution declaration, exactly as
/// authored in TOML (`anchor=`/`after=`/`era=` plus `offset=`/`to_offset=`)
/// -- see this module's own doc comment for the four forms. `Anchor`
/// covers BOTH form (a) and form (b): see `ResolutionForm::classify`'s own
/// doc comment for why the offset-zero-and-bound-to-this-event case is
/// classified as "anchor binding" rather than modeled as a fifth variant.
#[derive(Debug, Clone, PartialEq)]
pub enum DateResolution {
    Anchor { anchor_id: String, offset: i32, to_offset: i32 },
    Sequence { after: String, offset: i32, to_offset: i32 },
    Era { era_id: String },
}

/// The four forms, as reported in `ResolutionLogEntry`/the migration
/// report's own per-form tally -- a STRING tag, not a fifth
/// `DateResolution` variant, because "is this row anchor BINDING or
/// anchor-RELATIVE" is not something a curator ever declares (both are
/// authored identically, `anchor = "<id>"` plus optional offsets) -- it is
/// a fact about the TABLE (does this anchor's own `event_id` name this
/// exact event) that only the resolver, which has the table in hand, can
/// determine. Modeling it as a separate `DateResolution` variant would let
/// a curator's own typo (or a future retargeted anchor) silently disagree
/// with the table about which case applies; deriving it here means the
/// classification can never drift from the table's own truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionForm {
    AnchorBinding,
    AnchorRelative,
    Sequence,
    Era,
}

impl ResolutionForm {
    pub fn as_str(self) -> &'static str {
        match self {
            ResolutionForm::AnchorBinding => "anchor-binding",
            ResolutionForm::AnchorRelative => "anchor-relative",
            ResolutionForm::Sequence => "sequence",
            ResolutionForm::Era => "era",
        }
    }
}

/// One resolved event's own audit-trail row -- mirrors
/// `nt_calibration::CalibrationLogEntry`/`chronology::DateOverrideLogEntry`'s
/// own shape (id, what happened, before/after aren't applicable here since
/// there is no curated "before" -- a curated event never HAD a literal
/// year to begin with, so this logs the resolved RESULT plus its own
/// provenance instead).
#[derive(Debug, Clone, PartialEq)]
pub struct ResolutionLogEntry {
    pub id: String,
    pub form: ResolutionForm,
    /// The anchor id / `after` event id / era id this event resolved
    /// against.
    pub reference: String,
    pub from_year: Year,
    pub to_year: Year,
}

/// Every way `resolve_curated_dates` fails loud -- each names the event
/// id so a curator can find the bad row immediately, matching this
/// codebase's own established error-message discipline
/// (`atlas_etl::curated`'s own `bail!` messages, `atlas_etl::validate`'s
/// own named-violation style).
#[derive(Debug, Clone, PartialEq)]
pub enum ResolveError {
    UnknownAnchor { event_id: String, anchor_id: String },
    UnknownEra { event_id: String, era_id: String },
    DanglingAfter { event_id: String, after_id: String },
    /// A `after=` chain that loops back on itself -- `path` is the full
    /// cycle, in resolution order, for an unambiguous error message (not
    /// just "id X" with no context on how the cycle was reached).
    Cycle { path: Vec<String> },
    InvalidRange { event_id: String, from_year: Year, to_year: Year },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::UnknownAnchor { event_id, anchor_id } => {
                write!(f, "curated event '{event_id}' resolves anchor = \"{anchor_id}\", which is not a row in chronology-anchors.toml")
            }
            ResolveError::UnknownEra { event_id, era_id } => {
                write!(f, "curated event '{event_id}' resolves era = \"{era_id}\", which is not a row in eras.toml")
            }
            ResolveError::DanglingAfter { event_id, after_id } => {
                write!(f, "curated event '{event_id}' resolves after = \"{after_id}\", which is neither another curated event nor a known Theographic import id")
            }
            ResolveError::Cycle { path } => {
                write!(f, "curated date resolution cycle: {}", path.join(" -> "))
            }
            ResolveError::InvalidRange { event_id, from_year, to_year } => {
                write!(f, "curated event '{event_id}' resolves to an inverted range (from_year={from_year} > to_year={to_year}) -- fix its own offset/to_offset")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// Resolves every curated event's own `when` from its `DateResolution`
/// (`resolutions`, keyed by event id -- an event absent from this map is
/// left completely untouched, e.g. a `kind = "general"` row, which never
/// had one) against `anchors`/`eras` (the canonical tables) and
/// `base_dates` (id -> (from_year, to_year) for every Theographic import
/// event, ALREADY calibrated -- see this module's own doc comment for why
/// ordering matters). Mutates `events` in place; returns the full
/// audit-trail log, one row per RESOLVED event, in dependency order (a
/// `Sequence` event's own log row always follows whatever it chained off,
/// deterministic given fixed inputs, though not necessarily `events`'s own
/// array order -- callers wanting per-book/per-file tallies group this log
/// by their own means, same as `nt_calibration`'s own log is grouped by
/// its caller).
pub fn resolve_curated_dates(
    anchors: &[ChronologyAnchor],
    eras: &[Era],
    base_dates: &HashMap<String, (Year, Year)>,
    events: &mut [Event],
    resolutions: &HashMap<String, DateResolution>,
) -> Result<Vec<ResolutionLogEntry>, ResolveError> {
    let anchor_by_id: HashMap<&str, &ChronologyAnchor> = anchors.iter().map(|a| (a.id.as_str(), a)).collect();
    let era_by_id: HashMap<&str, &Era> = eras.iter().map(|e| (e.id.as_str(), e)).collect();
    // anchor id -> the event id it is bound to, purely to classify (a)
    // anchor-binding vs (b) anchor-relative in the log -- see
    // `ResolutionForm`'s own doc comment for why this is derived, not
    // curator-declared.
    let anchor_bound_event: HashMap<&str, &str> =
        anchors.iter().filter_map(|a| a.event_id.as_deref().map(|eid| (a.id.as_str(), eid))).collect();

    let mut resolved: HashMap<String, (Year, Year)> = HashMap::new();
    let mut log: Vec<ResolutionLogEntry> = Vec::new();

    // Stable, deterministic outer order (a plain `Vec`, not a HashMap
    // iteration) -- `events`'s own array order -- so repeated ETL runs
    // against unchanged input produce a byte-identical log every time
    // (only the RECURSION order within one event's own dependency chain
    // can differ from this outer order, never the run-to-run result for
    // fixed input).
    let ids_in_order: Vec<String> = events.iter().map(|e| e.id.clone()).collect();
    for id in &ids_in_order {
        if resolutions.contains_key(id) {
            resolve_one(id, resolutions, &anchor_by_id, &era_by_id, base_dates, &mut resolved, &mut Vec::new(), &mut log, &anchor_bound_event)?;
        }
    }

    for e in events.iter_mut() {
        if let Some(&(from_year, to_year)) = resolved.get(&e.id) {
            e.when = TimeRange::new(from_year, to_year)
                .map_err(|_| ResolveError::InvalidRange { event_id: e.id.clone(), from_year, to_year })?;
        }
    }
    Ok(log)
}

#[allow(clippy::too_many_arguments)]
fn resolve_one(
    id: &str,
    resolutions: &HashMap<String, DateResolution>,
    anchor_by_id: &HashMap<&str, &ChronologyAnchor>,
    era_by_id: &HashMap<&str, &Era>,
    base_dates: &HashMap<String, (Year, Year)>,
    resolved: &mut HashMap<String, (Year, Year)>,
    stack: &mut Vec<String>,
    log: &mut Vec<ResolutionLogEntry>,
    anchor_bound_event: &HashMap<&str, &str>,
) -> Result<(Year, Year), ResolveError> {
    if let Some(&v) = resolved.get(id) {
        return Ok(v);
    }
    if stack.iter().any(|s| s == id) {
        let mut path = stack.clone();
        path.push(id.to_string());
        return Err(ResolveError::Cycle { path });
    }
    // `id` is only ever called with a key `resolutions` (the top-level
    // loop) OR a `Sequence::after` value the branch below already checked
    // exists in `resolutions` before recursing -- `.expect` here documents
    // that invariant rather than threading a fallible lookup through every
    // caller a second time.
    let res = resolutions.get(id).expect("resolve_one is only ever called with a key known to be in `resolutions`");

    stack.push(id.to_string());
    let (from_year, to_year, form, reference) = match res {
        DateResolution::Anchor { anchor_id, offset, to_offset } => {
            let Some(a) = anchor_by_id.get(anchor_id.as_str()) else {
                return Err(ResolveError::UnknownAnchor { event_id: id.to_string(), anchor_id: anchor_id.clone() });
            };
            let from_year = a.year + offset;
            let to_year = a.year + to_offset;
            let is_binding = *offset == 0 && *to_offset == 0 && anchor_bound_event.get(anchor_id.as_str()) == Some(&id);
            let form = if is_binding { ResolutionForm::AnchorBinding } else { ResolutionForm::AnchorRelative };
            (from_year, to_year, form, anchor_id.clone())
        }
        DateResolution::Sequence { after, offset, to_offset } => {
            if !resolutions.contains_key(after) && !base_dates.contains_key(after) {
                return Err(ResolveError::DanglingAfter { event_id: id.to_string(), after_id: after.clone() });
            }
            let base = if let Some(&v) = base_dates.get(after) {
                // A Theographic import id is a fixed base case (already
                // calibrated by the time this module runs) -- resolved
                // directly, never recursed into (it has no `DateResolution`
                // of its own; a curated event id always wins if the SAME id
                // string somehow named both, which never happens in
                // practice since the two id namespaces are disjoint by
                // convention -- `theo-*` vs everything else).
                v
            } else {
                resolve_one(after, resolutions, anchor_by_id, era_by_id, base_dates, resolved, stack, log, anchor_bound_event)?
            };
            (base.0 + offset, base.0 + to_offset, ResolutionForm::Sequence, after.clone())
        }
        DateResolution::Era { era_id } => {
            let Some(e) = era_by_id.get(era_id.as_str()) else {
                return Err(ResolveError::UnknownEra { event_id: id.to_string(), era_id: era_id.clone() });
            };
            (e.from_year, e.to_year, ResolutionForm::Era, era_id.clone())
        }
    };
    stack.pop();

    resolved.insert(id.to_string(), (from_year, to_year));
    log.push(ResolutionLogEntry { id: id.to_string(), form, reference, from_year, to_year });
    Ok((from_year, to_year))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Event;

    fn event(id: &str) -> Event {
        Event { id: id.into(), label: id.into(), when: TimeRange::new(1, 1).unwrap(), verses: vec!["GEN.1.1".into()], ..Default::default() }
    }

    fn anchor(id: &str, year: Year, event_id: Option<&str>) -> ChronologyAnchor {
        ChronologyAnchor { id: id.into(), label: id.into(), year, event_id: event_id.map(String::from), era_boundary: false, source: "test".into(), note: None }
    }

    fn era(id: &str, from: Year, to: Year) -> Era {
        Era { id: id.into(), name: id.into(), from_year: from, to_year: to }
    }

    // --- form (a)/(b): DateResolution::Anchor -----------------------------

    #[test]
    fn anchor_binding_resolves_to_the_anchor_year_exactly() {
        let anchors = vec![anchor("solomon-crowned", -1015, Some("1ki_solomon_anointed"))];
        let mut events = vec![event("1ki_solomon_anointed")];
        let resolutions = HashMap::from([("1ki_solomon_anointed".to_string(), DateResolution::Anchor { anchor_id: "solomon-crowned".into(), offset: 0, to_offset: 0 })]);
        let log = resolve_curated_dates(&anchors, &[], &HashMap::new(), &mut events, &resolutions).unwrap();
        assert_eq!(events[0].when, TimeRange::new(-1015, -1015).unwrap());
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].form, ResolutionForm::AnchorBinding, "offset 0 AND this event is the anchor's own bound event_id -- form (a)");
        assert_eq!(log[0].reference, "solomon-crowned");
    }

    #[test]
    fn anchor_relative_offset_resolves_arithmetically_both_ends() {
        let anchors = vec![anchor("kingdom-divided", -975, Some("1ki_kingdom_divided"))];
        let mut events = vec![event("2ki_fall_of_samaria")];
        let resolutions = HashMap::from([(
            "2ki_fall_of_samaria".to_string(),
            DateResolution::Anchor { anchor_id: "kingdom-divided".into(), offset: -9, to_offset: 254 },
        )]);
        let log = resolve_curated_dates(&anchors, &[], &HashMap::new(), &mut events, &resolutions).unwrap();
        // -975 + (-9) = -984 (from), -975 + 254 = -721 (to) -- an arbitrary
        // but exact worked check that both offsets are applied
        // independently against the SAME anchor year.
        assert_eq!(events[0].when, TimeRange::new(-984, -721).unwrap());
        assert_eq!(log[0].form, ResolutionForm::AnchorRelative, "nonzero offset -- form (b), never binding, even though this anchor IS bound (to a DIFFERENT event)");
    }

    #[test]
    fn offset_zero_against_an_anchor_bound_to_a_different_event_is_still_relative_not_binding() {
        // A coincidental offset-0 alignment with an anchor bound to
        // SOMEONE ELSE'S event must never be misreported as this event
        // being the anchor's own row (ResolutionForm's own doc comment).
        let anchors = vec![anchor("solomon-crowned", -1015, Some("1ki_solomon_anointed"))];
        let mut events = vec![event("some_other_event")];
        let resolutions = HashMap::from([("some_other_event".to_string(), DateResolution::Anchor { anchor_id: "solomon-crowned".into(), offset: 0, to_offset: 0 })]);
        let log = resolve_curated_dates(&anchors, &[], &HashMap::new(), &mut events, &resolutions).unwrap();
        assert_eq!(log[0].form, ResolutionForm::AnchorRelative);
    }

    #[test]
    fn unknown_anchor_id_fails_loud_naming_both_ids() {
        let mut events = vec![event("x")];
        let resolutions = HashMap::from([("x".to_string(), DateResolution::Anchor { anchor_id: "not-a-real-anchor".into(), offset: 0, to_offset: 0 })]);
        let err = resolve_curated_dates(&[], &[], &HashMap::new(), &mut events, &resolutions).unwrap_err();
        assert_eq!(err, ResolveError::UnknownAnchor { event_id: "x".into(), anchor_id: "not-a-real-anchor".into() });
    }

    // --- form (c): DateResolution::Sequence --------------------------------

    #[test]
    fn sequence_resolves_against_another_curated_events_own_from_year() {
        let anchors = vec![anchor("david-hebron", -1055, Some("df_hebron"))];
        let mut events = vec![event("df_hebron"), event("df_ziklag")];
        let resolutions = HashMap::from([
            ("df_hebron".to_string(), DateResolution::Anchor { anchor_id: "david-hebron".into(), offset: 0, to_offset: 0 }),
            ("df_ziklag".to_string(), DateResolution::Sequence { after: "df_hebron".into(), offset: -3, to_offset: -3 }),
        ]);
        let log = resolve_curated_dates(&anchors, &[], &HashMap::new(), &mut events, &resolutions).unwrap();
        assert_eq!(events[1].when, TimeRange::new(-1058, -1058).unwrap());
        let ziklag_row = log.iter().find(|r| r.id == "df_ziklag").unwrap();
        assert_eq!(ziklag_row.form, ResolutionForm::Sequence);
        assert_eq!(ziklag_row.reference, "df_hebron");
    }

    #[test]
    fn sequence_chains_transitively_through_several_curated_events() {
        let anchors = vec![anchor("david-hebron", -1055, Some("df_hebron"))];
        let mut events = vec![event("df_hebron"), event("df_ziklag"), event("df_en_gedi")];
        let resolutions = HashMap::from([
            ("df_hebron".to_string(), DateResolution::Anchor { anchor_id: "david-hebron".into(), offset: 0, to_offset: 0 }),
            ("df_ziklag".to_string(), DateResolution::Sequence { after: "df_hebron".into(), offset: -3, to_offset: -3 }),
            ("df_en_gedi".to_string(), DateResolution::Sequence { after: "df_ziklag".into(), offset: -1, to_offset: -1 }),
        ]);
        resolve_curated_dates(&anchors, &[], &HashMap::new(), &mut events, &resolutions).unwrap();
        assert_eq!(events[2].when, TimeRange::new(-1059, -1059).unwrap(), "df_en_gedi = df_ziklag(-1058) - 1, transitively through the chain, order-independent of `events`'s own array order");
    }

    #[test]
    fn sequence_two_independently_tuned_offsets_reproduce_a_mixed_provenance_range() {
        // The module's own worked example: 2ki_flight_and_jehoiachin_released
        // -- from_year moves WITH its anchor, to_year stays fixed regardless.
        let anchors = vec![anchor("jerusalem-falls", -588, Some("exl_jerusalem"))];
        let mut events = vec![event("exl_jerusalem"), event("range_event")];
        let resolutions = HashMap::from([
            ("exl_jerusalem".to_string(), DateResolution::Anchor { anchor_id: "jerusalem-falls".into(), offset: 0, to_offset: 0 }),
            ("range_event".to_string(), DateResolution::Sequence { after: "exl_jerusalem".into(), offset: 0, to_offset: 28 }),
        ]);
        resolve_curated_dates(&anchors, &[], &HashMap::new(), &mut events, &resolutions).unwrap();
        assert_eq!(events[1].when, TimeRange::new(-588, -560).unwrap());
    }

    #[test]
    fn sequence_can_resolve_against_a_theographic_base_date() {
        let mut events = vec![event("jer_the_two_baskets_of_figs")];
        let resolutions = HashMap::from([("jer_the_two_baskets_of_figs".to_string(), DateResolution::Sequence { after: "theo-241".into(), offset: 0, to_offset: 0 })]);
        let base_dates = HashMap::from([("theo-241".to_string(), (-598, -598))]);
        resolve_curated_dates(&[], &[], &base_dates, &mut events, &resolutions).unwrap();
        assert_eq!(events[0].when, TimeRange::new(-598, -598).unwrap());
    }

    #[test]
    fn dangling_after_reference_fails_loud() {
        let mut events = vec![event("x")];
        let resolutions = HashMap::from([("x".to_string(), DateResolution::Sequence { after: "does-not-exist".into(), offset: 0, to_offset: 0 })]);
        let err = resolve_curated_dates(&[], &[], &HashMap::new(), &mut events, &resolutions).unwrap_err();
        assert_eq!(err, ResolveError::DanglingAfter { event_id: "x".into(), after_id: "does-not-exist".into() });
    }

    #[test]
    fn a_two_event_cycle_fails_loud_naming_the_path() {
        let mut events = vec![event("a"), event("b")];
        let resolutions = HashMap::from([
            ("a".to_string(), DateResolution::Sequence { after: "b".into(), offset: 0, to_offset: 0 }),
            ("b".to_string(), DateResolution::Sequence { after: "a".into(), offset: 0, to_offset: 0 }),
        ]);
        let err = resolve_curated_dates(&[], &[], &HashMap::new(), &mut events, &resolutions).unwrap_err();
        match err {
            ResolveError::Cycle { path } => assert!(path.len() >= 2, "cycle path must name at least the two looping ids: {path:?}"),
            other => panic!("expected Cycle, got {other:?}"),
        }
    }

    #[test]
    fn a_self_reference_is_a_cycle_of_one() {
        let mut events = vec![event("a")];
        let resolutions = HashMap::from([("a".to_string(), DateResolution::Sequence { after: "a".into(), offset: 0, to_offset: 0 })]);
        let err = resolve_curated_dates(&[], &[], &HashMap::new(), &mut events, &resolutions).unwrap_err();
        assert!(matches!(err, ResolveError::Cycle { .. }));
    }

    // --- form (d): DateResolution::Era -------------------------------------

    #[test]
    fn era_placement_resolves_to_the_eras_own_window_directly() {
        let eras = vec![era("exile", -586, -539)];
        let mut events = vec![event("some_wide_event")];
        let resolutions = HashMap::from([("some_wide_event".to_string(), DateResolution::Era { era_id: "exile".into() })]);
        let log = resolve_curated_dates(&[], &eras, &HashMap::new(), &mut events, &resolutions).unwrap();
        assert_eq!(events[0].when, TimeRange::new(-586, -539).unwrap());
        assert_eq!(log[0].form, ResolutionForm::Era);
        assert_eq!(log[0].reference, "exile");
    }

    #[test]
    fn unknown_era_id_fails_loud() {
        let mut events = vec![event("x")];
        let resolutions = HashMap::from([("x".to_string(), DateResolution::Era { era_id: "not-a-real-era".into() })]);
        let err = resolve_curated_dates(&[], &[], &HashMap::new(), &mut events, &resolutions).unwrap_err();
        assert_eq!(err, ResolveError::UnknownEra { event_id: "x".into(), era_id: "not-a-real-era".into() });
    }

    // --- cross-cutting -------------------------------------------------------

    #[test]
    fn an_event_with_no_resolution_entry_is_left_completely_untouched() {
        // kind="general" rows never get a DateResolution -- this must be a
        // pure no-op, not an error, and not even a lookup attempt.
        let mut events = vec![event("general_thing")];
        let placeholder = events[0].when;
        resolve_curated_dates(&[], &[], &HashMap::new(), &mut events, &HashMap::new()).unwrap();
        assert_eq!(events[0].when, placeholder);
    }

    #[test]
    fn inverted_offsets_fail_loud_rather_than_silently_shipping_a_backwards_range() {
        let anchors = vec![anchor("passion", 33, Some("pw_golgotha"))];
        let mut events = vec![event("pw_golgotha")];
        let resolutions = HashMap::from([("pw_golgotha".to_string(), DateResolution::Anchor { anchor_id: "passion".into(), offset: 5, to_offset: 0 })]);
        let err = resolve_curated_dates(&anchors, &[], &HashMap::new(), &mut events, &resolutions).unwrap_err();
        assert_eq!(err, ResolveError::InvalidRange { event_id: "pw_golgotha".into(), from_year: 38, to_year: 33 });
    }

    #[test]
    fn resolution_log_is_deterministic_across_repeated_runs() {
        let anchors = vec![anchor("david-hebron", -1055, Some("df_hebron"))];
        let mut events_a = vec![event("df_hebron"), event("df_ziklag"), event("df_en_gedi")];
        let mut events_b = events_a.clone();
        let resolutions = HashMap::from([
            ("df_hebron".to_string(), DateResolution::Anchor { anchor_id: "david-hebron".into(), offset: 0, to_offset: 0 }),
            ("df_ziklag".to_string(), DateResolution::Sequence { after: "df_hebron".into(), offset: -3, to_offset: -3 }),
            ("df_en_gedi".to_string(), DateResolution::Sequence { after: "df_ziklag".into(), offset: -1, to_offset: -1 }),
        ]);
        let log_a = resolve_curated_dates(&anchors, &[], &HashMap::new(), &mut events_a, &resolutions).unwrap();
        let log_b = resolve_curated_dates(&anchors, &[], &HashMap::new(), &mut events_b, &resolutions).unwrap();
        assert_eq!(log_a, log_b);
    }
}
