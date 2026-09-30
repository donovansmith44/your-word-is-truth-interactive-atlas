//! Parsers for the curated TOML files. Every parser here is pure and STRUCTURAL only: a malformed file bails
//! immediately, and every cross-check that needs the wider compile -- a real event id, a real place, a verse
//! that exists in the text -- belongs to `validate`. A curator-friendly range expands into single verses.

use anyhow::{bail, Context, Result};
use atlas_core::data::{BookAuthorship, BookMeta, BookNarrationWindow, CatechismItem, CatechismPart, ChronologyAnchor, Era, Event, EventId, EventKind, FulfillmentSeed, Landmark, LandMaskRegion, Narrative, NamedAfterSeed, BrethrenSeed, ParentageExclusion, ParentageSeed, PeopleGroupReclassify, PeopleGroupSeed, PlaceBlurbEntry, PlaceDateClaim, PlaceHistory, PlaceNameAlias, PlaceNameEntry, PersonId, Polity, PolityDelta, PolityEra, TypologySeed};
use atlas_core::refs::ScriptureRef;
use atlas_core::time::TimeRange;
use serde::Deserialize;

use crate::catechism_map::{Deut5Entry, MappingFile, MappingOverride};
use crate::concord::ConcordTitleOverride;

#[derive(Deserialize)]
struct ErasFile {
    era: Vec<Era>,
}

/// Contiguity, coverage and zero-year checks belong to `validate`, which needs the full picture and must
/// report EVERY violation rather than fail at the first bad era.
pub fn parse_eras(input: &str) -> Result<Vec<Era>> {
    let f: ErasFile = toml::from_str(input).context("eras.toml: invalid TOML or does not match the [[era]] schema")?;
    Ok(f.era)
}

#[derive(Deserialize)]
struct ChronologyAnchorsFile {
    anchor: Vec<ChronologyAnchor>,
}

pub fn parse_chronology_anchors(input: &str) -> Result<Vec<ChronologyAnchor>> {
    let f: ChronologyAnchorsFile =
        toml::from_str(input).context("chronology-anchors.toml: invalid TOML or does not match the [[anchor]] schema")?;
    Ok(f.anchor)
}

#[derive(Deserialize)]
struct BookNarrationWindowsFile {
    window: Vec<BookNarrationWindow>,
}

pub fn parse_book_narration_windows(input: &str) -> Result<Vec<BookNarrationWindow>> {
    let f: BookNarrationWindowsFile = toml::from_str(input)
        .context("book-narration-windows.toml: invalid TOML or does not match the [[window]] schema")?;
    Ok(f.window)
}

#[derive(Deserialize)]
struct BookToml {
    code: String,
    author: String,
    #[serde(default)]
    author_ids: Vec<PersonId>,
    #[serde(default)]
    write_place: Option<String>,
    #[serde(default)]
    write_from: Option<i32>,
    #[serde(default)]
    write_to: Option<i32>,
}

#[derive(Deserialize)]
struct BooksFile {
    book: Vec<BookToml>,
}

/// One file, two compiled views: the served metadata and the authorship the graph lowers into rows.
#[derive(Debug, PartialEq)]
pub struct CuratedBooks {
    pub meta: Vec<BookMeta>,
    pub authorship: Vec<BookAuthorship>,
}

/// The TOML field is `code` while the compiled field is `book`: this wrapper does that one rename.
pub fn parse_books(input: &str) -> Result<CuratedBooks> {
    let f: BooksFile = toml::from_str(input).context("books.toml: invalid TOML or does not match the [[book]] schema")?;
    let (meta, authorship) = f
        .book
        .into_iter()
        .map(|b| {
            (
                BookMeta { book: b.code.clone(), author: b.author, write_place: b.write_place, write_from: b.write_from, write_to: b.write_to },
                BookAuthorship { book: b.code, author_ids: b.author_ids },
            )
        })
        .unzip();
    Ok(CuratedBooks { meta, authorship })
}

/// One file holds exactly one narrative, as bare top-level fields rather than an array of tables, hence the
/// singular.
pub fn parse_narrative(input: &str) -> Result<Narrative> {
    toml::from_str(input).context("narrative TOML: invalid TOML or does not match the Narrative schema (id/name/color/legs)")
}

#[derive(Deserialize)]
struct EventToml {
    id: String,
    label: String,
    /// A `general` row must OMIT both; an `event` row, which is the default, requires both.
    #[serde(default)]
    from_year: Option<i32>,
    #[serde(default)]
    to_year: Option<i32>,
    /// Omitted by a `general` row, exactly as the years are.
    #[serde(default)]
    places: Vec<String>,
    #[serde(default)]
    verses: Vec<String>,
    #[serde(default)]
    kind: Option<EventKind>,
    /// Defaulted so every event authored before these fields existed keeps parsing with no migration.
    #[serde(default)]
    robertson_section: Option<String>,
    /// Inline provenance for a brand-new row. A pre-existing imported event has no row to carry a field, so
    /// it is enriched through the separate merge file instead.
    #[serde(default)]
    acts_section: Option<String>,
    #[serde(default)]
    atlas_section: Option<String>,
    /// Inline only: this one has no merge-file sibling, because every container titled from a superscription
    /// is a brand-new row rather than the promotion of an imported event.
    #[serde(default)]
    kjv_superscription: Option<String>,
    #[serde(default)]
    ref_note: Option<String>,
    #[serde(default)]
    order_key: i32,
}

fn default_event_kind() -> EventKind {
    EventKind::Event
}

#[derive(Deserialize)]
struct EventsFile {
    event: Vec<EventToml>,
}

/// Expands one curated verse entry -- a single canonical verse, or a same-chapter range -- into one or more
/// canonical single-verse strings. `context` names the record the ref came from, purely for the error message.
/// `pub` so the binary can reuse it for the supplement file, whose refs are identical in shape.
pub fn expand_verse_ref(raw: &str, context: &str, out: &mut Vec<String>) -> Result<()> {
    match ScriptureRef::parse(raw) {
        Ok(ScriptureRef::Verse(v)) => out.push(format!("{}.{}.{}", v.book.code(), v.chapter, v.verse)),
        Ok(ScriptureRef::Passage { book, chapter, from_verse, to_verse }) => {
            for verse in from_verse..=to_verse {
                out.push(format!("{}.{}.{}", book.code(), chapter, verse));
            }
        }
        _ => bail!("curated data '{context}' has an unparseable verse ref '{raw}' (expected e.g. 'EXO.12.37' or 'EXO.14.21-31')"),
    }
    Ok(())
}

/// Hard-errors rather than soft-dropping: this is our own authored data, held to a higher bar than third-party rows.
/// `kind` gates which fields are required in BOTH directions -- an `event` row carries places and both years, a
/// `general` row none of them -- and a row carrying both a `general` kind and a date is a hard error.
pub fn parse_events_extra(input: &str) -> Result<Vec<Event>> {
    let f: EventsFile =
        toml::from_str(input).context("events-extra.toml: invalid TOML or does not match the [[event]] schema")?;

    let mut out = Vec::with_capacity(f.event.len());
    for e in f.event {
        let kind = e.kind.unwrap_or_else(default_event_kind);
        let (when, places) = if kind == EventKind::General {
            if e.from_year.is_some() || e.to_year.is_some() {
                bail!(
                    "curated event '{}' is kind=\"general\" but specifies from_year/to_year -- a general-kind passage must not claim a date (omit both fields; do not fabricate)",
                    e.id
                );
            }
            if !e.places.is_empty() {
                bail!(
                    "curated event '{}' is kind=\"general\" but specifies places -- a general-kind passage must not claim a place mapping (omit the field; do not fabricate)",
                    e.id
                );
            }
            (TimeRange::undated(), e.places)
        } else {
            if e.places.is_empty() {
                bail!("curated event '{}' has no places (places[0] is required as the narrative-arrow anchor)", e.id);
            }
            let (from_year, to_year) = match (e.from_year, e.to_year) {
                (Some(fy), Some(ty)) => (fy, ty),
                _ => bail!(
                    "curated event '{}' is missing from_year/to_year -- required unless kind=\"general\"",
                    e.id
                ),
            };
            let when = TimeRange::new(from_year, to_year).map_err(|src| {
                anyhow::anyhow!("curated event '{}' (from_year={}, to_year={}): {}", e.id, from_year, to_year, src)
            })?;
            (when, e.places)
        };
        let mut verses = Vec::new();
        for v in &e.verses {
            expand_verse_ref(v, &e.id, &mut verses)?;
        }
        out.push(Event {
            id: e.id,
            label: e.label,
            when,
            places,
            verses,
            kind,
            robertson_section: e.robertson_section,
            acts_section: e.acts_section,
            atlas_section: e.atlas_section,
            kjv_superscription: e.kjv_superscription,
            ref_note: e.ref_note,
            order_key: e.order_key,
            ..Default::default()
        });
    }
    Ok(out)
}

#[derive(Deserialize)]
struct EventWitnessesFile {
    witness: Vec<WitnessToml>,
}

#[derive(Deserialize)]
struct WitnessToml {
    event_id: String,
    book: String,
    /// Curator-friendly single-verse-or-range strings in our own canonical codes, expanded the same way the
    /// events file's own verses are.
    verses: Vec<String>,
    #[serde(default)]
    ref_note: Option<String>,
    #[serde(default)]
    robertson_section: Option<String>,
}

/// A FLAT array, each row naming its own event id, deliberately NOT nested under a per-event group: a nested
/// array-of-tables subtable attaches to the most recently opened parent, which once silently mis-attached real
/// curated rows. A flat list with an explicit id per row has no such failure mode.
pub fn parse_event_witnesses(input: &str) -> Result<Vec<(String, atlas_core::data::EventWitness)>> {
    let f: EventWitnessesFile =
        toml::from_str(input).context("event-witnesses.toml: invalid TOML or does not match the [[witness]] schema")?;

    let mut out = Vec::with_capacity(f.witness.len());
    for w in f.witness {
        let mut verses = Vec::new();
        for v in &w.verses {
            expand_verse_ref(v, &format!("witness for event '{}' ({})", w.event_id, w.book), &mut verses)?;
        }
        let translations =
            std::collections::HashMap::from([(atlas_core::translation::DEFAULT_TRANSLATION.to_string(), verses)]);
        out.push((
            w.event_id,
            atlas_core::data::EventWitness { book: w.book, translations, ref_note: w.ref_note, robertson_section: w.robertson_section },
        ));
    }
    Ok(out)
}

#[derive(Deserialize)]
struct AttestationCorrectionsFile {
    #[serde(default)]
    mention: Vec<MentionToml>,
    #[serde(default)]
    analogue: Vec<AnalogueToml>,
}

#[derive(Deserialize)]
struct MentionToml {
    event_id: String,
    verses: Vec<String>,
    note: String,
}

#[derive(Deserialize)]
struct AnalogueToml {
    a: String,
    b: String,
    note: String,
}

/// A mention row is a RETYPE, not a deletion: the compile strips the named verses out of that event's own
/// verses and witness rows so no attestation is built for them, and the row goes on to become a mention
/// pointing at the event. The fact changes type and keeps its provenance; nothing is dropped.
pub fn parse_attestation_corrections(
    input: &str,
) -> Result<(Vec<atlas_core::data::EventMentionSeed>, Vec<atlas_core::data::EventAnalogueSeed>)> {
    let f: AttestationCorrectionsFile = toml::from_str(input)
        .context("attestation-corrections.toml: invalid TOML or does not match the [[mention]]/[[analogue]] schema")?;

    let mut mentions = Vec::with_capacity(f.mention.len());
    for m in f.mention {
        let mut verses = Vec::new();
        for v in &m.verses {
            expand_verse_ref(v, &format!("mention row for event '{}'", m.event_id), &mut verses)?;
        }
        if verses.is_empty() {
            bail!(
                "attestation-corrections.toml: [[mention]] row for event '{}' names no verses -- a retype with nothing to retype is a curation mistake, not an empty no-op",
                m.event_id
            );
        }
        mentions.push(atlas_core::data::EventMentionSeed { event_id: m.event_id, verses, note: m.note });
    }

    let mut analogues = Vec::with_capacity(f.analogue.len());
    for a in f.analogue {
        if a.a == a.b {
            bail!("attestation-corrections.toml: [[analogue]] row joins '{}' to itself -- an Analogue asserts two DISTINCT events", a.a);
        }
        analogues.push(atlas_core::data::EventAnalogueSeed { a: a.a, b: a.b, note: a.note });
    }

    Ok((mentions, analogues))
}

#[derive(Deserialize)]
struct ActsSectionsFile {
    #[serde(default)]
    section: Vec<ActsSectionToml>,
}

#[derive(Deserialize)]
struct ActsSectionToml {
    event_id: String,
    acts_section: String,
}

/// This file exists so a bare imported event -- which has no authored row of its own to add a field to -- can
/// still gain provenance: the rows are merged onto the full combined event set by id.
pub fn parse_acts_sections(input: &str) -> Result<Vec<(String, String)>> {
    let f: ActsSectionsFile =
        toml::from_str(input).context("acts-sections.toml: invalid TOML or does not match the [[section]] schema")?;
    Ok(f.section.into_iter().map(|s| (s.event_id, s.acts_section)).collect())
}

#[derive(Deserialize)]
struct AtlasSectionsFile {
    #[serde(default)]
    section: Vec<AtlasSectionToml>,
}

#[derive(Deserialize)]
struct AtlasSectionToml {
    event_id: String,
    atlas_section: String,
}

/// The same flat, merge-by-id shape as the acts sections, for the atlas-section field instead.
pub fn parse_atlas_sections(input: &str) -> Result<Vec<(String, String)>> {
    let f: AtlasSectionsFile =
        toml::from_str(input).context("atlas-sections.toml: invalid TOML or does not match the [[section]] schema")?;
    Ok(f.section.into_iter().map(|s| (s.event_id, s.atlas_section)).collect())
}

#[derive(Deserialize)]
struct CoverageManifestFile {
    declared: Vec<String>,
}

/// A flat list of canonical book codes this project claims are FULLY covered: every verse of that book belongs
/// to at least one titled container. Whether the claim is true is a test's job, over the compiled data.
pub fn parse_coverage_manifest(input: &str) -> Result<Vec<String>> {
    let f: CoverageManifestFile =
        toml::from_str(input).context("coverage-manifest.toml: invalid TOML or does not match the 'declared' schema")?;
    Ok(f.declared)
}

#[derive(Deserialize)]
struct LandmarksFile {
    landmark: Vec<Landmark>,
}

#[derive(Deserialize)]
struct PeopleEternalFile {
    #[serde(default)]
    eternal: Vec<EternalToml>,
}

#[derive(Deserialize)]
struct EternalToml {
    id: String,
    #[serde(default)]
    grounds: Vec<String>,
}

/// `(person id, Scripture grounds)` per entry. An entry with no grounds is refused here: eternity is a claim,
/// and a claim carries its Scripture. Whether the person exists is the compile's check, which has them in hand.
pub fn parse_people_eternal(input: &str) -> Result<Vec<(String, Vec<String>)>> {
    let f: PeopleEternalFile = toml::from_str(input).context("people-eternal.toml: invalid TOML or does not match the [[eternal]] schema")?;
    let mut out = Vec::with_capacity(f.eternal.len());
    for e in f.eternal {
        if e.grounds.is_empty() {
            anyhow::bail!("people-eternal.toml: '{}' claims eternity without Scripture grounds", e.id);
        }
        for g in &e.grounds {
            atlas_core::refs::VerseId::parse_canonical(g).map_err(|err| anyhow::anyhow!("people-eternal.toml: '{}' ground {g:?} is not a canonical verse ref: {err}", e.id))?;
        }
        out.push((e.id, e.grounds));
    }
    Ok(out)
}

#[derive(Deserialize)]
struct ParentageFile {
    #[serde(default)]
    parentage: Vec<ParentageToml>,
    #[serde(default)]
    exclusion: Vec<ExclusionToml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ParentageToml {
    parent: String,
    child: String,
    kind: atlas_graph_types::edge::Parentage,
    grounds: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExclusionToml {
    parent: String,
    child: String,
    grounds: Vec<String>,
}

#[derive(Debug)]
pub struct CuratedParentage {
    pub declared: Vec<ParentageSeed>,
    pub excluded: Vec<ParentageExclusion>,
}

fn scripture_justification(file: &str, pair: &str, grounds: &[String]) -> Result<atlas_graph_types::edge::Justification> {
    let mut set = std::collections::BTreeSet::new();
    for g in grounds {
        let vid = atlas_core::refs::VerseId::parse_canonical(g).map_err(|err| anyhow::anyhow!("{file}: {pair} ground {g:?} is not a canonical verse ref: {err}"))?;
        let unit = vid.locus();
        set.insert(atlas_graph_types::edge::Ground::Scripture(atlas_graph_types::text::LocusRange { from: unit.clone(), to: unit }));
    }
    Ok(atlas_graph_types::edge::Justification { text: None, grounds: set })
}

pub fn parse_parentage(input: &str) -> Result<CuratedParentage> {
    let f: ParentageFile = toml::from_str(input).map_err(|_| anyhow::anyhow!("parentage.toml: invalid TOML or does not match the [[parentage]] schema"))?;
    let mut pairs = std::collections::BTreeSet::new();
    let mut declared = Vec::with_capacity(f.parentage.len());
    for e in f.parentage {
        let pair = format!("'{}' -> '{}'", e.parent, e.child);
        if e.kind == atlas_graph_types::edge::Parentage::Natural {
            bail!("parentage.toml: {pair} declares natural parentage, which every undeclared row already is");
        }
        if e.grounds.is_empty() {
            bail!("parentage.toml: {pair} declares {} parentage without Scripture grounds", e.kind.name());
        }
        if !pairs.insert((e.parent.clone(), e.child.clone())) {
            bail!("parentage.toml: {pair} is declared twice");
        }
        let justification = scripture_justification("parentage.toml", &pair, &e.grounds)?;
        declared.push(ParentageSeed { parent: e.parent, child: e.child, parentage: e.kind, justification });
    }
    let mut excluded = Vec::with_capacity(f.exclusion.len());
    for e in f.exclusion {
        let pair = format!("'{}' -> '{}'", e.parent, e.child);
        if e.grounds.is_empty() {
            bail!("parentage.toml: {pair} is excluded without Scripture grounds");
        }
        if !pairs.insert((e.parent.clone(), e.child.clone())) {
            bail!("parentage.toml: {pair} is declared twice");
        }
        scripture_justification("parentage.toml", &pair, &e.grounds)?;
        excluded.push(ParentageExclusion { parent: e.parent, child: e.child });
    }
    Ok(CuratedParentage { declared, excluded })
}

#[derive(Deserialize)]
struct BrethrenFile {
    #[serde(default)]
    brethren: Vec<BrethrenToml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BrethrenToml {
    a: String,
    b: String,
    grounds: Vec<String>,
}

pub fn parse_brethren(input: &str) -> Result<Vec<BrethrenSeed>> {
    let f: BrethrenFile = toml::from_str(input).map_err(|_| anyhow::anyhow!("brethren.toml: invalid TOML or does not match the [[brethren]] schema"))?;
    let mut out = Vec::with_capacity(f.brethren.len());
    for e in f.brethren {
        let pair = format!("'{}' -> '{}'", e.a, e.b);
        if e.grounds.is_empty() {
            bail!("brethren.toml: {pair} are brethren without Scripture grounds");
        }
        let justification = scripture_justification("brethren.toml", &pair, &e.grounds)?;
        out.push(BrethrenSeed { a: e.a, b: e.b, justification });
    }
    Ok(out)
}

pub fn parse_landmarks(input: &str) -> Result<Vec<Landmark>> {
    let f: LandmarksFile =
        toml::from_str(input).context("landmarks.toml: invalid TOML or does not match the [[landmark]] schema")?;
    Ok(f.landmark)
}

#[derive(Deserialize)]
struct PlaceHistoryFile {
    place: Vec<PlaceToml>,
}

#[derive(Deserialize)]
struct PlaceToml {
    id: String,
    #[serde(default, rename = "name")]
    names: Vec<NameToml>,
    #[serde(default, rename = "blurb")]
    blurbs: Vec<BlurbToml>,
    #[serde(default)]
    established: Option<DateClaimToml>,
    #[serde(default)]
    destroyed: Option<DateClaimToml>,
}

#[derive(Deserialize)]
struct NameToml {
    name: String,
    from: i32,
    to: i32,
    #[serde(default)]
    verses: Vec<String>,
}

#[derive(Deserialize)]
struct BlurbToml {
    text: String,
    from: i32,
    to: i32,
    breadth: String,
}

/// Exactly one of `year` or `from`+`to` is required: both, or neither, is a curator authoring mistake and a
/// hard error rather than a soft drop.
#[derive(Deserialize)]
struct DateClaimToml {
    year: Option<i32>,
    from: Option<i32>,
    to: Option<i32>,
    #[serde(default)]
    verses: Vec<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    event: Option<EventId>,
}

fn resolve_date_claim(claim: DateClaimToml, place_id: &str, field: &str) -> Result<PlaceDateClaim> {
    let when = match (claim.year, claim.from, claim.to) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
            bail!("place '{place_id}' {field}: has BOTH 'year' and 'from'/'to' -- use exactly one shape")
        }
        (Some(y), None, None) => TimeRange::new(y, y),
        (None, Some(f), Some(t)) => TimeRange::new(f, t),
        (None, Some(_), None) | (None, None, Some(_)) => {
            bail!("place '{place_id}' {field}: 'from'/'to' range needs BOTH bounds")
        }
        (None, None, None) => {
            bail!("place '{place_id}' {field}: needs either 'year' or 'from'+'to'")
        }
    }
    .map_err(|src| anyhow::anyhow!("place '{place_id}' {field}: {src}"))?;
    Ok(PlaceDateClaim { when, verses: claim.verses, note: claim.note, event: claim.event })
}

pub fn parse_place_history(input: &str) -> Result<Vec<PlaceHistory>> {
    let f: PlaceHistoryFile =
        toml::from_str(input).context("place-history.toml: invalid TOML or does not match the [[place]] schema")?;

    let mut out = Vec::with_capacity(f.place.len());
    for p in f.place {
        let mut names = Vec::with_capacity(p.names.len());
        for n in p.names {
            let when = TimeRange::new(n.from, n.to).map_err(|src| {
                anyhow::anyhow!("place '{}' name '{}' (from={}, to={}): {}", p.id, n.name, n.from, n.to, src)
            })?;
            names.push(PlaceNameEntry { name: n.name, when, verses: n.verses });
        }

        let mut blurbs = Vec::with_capacity(p.blurbs.len());
        for b in p.blurbs {
            let when = TimeRange::new(b.from, b.to).map_err(|src| {
                anyhow::anyhow!("place '{}' blurb '{}...' (from={}, to={}): {}", p.id, &b.text.chars().take(24).collect::<String>(), b.from, b.to, src)
            })?;
            blurbs.push(PlaceBlurbEntry { text: b.text, when, breadth: b.breadth });
        }

        let established = p.established.map(|c| resolve_date_claim(c, &p.id, "established")).transpose()?;
        let destroyed = p.destroyed.map(|c| resolve_date_claim(c, &p.id, "destroyed")).transpose()?;

        out.push(PlaceHistory { id: p.id, names, blurbs, established, destroyed });
    }
    Ok(out)
}

#[derive(Deserialize)]
struct PlaceNamesKjvFile {
    alias: Vec<AliasToml>,
}

#[derive(Deserialize)]
struct AliasToml {
    id: String,
    name: String,
    #[serde(default)]
    verses: Vec<String>,
}

/// A flat array, each row naming its own place id. A row's plain name is wrapped into a `{"kjv": name}`
/// translation map here -- kjv is the only key today, and identity survives future translations -- and its
/// verses are passed through AS-IS: they are plain citation ids, not curator-friendly ranges to expand.
pub fn parse_place_names_kjv(input: &str) -> Result<Vec<PlaceNameAlias>> {
    let f: PlaceNamesKjvFile =
        toml::from_str(input).context("place-names-kjv.toml: invalid TOML or does not match the [[alias]] schema")?;

    Ok(f.alias
        .into_iter()
        .map(|a| PlaceNameAlias {
            id: a.id,
            translations: std::collections::HashMap::from([(atlas_core::translation::DEFAULT_TRANSLATION.to_string(), a.name)]),
            verses: a.verses,
        })
        .collect())
}

#[derive(Deserialize)]
struct PolityToml {
    id: String,
    #[serde(rename = "era")]
    eras: Vec<PolityEraToml>,
}

#[derive(Deserialize)]
struct PolityEraToml {
    name: String,
    from: i32,
    to: i32,
    ref_note: String,
    rings: Vec<Vec<(f64, f64)>>,
    /// Nested subtables attach to exactly the era element they are written under, by TOML's own rule, so no id
    /// or index matching is needed. Both default, so an era that honestly omits them keeps parsing.
    #[serde(default)]
    transition: Option<PolityDeltaToml>,
    #[serde(default)]
    fall: Option<PolityDeltaToml>,
}

#[derive(Deserialize)]
struct PolityDeltaToml {
    event: String,
    #[serde(default)]
    verses: Vec<String>,
    ref_note: String,
    /// Required, with no default: a curator MUST name which era's own `from` year this block belongs to, which
    /// is what makes the mis-attachment this shape once suffered structurally checkable.
    for_era_from: i32,
}

/// `color_key` is left PROVISIONAL at 0 here: a collision-free assignment needs to see every other polity in
/// the roster, which a single file parsed in isolation cannot. The caller overwrites it in one pass once the
/// full sorted roster is in hand.
pub fn parse_polity(input: &str) -> Result<Polity> {
    let f: PolityToml = toml::from_str(input).context("polity TOML: invalid TOML or does not match the id/[[era]] schema")?;
    let eras = f
        .eras
        .into_iter()
        .map(|e| PolityEra {
            name: e.name,
            from: e.from,
            to: e.to,
            ref_note: e.ref_note,
            rings: e.rings,
            transition: e.transition.map(|d| PolityDelta { event: d.event, verses: d.verses, ref_note: d.ref_note, for_era_from: d.for_era_from }),
            fall: e.fall.map(|d| PolityDelta { event: d.event, verses: d.verses, ref_note: d.ref_note, for_era_from: d.for_era_from }),
        })
        .collect();
    Ok(Polity { id: f.id, color_key: 0, eras })
}

#[derive(Deserialize)]
struct LandMaskFile {
    region: Vec<LandMaskRegionToml>,
}

#[derive(Deserialize)]
struct LandMaskRegionToml {
    name: String,
    ref_note: String,
    rings: Vec<Vec<(f64, f64)>>,
}

pub fn parse_land_mask(input: &str) -> Result<Vec<LandMaskRegion>> {
    let f: LandMaskFile =
        toml::from_str(input).context("land-mask.toml: invalid TOML or does not match the [[region]] schema")?;
    Ok(f.region.into_iter().map(|r| LandMaskRegion { name: r.name, ref_note: r.ref_note, rings: r.rings }).collect())
}

#[derive(Deserialize)]
struct CatechismFile {
    part: Vec<CatechismPartToml>,
}

#[derive(Deserialize)]
struct CatechismPartToml {
    id: String,
    title: String,
    item: Vec<CatechismItemToml>,
}

#[derive(Deserialize)]
struct CatechismItemToml {
    id: String,
    name: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    explanation_heading: Option<String>,
    explanation: String,
    #[serde(default)]
    where_written: Option<String>,
    #[serde(default)]
    verses: Vec<String>,
    #[serde(default)]
    ref_note: Option<String>,
}

pub fn parse_catechism(input: &str) -> Result<Vec<CatechismPart>> {
    let f: CatechismFile =
        toml::from_str(input).context("catechism.toml: invalid TOML or does not match the [[part]]/[[part.item]] schema")?;

    let mut parts = Vec::with_capacity(f.part.len());
    for p in f.part {
        let mut items = Vec::with_capacity(p.item.len());
        for it in p.item {
            let mut verses = Vec::new();
            for v in &it.verses {
                expand_verse_ref(v, &it.id, &mut verses)?;
            }
            items.push(CatechismItem {
                id: it.id,
                name: it.name,
                text: it.text,
                explanation_heading: it.explanation_heading.unwrap_or_else(|| "What does this mean?".to_string()),
                explanation: it.explanation,
                where_written: it.where_written,
                verses,
                ref_note: it.ref_note,
                // Question-level citations are merged in separately, after this pure parse, so they are always
                // empty here.
                questions: Vec::new(),
            });
        }
        parts.push(CatechismPart { id: p.id, title: p.title, items });
    }
    Ok(parts)
}

#[derive(Deserialize)]
struct CatechismMappingFileToml {
    file: Vec<MappingFileToml>,
}

#[derive(Deserialize)]
struct MappingFileToml {
    path: String,
    item: String,
    #[serde(default, rename = "override")]
    overrides: Vec<MappingOverrideToml>,
}

#[derive(Deserialize)]
struct MappingOverrideToml {
    item: String,
    questions: Vec<u32>,
}

pub fn parse_catechism_mapping(input: &str) -> Result<Vec<MappingFile>> {
    let f: CatechismMappingFileToml =
        toml::from_str(input).context("catechism-mapping.toml: invalid TOML or does not match the [[file]] schema")?;
    Ok(f.file
        .into_iter()
        .map(|row| MappingFile {
            path: row.path,
            item: row.item,
            overrides: row.overrides.into_iter().map(|o| MappingOverride { item: o.item, questions: o.questions }).collect(),
        })
        .collect())
}

#[derive(Deserialize)]
struct CatechismDeut5File {
    entry: Vec<Deut5EntryToml>,
}

#[derive(Deserialize)]
struct Deut5EntryToml {
    item: String,
    verses: Vec<String>,
    ref_note: String,
}

/// These refs are OUR OWN canonical strings, identical in shape to the main catechism file's, so the range
/// expansion is reused verbatim and the vendored repo's human-readable canonicalizer is deliberately not used.
pub fn parse_catechism_deut5(input: &str) -> Result<Vec<Deut5Entry>> {
    let f: CatechismDeut5File =
        toml::from_str(input).context("catechism-deut5.toml: invalid TOML or does not match the [[entry]] schema")?;
    Ok(f.entry.into_iter().map(|e| Deut5Entry { item: e.item, verses: e.verses, ref_note: e.ref_note }).collect())
}

#[derive(Deserialize)]
struct PeopleGroupsFile {
    #[serde(default)]
    group: Vec<PeopleGroupSeed>,
    #[serde(default)]
    reclassify: Vec<PeopleGroupReclassify>,
    #[serde(default)]
    named_after: Vec<NamedAfterSeed>,
}

/// Three independent arrays: the curated nation seeds, the gentilic reclassification rows, and the eponymy
/// seed rows. Whether an eponymy row's ids name real people or groups is the graph adapter's check, which has
/// the built picture.
pub fn parse_people_group_seeds(input: &str) -> Result<(Vec<PeopleGroupSeed>, Vec<PeopleGroupReclassify>, Vec<NamedAfterSeed>)> {
    let f: PeopleGroupsFile =
        toml::from_str(input).context("people-groups.toml: invalid TOML or does not match the [[group]]/[[reclassify]]/[[named_after]] schema")?;
    Ok((f.group, f.reclassify, f.named_after))
}

#[derive(Deserialize)]
struct FulfillmentsFile {
    #[serde(default)]
    fulfillment: Vec<FulfillmentSeed>,
}

pub fn parse_fulfillments(input: &str) -> Result<Vec<FulfillmentSeed>> {
    let f: FulfillmentsFile = toml::from_str(input).context("fulfillments.toml: invalid TOML or does not match the [[fulfillment]] schema")?;
    Ok(f.fulfillment)
}

#[derive(Deserialize)]
struct TypologyFile {
    #[serde(default)]
    typology: Vec<TypologySeed>,
}

pub fn parse_typology(input: &str) -> Result<Vec<TypologySeed>> {
    let f: TypologyFile = toml::from_str(input).context("typology.toml: invalid TOML or does not match the [[typology]] schema")?;
    Ok(f.typology)
}

#[derive(Deserialize)]
struct ConcordTitlesFile {
    article: Vec<ConcordTitleOverride>,
}

pub fn parse_concord_titles(input: &str) -> Result<Vec<ConcordTitleOverride>> {
    let f: ConcordTitlesFile = toml::from_str(input).context("concord-titles.toml: invalid TOML or does not match the [[article]] schema")?;
    Ok(f.article)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{LandmarkKind, LandmarkSize};

    #[test]
    fn parse_concord_titles_reads_valid_toml() {
        // Arrange
        let toml = "[[article]]\ndocument = \"small-catechism\"\narticle = 2\ntitle = \"I. The Ten Commandments\"\n\n[[article]]\ndocument = \"small-catechism\"\narticle = 6\ntitle = \"V. Confession\"\n";
        // Act
        let titles = parse_concord_titles(toml).unwrap();
        // Assert
        assert_eq!(
            titles,
            vec![
                ConcordTitleOverride { document: "small-catechism".to_string(), article: 2, title: "I. The Ten Commandments".to_string() },
                ConcordTitleOverride { document: "small-catechism".to_string(), article: 6, title: "V. Confession".to_string() },
            ]
        );
    }

    const CONCORD_TITLES_REFUSED: &str = "concord-titles.toml: invalid TOML or does not match the [[article]] schema";

    #[test]
    fn parse_concord_titles_refuses_text_that_is_not_toml_naming_the_file() {
        // Act
        let refused = parse_concord_titles("not valid toml [[[").unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), CONCORD_TITLES_REFUSED);
    }

    #[test]
    fn parse_concord_titles_refuses_an_article_without_a_title_naming_the_file() {
        // Act
        let refused = parse_concord_titles("[[article]]\ndocument = \"small-catechism\"\narticle = 2\n").unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), CONCORD_TITLES_REFUSED);
    }

    #[test]
    fn parse_landmarks_reads_valid_toml() {
        // Act
        let landmarks = parse_landmarks(include_str!("../tests/fixtures/landmarks-sample.toml")).unwrap();
        // Assert
        assert_eq!(
            landmarks,
            vec![
                Landmark { name: "Jordan River".to_string(), kind: LandmarkKind::Water, lat: 31.76, lon: 35.55, size: None },
                Landmark { name: "Mount Sinai".to_string(), kind: LandmarkKind::Mountain, lat: 28.54, lon: 33.97, size: None },
                Landmark { name: "Negev".to_string(), kind: LandmarkKind::Region, lat: 31.24, lon: 34.84, size: Some(LandmarkSize::Large) },
            ]
        );
    }

    #[test]
    fn parse_polity_reads_valid_toml_and_computes_color_key() {
        let polity = parse_polity(include_str!("../tests/fixtures/polities-sample.toml")).unwrap();
        assert_eq!(polity.id, "testland");
        assert_eq!(polity.eras.len(), 2);
        assert_eq!(polity.eras[0].name, "Testland");
        assert_eq!(polity.eras[0].from, -2000);
        assert_eq!(polity.eras[0].to, -1500);
        assert_eq!(polity.eras[0].rings.len(), 1);
        assert_eq!(polity.eras[0].rings[0].len(), 5);
        assert_eq!(polity.eras[0].rings[0][0], (10.0, 10.0), "rings are [lat, lon], first pair verbatim");
        assert_eq!(polity.eras[1].name, "Greater Testland");
        assert!(polity.eras[0].transition.is_none(), "first era's own [era.transition] is honestly absent in the fixture");
        assert!(polity.eras[0].fall.is_none());

        let transition = polity.eras[1].transition.as_ref().expect("second era carries a transition in the fixture");
        assert_eq!(transition.event, "Testland expands");
        assert_eq!(transition.verses, vec!["GEN.1.1".to_string()]);
        assert_eq!(transition.ref_note, "synthetic fixture, not a real citation");
        assert_eq!(transition.for_era_from, -1499, "fix round 1 (I1): echoes the SAME era's own from it's actually attached to");

        let fall = polity.eras[1].fall.as_ref().expect("second era carries a fall in the fixture");
        assert_eq!(fall.event, "Greater Testland falls");
        assert_eq!(fall.verses, vec!["GEN.1.2".to_string(), "GEN.1.3".to_string()]);
        assert_eq!(fall.for_era_from, -1499);

        assert_eq!(polity.color_key, 0);
    }

    #[test]
    fn parse_polity_rejects_malformed_toml() {
        assert!(parse_polity("not = [valid").is_err());
        assert!(parse_polity("id = \"x\"").is_err(), "missing [[era]] array entirely");
    }

    #[test]
    fn parse_land_mask_reads_valid_toml() {
        let toml = r#"
[[region]]
name = "Testland coast"
ref_note = "test fixture, not a real coastline"
rings = [
  [[10.0, 10.0], [10.0, 20.0], [20.0, 20.0], [20.0, 10.0], [10.0, 10.0]]
]

[[region]]
name = "Testisle"
ref_note = "test fixture"
rings = [
  [[30.0, 30.0], [30.0, 32.0], [32.0, 31.0], [30.0, 30.0]]
]
"#;
        let regions = parse_land_mask(toml).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].name, "Testland coast");
        assert_eq!(regions[0].rings.len(), 1);
        assert_eq!(regions[0].rings[0][0], (10.0, 10.0), "rings are [lat, lon], first pair verbatim");
        assert_eq!(regions[1].name, "Testisle");
    }

    #[test]
    fn parse_land_mask_rejects_malformed_toml() {
        assert!(parse_land_mask("not = [valid").is_err());
        assert!(parse_land_mask("foo = 1").is_err(), "missing [[region]] array entirely");
    }

    #[test]
    fn parse_catechism_reads_valid_toml_with_defaults_applied() {
        let toml = r#"
[[part]]
id = "ten-commandments"
title = "The Ten Commandments"

  [[part.item]]
  id = "commandment-1"
  name = "The First Commandment"
  text = "Thou shalt have no other gods."
  explanation = "We should fear, love, and trust in God above all things."

  [[part.item]]
  id = "commandments-close"
  name = "What Does God Say of All These Commandments?"
  text = "I the LORD thy God am a jealous God..."
  explanation = "God threatens to punish all that transgress these commandments."
  verses = ["EXO.20.5-6"]
  ref_note = "f. read as covering v.6"

[[part]]
id = "baptism"
title = "The Sacrament of Holy Baptism"

  [[part.item]]
  id = "baptism-1"
  name = "Baptism — Part the First"
  explanation_heading = "What is Baptism?"
  explanation = "Baptism is not simple water only..."
  where_written = "Christ, our Lord, says..."
  verses = ["MAT.28.19"]
"#;
        let parts = parse_catechism(toml).unwrap();
        assert_eq!(parts.len(), 2);

        let commandments = &parts[0];
        assert_eq!(commandments.id, "ten-commandments");
        assert_eq!(commandments.title, "The Ten Commandments");
        assert_eq!(commandments.items.len(), 2);

        let first = &commandments.items[0];
        assert_eq!(first.id, "commandment-1");
        assert_eq!(first.text.as_deref(), Some("Thou shalt have no other gods."));
        assert_eq!(first.explanation_heading, "What does this mean?");
        assert_eq!(first.where_written, None);
        assert!(first.verses.is_empty());
        assert_eq!(first.ref_note, None);

        let close = &commandments.items[1];
        assert_eq!(close.verses, vec!["EXO.20.5".to_string(), "EXO.20.6".to_string()]);
        assert_eq!(close.ref_note.as_deref(), Some("f. read as covering v.6"));

        let baptism = &parts[1];
        let b1 = &baptism.items[0];
        assert_eq!(b1.text, None, "Baptism items have no separate prompt text -- see CatechismItem's own doc comment");
        assert_eq!(b1.explanation_heading, "What is Baptism?");
        assert_eq!(b1.where_written.as_deref(), Some("Christ, our Lord, says..."));
        assert_eq!(b1.verses, vec!["MAT.28.19".to_string()]);
    }

    #[test]
    fn parse_catechism_rejects_malformed_toml() {
        assert!(parse_catechism("not = [valid").is_err());
        assert!(parse_catechism("id = \"x\"").is_err(), "missing [[part]] array entirely");
    }

    #[test]
    fn parse_catechism_rejects_an_unparseable_verse_ref() {
        let toml = r#"
[[part]]
id = "p"
title = "P"
  [[part.item]]
  id = "i1"
  name = "N"
  explanation = "E"
  verses = ["not-a-ref"]
"#;
        let err = parse_catechism(toml).unwrap_err();
        assert!(err.to_string().contains("i1"), "{err}");
    }

    #[test]
    fn parse_catechism_mapping_reads_valid_toml_with_and_without_overrides() {
        let toml = r#"
[[file]]
path = "resources/02.1-The-First-Commandment.yaml"
item = "commandment-1"

[[file]]
path = "resources/05.2.1-Confession-and-Absolution.yaml"
item = "confession-1"

  [[file.override]]
  item = "confession-2"
  questions = [6, 7, 9]
"#;
        let rows = parse_catechism_mapping(toml).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].path, "resources/02.1-The-First-Commandment.yaml");
        assert_eq!(rows[0].item, "commandment-1");
        assert!(rows[0].overrides.is_empty());
        assert_eq!(rows[1].overrides.len(), 1);
        assert_eq!(rows[1].overrides[0].item, "confession-2");
        assert_eq!(rows[1].overrides[0].questions, vec![6, 7, 9]);
    }

    #[test]
    fn parse_catechism_mapping_rejects_malformed_toml() {
        assert!(parse_catechism_mapping("not = [valid").is_err());
        assert!(parse_catechism_mapping("foo = 1").is_err(), "missing [[file]] array entirely");
    }

    #[test]
    fn parse_catechism_deut5_reads_valid_toml() {
        let toml = r#"
[[entry]]
item = "commandment-1"
verses = ["DEU.5.7"]
ref_note = "test note"

[[entry]]
item = "commandments-close"
verses = ["DEU.5.9-10"]
ref_note = "another note"
"#;
        let entries = parse_catechism_deut5(toml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].item, "commandment-1");
        assert_eq!(entries[0].verses, vec!["DEU.5.7".to_string()]);
        assert_eq!(entries[0].ref_note, "test note");
        assert_eq!(entries[1].verses, vec!["DEU.5.9-10".to_string()]);
    }

    #[test]
    fn parse_catechism_deut5_rejects_malformed_toml() {
        assert!(parse_catechism_deut5("not = [valid").is_err());
        assert!(parse_catechism_deut5("item = \"x\"").is_err(), "missing [[entry]] array entirely");
    }

    #[test]
    fn expand_verse_ref_handles_single_and_range() {
        let mut out = Vec::new();
        expand_verse_ref("EXO.12.37", "e1", &mut out).unwrap();
        assert_eq!(out, vec!["EXO.12.37".to_string()]);

        let mut out2 = Vec::new();
        expand_verse_ref("GEN.12.14-20", "e2", &mut out2).unwrap();
        assert_eq!(out2.len(), 7);
        assert_eq!(out2.first().unwrap(), "GEN.12.14");
        assert_eq!(out2.last().unwrap(), "GEN.12.20");
    }

    #[test]
    fn parse_event_witnesses_reads_a_flat_witness_list_with_translation_indirection() {
        let toml = r#"
[[witness]]
event_id = "pw_golgotha"
book = "MAT"
verses = ["MAT.27.33-50"]
ref_note = "Matthew 27:33-50 read directly"
robertson_section = "Robertson (1922) §164"

[[witness]]
event_id = "pw_golgotha"
book = "JHN"
verses = ["JHN.19.17-30"]
"#;
        let rows = parse_event_witnesses(toml).unwrap();
        assert_eq!(rows.len(), 2);

        let (event_id, mat) = &rows[0];
        assert_eq!(event_id, "pw_golgotha");
        assert_eq!(mat.book, "MAT");
        let kjv = mat.translations.get("kjv").expect("kjv translation must be populated");
        assert_eq!(kjv.len(), 18);
        assert_eq!(kjv.first().unwrap(), "MAT.27.33");
        assert_eq!(kjv.last().unwrap(), "MAT.27.50");
        assert_eq!(mat.ref_note.as_deref(), Some("Matthew 27:33-50 read directly"));
        assert_eq!(mat.robertson_section.as_deref(), Some("Robertson (1922) §164"));

        let (event_id2, jhn) = &rows[1];
        assert_eq!(event_id2, "pw_golgotha");
        assert_eq!(jhn.book, "JHN");
        assert_eq!(jhn.ref_note, None);
    }

    #[test]
    fn parse_event_witnesses_rejects_malformed_toml() {
        assert!(parse_event_witnesses("not = [valid").is_err());
        assert!(parse_event_witnesses("foo = 1").is_err(), "missing [[witness]] array entirely");
    }

    #[test]
    fn parse_event_witnesses_rejects_an_unparseable_verse_ref() {
        let toml = r#"
[[witness]]
event_id = "e1"
book = "MAT"
verses = ["not-a-ref"]
"#;
        let err = parse_event_witnesses(toml).unwrap_err();
        assert!(err.to_string().contains("e1"), "{err}");
    }

    #[test]
    fn parse_place_names_kjv_reads_a_flat_alias_list_with_translation_indirection() {
        let toml = r#"
[[alias]]
id = "cush-2"
name = "Ethiopia"
verses = ["GEN.2.13"]

[[alias]]
id = "tigris"
name = "Hiddekel"
verses = ["GEN.2.14", "DAN.10.4"]
"#;
        let rows = parse_place_names_kjv(toml).unwrap();
        assert_eq!(rows.len(), 2);

        assert_eq!(rows[0].id, "cush-2");
        assert_eq!(rows[0].translations.get("kjv").map(String::as_str), Some("Ethiopia"));
        assert_eq!(rows[0].verses, vec!["GEN.2.13".to_string()]);

        assert_eq!(rows[1].id, "tigris");
        assert_eq!(rows[1].translations.get("kjv").map(String::as_str), Some("Hiddekel"));
        assert_eq!(rows[1].verses, vec!["GEN.2.14".to_string(), "DAN.10.4".to_string()]);
    }

    #[test]
    fn parse_place_names_kjv_rejects_malformed_toml() {
        assert!(parse_place_names_kjv("not = [valid").is_err());
        assert!(parse_place_names_kjv("foo = 1").is_err(), "missing [[alias]] array entirely");
    }

    #[test]
    fn parse_people_group_seeds_reads_all_three_arrays() {
        let toml = r#"
[[group]]
id = "ammonites"
label = "Ammonites"

[[group]]
id = "moabites"
label = "Moabites"

[[reclassify]]
person_slug = "jebusite_748"
reason = "Gen-10 gentilic collective (GEN 10:16), misfiled as a person."

[[named_after]]
namesake_kind = "people_group"
namesake_id = "ammonites"
eponym = "ben-ammi_451"
text = "She called his name Ben-ammi: the same is the father of the children of Ammon."
grounds = [{ from = "GEN.19.38" }]

[[named_after]]
namesake_kind = "people_group"
namesake_id = "edomites"
eponym = "esau_1216"
grounds = [{ from = "GEN.36.8", to = "GEN.36.9" }, { from = "GEN.25.30" }]
"#;
        let (groups, reclassify, named_after) = parse_people_group_seeds(toml).unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].id, "ammonites");
        assert_eq!(groups[0].label, "Ammonites");
        assert_eq!(groups[1].id, "moabites");

        assert_eq!(reclassify.len(), 1);
        assert_eq!(reclassify[0].person_slug, "jebusite_748");
        assert!(reclassify[0].reason.contains("Gen-10"));

        assert_eq!(named_after.len(), 2);
        assert_eq!(named_after[0].namesake_kind, "people_group");
        assert_eq!(named_after[0].namesake_id, "ammonites");
        assert_eq!(named_after[0].eponym, "ben-ammi_451");
        assert!(named_after[0].text.as_deref().unwrap().contains("Ben-ammi"));
        assert_eq!(named_after[0].grounds.len(), 1);
        assert_eq!(named_after[0].grounds[0].from, "GEN.19.38");
        assert_eq!(named_after[0].grounds[0].to, None, "single-verse ground: to defaults to absent, not a copy of from");

        assert_eq!(named_after[1].namesake_id, "edomites");
        assert_eq!(named_after[1].text, None, "text is optional");
        assert_eq!(named_after[1].grounds.len(), 2, "Edomites carries TWO scripture grounds (GEN 36:8-9 and GEN 25:30)");
        assert_eq!(named_after[1].grounds[0].from, "GEN.36.8");
        assert_eq!(named_after[1].grounds[0].to.as_deref(), Some("GEN.36.9"));
        assert_eq!(named_after[1].grounds[1].from, "GEN.25.30");
        assert_eq!(named_after[1].grounds[1].to, None);
    }

    #[test]
    fn parse_people_group_seeds_defaults_every_array_to_empty_when_absent() {
        let (groups, reclassify, named_after) = parse_people_group_seeds("").unwrap();
        assert!(groups.is_empty());
        assert!(reclassify.is_empty());
        assert!(named_after.is_empty());
    }

    #[test]
    fn parse_people_group_seeds_rejects_malformed_toml() {
        assert!(parse_people_group_seeds("not = [valid").is_err());
    }

    fn john_3_16() -> atlas_graph_types::edge::Justification {
        let unit = atlas_graph_types::text::BibleLocus::whole(atlas_graph_types::text::VerseRef { book: 42, chapter: 3, verse: 16 });
        atlas_graph_types::edge::Justification {
            text: None,
            grounds: [atlas_graph_types::edge::Ground::Scripture(atlas_graph_types::text::LocusRange { from: unit.clone(), to: unit })].into_iter().collect(),
        }
    }

    #[test]
    fn parse_parentage_reads_a_declared_parentage_with_its_scripture_grounds() {
        // Arrange
        let toml = "[[parentage]]\nparent = \"god_1324\"\nchild = \"jesus_905\"\nkind = \"eternal\"\ngrounds = [\"JHN.3.16\"]\n";
        // Act
        let seeds = parse_parentage(toml).unwrap().declared;
        // Assert
        assert_eq!(
            seeds,
            vec![ParentageSeed {
                parent: "god_1324".to_string(),
                child: "jesus_905".to_string(),
                parentage: atlas_graph_types::edge::Parentage::Eternal,
                justification: john_3_16(),
            }]
        );
    }

    #[test]
    fn parse_parentage_refuses_a_natural_declaration_because_every_undeclared_row_is_natural() {
        // Act
        let refused = parse_parentage("[[parentage]]\nparent = \"adam_78\"\nchild = \"seth_2504\"\nkind = \"natural\"\ngrounds = [\"GEN.5.3\"]\n").unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), "parentage.toml: 'adam_78' -> 'seth_2504' declares natural parentage, which every undeclared row already is");
    }

    #[test]
    fn parse_parentage_refuses_a_declaration_without_scripture_grounds() {
        // Act
        let refused = parse_parentage("[[parentage]]\nparent = \"god_1324\"\nchild = \"adam_78\"\nkind = \"created\"\ngrounds = []\n").unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), "parentage.toml: 'god_1324' -> 'adam_78' declares created parentage without Scripture grounds");
    }

    #[test]
    fn parse_parentage_refuses_a_ground_that_is_not_a_canonical_verse() {
        // Act
        let refused = parse_parentage("[[parentage]]\nparent = \"god_1324\"\nchild = \"adam_78\"\nkind = \"created\"\ngrounds = [\"Luke 3:38\"]\n").unwrap_err();
        // Assert
        assert!(refused.to_string().starts_with("parentage.toml: 'god_1324' -> 'adam_78' ground \"Luke 3:38\" is not a canonical verse ref: "), "{refused}");
    }

    #[test]
    fn parse_parentage_refuses_a_pair_declared_twice() {
        // Arrange
        let entry = "[[parentage]]\nparent = \"god_1324\"\nchild = \"adam_78\"\nkind = \"created\"\ngrounds = [\"LUK.3.38\"]\n";
        // Act
        let refused = parse_parentage(&format!("{entry}{entry}")).unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), "parentage.toml: 'god_1324' -> 'adam_78' is declared twice");
    }

    #[test]
    fn parse_parentage_refuses_a_kind_outside_the_vocabulary() {
        // Act
        let refused = parse_parentage("[[parentage]]\nparent = \"a\"\nchild = \"b\"\nkind = \"adoptive\"\ngrounds = [\"GEN.1.1\"]\n").unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), "parentage.toml: invalid TOML or does not match the [[parentage]] schema");
    }

    #[test]
    fn parse_parentage_reads_an_exclusion_of_a_pair_the_source_wrongly_states() {
        // Arrange
        let toml = "[[exclusion]]\nparent = \"mary_1938\"\nchild = \"james_719\"\ngrounds = [\"MAT.13.55\"]\n";
        // Act
        let parentage = parse_parentage(toml).unwrap();
        // Assert
        assert_eq!(
            (parentage.declared, parentage.excluded),
            (vec![], vec![ParentageExclusion { parent: "mary_1938".to_string(), child: "james_719".to_string() }])
        );
    }

    #[test]
    fn parse_parentage_refuses_an_exclusion_without_scripture_grounds() {
        // Act
        let refused = parse_parentage("[[exclusion]]\nparent = \"mary_1938\"\nchild = \"james_719\"\ngrounds = []\n").unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), "parentage.toml: 'mary_1938' -> 'james_719' is excluded without Scripture grounds");
    }

    #[test]
    fn parse_parentage_refuses_a_pair_both_declared_and_excluded() {
        // Arrange
        let toml = "[[parentage]]\nparent = \"mary_1938\"\nchild = \"jesus_905\"\nkind = \"virgin\"\ngrounds = [\"LUK.1.35\"]\n[[exclusion]]\nparent = \"mary_1938\"\nchild = \"jesus_905\"\ngrounds = [\"LUK.1.35\"]\n";
        // Act
        let refused = parse_parentage(toml).unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), "parentage.toml: 'mary_1938' -> 'jesus_905' is declared twice");
    }

    #[test]
    fn parse_brethren_reads_brethren_with_their_scripture_grounds() {
        // Arrange
        let toml = "[[brethren]]\na = \"jesus_905\"\nb = \"james_719\"\ngrounds = [\"JHN.3.16\"]\n";
        // Act
        let brethren = parse_brethren(toml).unwrap();
        // Assert
        assert_eq!(brethren, vec![BrethrenSeed { a: "jesus_905".to_string(), b: "james_719".to_string(), justification: john_3_16() }]);
    }

    #[test]
    fn parse_brethren_refuses_brethren_without_scripture_grounds() {
        // Act
        let refused = parse_brethren("[[brethren]]\na = \"jesus_905\"\nb = \"james_719\"\ngrounds = []\n").unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), "brethren.toml: 'jesus_905' -> 'james_719' are brethren without Scripture grounds");
    }

    #[test]
    fn parse_brethren_refuses_a_ground_that_is_not_a_canonical_verse() {
        // Act
        let refused = parse_brethren("[[brethren]]\na = \"jesus_905\"\nb = \"james_719\"\ngrounds = [\"Gal 1:19\"]\n").unwrap_err();
        // Assert
        assert!(refused.to_string().starts_with("brethren.toml: 'jesus_905' -> 'james_719' ground \"Gal 1:19\" is not a canonical verse ref: "), "{refused}");
    }

    #[test]
    fn parse_brethren_refuses_text_outside_its_schema() {
        // Act
        let refused = parse_brethren("[[brethren]]\na = \"x\"\n").unwrap_err();
        // Assert
        assert_eq!(refused.to_string(), "brethren.toml: invalid TOML or does not match the [[brethren]] schema");
    }
}
