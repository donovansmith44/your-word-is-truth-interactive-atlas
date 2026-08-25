//! Batch RED-1 (owner orders 2026-08-25: "Red letters on Jesus' words in
//! every translation"; "SpokenAt is another edge"): turns
//! `atlas_etl::red_letter::RedLetterCorpus` (the parsed + aligned OSIS
//! red-letter source, `red_letter.rs`'s own module doc comment) into
//! `SpokenBy` rows (one per maximal contiguous verse RANGE of the verse
//! set, decision 3) and DERIVED `SpokenAt` rows (decision 3 / red1-
//! scouting.md's own "v1 data plan"). Mirrors `kretzmann_adapter.rs`'s own
//! "parsed corpus -> rows, self-contained" NORMALIZE-eligibility shape for
//! `SpokenBy` (needs only `ctx.red_letter` + `ctx.kjv_canon`, no OTHER
//! pass's output); `SpokenAt` is the one exception -- it reads `ctx.graph.
//! attests`/`ctx.graph.located_at`, so this adapter's own `normalize` call
//! MUST run AFTER `event_world::normalize` within the SAME NormalizePass
//! (pipeline.rs's own call-order list; both `attests`/`located_at` are
//! fully built there, before any OTHER pass runs, so this still needs no
//! new pipeline stage).
//!
//! JESUS PERSON ID: `jesus_905` -- verified directly against the vendored
//! Theographic `people.json` (batch-time grep, not assumed): TWO records
//! carry `"name": "Jesus"` (`jesus_904`/`jesus_905`), a real, disclosed
//! ambiguity in the source data, resolved by content, not guessed --
//! `jesus_905` carries `"surname": "Christ"`, `birthYear: "-4"`/
//! `deathYear: "30"`, `verseCount: 1831`, and an Easton's "Christ"
//! dictionary link; `jesus_904` is a mis-joined record actually describing
//! "Jesus, who is called Justus" (Col.4.11 -- its own `dictText` opens "A
//! Jewish Christian surnamed Justus"), `verseCount: 1`, and is itself
//! flagged `"ambiguous": true` by Theographic's own data. `jesus_905` is
//! unambiguously Jesus Christ; see batch-red1-report.md for the full
//! disambiguation record.
//!
//! SpokenBy DIRECTION (graph.rs's own `build_indexes` doc comment
//! precedent -- forward label decides subject/object, not a node-type
//! rule): `SpokenBy => "spoken-by" / "speech-of"` reads naturally as
//! "[this verse text] spoken-by [Jesus]" -- subject is the locus's own
//! FIRST verse (the `attests`/`comments_on` precedent: full range stays on
//! the row, first verse is the edge endpoint), object is the speaker.
//! `SpokenAt => "spoken-at" / "site-of-speech"` the same shape, one relation
//! wider (place instead of person) -- "[this verse text] spoken-at
//! [place]", mirroring `LocatedAt`'s own "event located-at place" polarity.
//!
//! SpokenAt DERIVATION (red1-scouting.md, decision 3): for each SpokenBy
//! range R, every `(event, place)` pair where `LocatedAt(event, place)`
//! exists AND `Attests(event, A)` exists AND R is FULLY CONTAINED in A
//! (`R.from >= A.from && R.to <= A.to`, canon-order comparison) is a
//! candidate; candidates are deduped by DISTINCT PLACE (decision 3: "emit
//! one row per distinct place, honestly" -- two events independently
//! attesting overlapping/nested ranges that resolve to the SAME place
//! collapse to one row, not two identical ones). `provenance` is the
//! ATTESTS row's own provenance (`"the event's own attestation"`,
//! red1-scouting.md verbatim); `justification` grounds in BOTH the
//! attested range (why this place) and the SpokenBy locus itself (self-
//! attestation, the SAME pattern `SpokenBy`'s own justification already
//! uses), plus a short derivation note naming the grounding event.
//! CONFIDENCE, disclosed: `atlas_graph_types::ingest::Confidence::Derived`
//! is the CONCEPTUAL grounding red1-scouting.md names ("the
//! Confidence::Derived variant exists for exactly this") -- but as of this
//! batch, NO row in this codebase's graph carries a wired `Confidence`
//! field at all (`ProvenanceId` is a plain `Interned`/`String` locator, not
//! the `Provenance{source,locator,confidence}` struct; confirmed by a
//! fresh grep: zero real `Confidence::` usage anywhere in `atlas-graph`
//! before this batch, and `people.rs`'s own `Confidence::Imported`
//! reference is doctrinal prose, not a wired field either). This adapter
//! follows the SAME house convention every other batch already does:
//! confidence rides the descriptive PROVENANCE STRING/doc comment, not an
//! unwired typed field -- `PROVENANCE_KIND` below is `"red-letter"`
//! (SpokenBy, asserted from the source) vs the SpokenAt row's own carried-
//! through `att.provenance` (already descriptive, e.g. `"event-
//! witnesses"`), with "derived" stated in this row's own `justification.
//! text` instead. A real, disclosed engineering call, not silently
//! deviating from the scouting memo's own words.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::edge::{Ground, Justification, SpokenAt, SpokenBy};
use atlas_graph_types::id::PersonId;
use atlas_graph_types::text::{BibleLocusRange, Locus, VerseRef};

use crate::pipeline::BuildCtx;

/// The Theographic person id for Jesus Christ -- module doc comment has
/// the full disambiguation record (two same-named candidates, resolved by
/// content).
pub const JESUS_PERSON_ID: &str = "jesus_905";

const SPOKEN_BY_PROVENANCE: &str = "red-letter";

#[derive(Debug, Clone, Copy, Default)]
pub struct RedLetterAdapterStats {
    pub spoken_by_rows: usize,
    pub spoken_at_rows: usize,
    /// Denominator for the coverage disclosure (decision 3: "coverage
    /// counts asserted + disclosed... expect partial") -- every SpokenBy
    /// range this batch built, regardless of whether it found a place.
    pub spoken_at_ranges_total: usize,
    /// Numerator: SpokenBy ranges that resolved to >=1 place.
    pub spoken_at_ranges_covered: usize,
}

/// Canon-order comparison key for a `VerseRef` -- the shared ordering both
/// the SpokenAt containment check and the event-span reconstruction below
/// use (module doc comment's own SpokenAt derivation law).
fn verse_key(v: &VerseRef) -> (u8, u16, u16) {
    (v.book, v.chapter, v.verse)
}

/// Resolves a (book_index, chapter) pair's own verse count from the
/// build's own `Canon` -- `Canon.books` is not guaranteed densely indexed
/// by book_index (a test fixture may carry a SUBSET; `kjv_adapter.rs`'s own
/// module doc comment has the full "resolve, don't assume positional"
/// reasoning), so this resolves by CODE, the same way `kjv_adapter::
/// ordered_verses_from_canon` already does.
fn chapter_verse_count(canon: &atlas_core::data::Canon, book_index: u8, chapter: u16) -> Option<u16> {
    canon
        .books
        .iter()
        .find(|b| atlas_core::canon::resolve_alias(&b.code).map(|id| id.0) == Some(book_index))
        .and_then(|b| b.chapters.get((chapter.saturating_sub(1)) as usize).copied())
}

fn locus_range(from: (u8, u16, u16), to: (u8, u16, u16)) -> Option<BibleLocusRange> {
    let f = Locus::whole(VerseRef { book: from.0, chapter: from.1, verse: from.2 });
    let t = Locus::whole(VerseRef { book: to.0, chapter: to.1, verse: to.2 });
    BibleLocusRange::new(f, t).ok()
}

/// Pipeline-facing NORMALIZE entry point (module doc comment has the full
/// ordering requirement: must run after `event_world::normalize` within
/// the same pass). Absent `ctx.red_letter` (every test fixture that
/// doesn't supply real red-letter data) is a true no-op -- the SAME
/// "absent == honestly empty, not a placeholder" treatment `ctx.kretzmann`/
/// `ctx.concord`/`ctx.brainfuel` already get.
pub fn normalize(ctx: &mut BuildCtx) -> RedLetterAdapterStats {
    let mut stats = RedLetterAdapterStats::default();
    let Some(corpus) = ctx.red_letter else {
        return stats;
    };

    let speaker = PersonId::new(JESUS_PERSON_ID.to_string());
    // DEFENSIVE (a real, self-caught gap: `window_law.rs`'s own real-`raw_
    // dir`-plus-`empty_atlas()` fixture builds from the REAL vendored
    // `data/raw/red-letter/` -- present on disk once vendored -- while
    // supplying NO Theographic person data at all, since its own scope is
    // reading-order only, "an empty AtlasData is the right fixture" per
    // that file's own doc comment): unlike `kretzmann`/`concord`/
    // `brainfuel` (each self-contained, building their OWN nodes), THIS
    // adapter's own rows reference a node `person_adapter::normalize`
    // builds, a real cross-adapter coupling no sibling adapter has. Rather
    // than let `law_check::every_authored_edge_resolves` catch this
    // downstream as a dangling reference (correct, but a worse failure
    // mode than simply not authoring rows a lawful build can't back), this
    // checks the SAME precondition up front: no Jesus Person node, no
    // SpokenBy/SpokenAt rows at all -- the SAME "absent == honestly empty,
    // not a placeholder" treatment `ctx.red_letter` itself already gets.
    // NEVER true of a real compile (`bins/compile_graph.rs`'s own AtlasData
    // is real Theographic data, confirmed to carry `jesus_905` at batch
    // time) -- this guard exists for a minimal/degraded test fixture that
    // vendors red-letter data but not people data, never for production.
    if !ctx.graph.nodes.contains_key(&speaker.clone().erase()) {
        return stats;
    }

    let canon = ctx.kjv_canon;
    let counts = |book: u8, chapter: u16| chapter_verse_count(canon, book, chapter);
    let ranges = atlas_etl::red_letter::contiguous_ranges(&corpus.verses, &counts);

    // SpokenAt's own event->place lookup, built ONCE (not per-range) --
    // `ctx.graph.located_at` is already fully populated by `event_world::
    // normalize`, earlier in this SAME pass (module doc comment).
    let mut located_at_by_event: BTreeMap<String, Vec<atlas_graph_types::id::PlaceId>> = BTreeMap::new();
    for loc in &ctx.graph.located_at {
        located_at_by_event.entry(loc.event.0.clone()).or_default().push(loc.place.clone());
    }

    // Each event's own OVERALL attested SPAN (min..max verse across every
    // one of its own `attests` rows), reconstructed once here -- NOT read
    // off any single `attests` ROW directly. `event_world::populate_
    // nodes_and_direct_rows` emits ONE `Attests` row PER WITNESSED VERSE
    // (that module's own M-C2 fix doc comment: "one row per witness VERSE
    // rather than per verse GROUP"), so a multi-verse event's own
    // attestation is the UNION of many single-verse rows, never one row
    // already spanning it -- red1-scouting.md's own "a speech locus
    // falling inside an event's attested verse RANGE" means THIS
    // reconstructed bounding span, the real, only sense in which an event
    // "has" a range at all.
    struct EventSpan {
        min: (u8, u16, u16),
        max: (u8, u16, u16),
        provenance: String,
    }
    let mut event_spans: BTreeMap<String, EventSpan> = BTreeMap::new();
    for att in &ctx.graph.attests {
        let f = verse_key(&att.attestation.from.unit);
        let t = verse_key(&att.attestation.to.unit);
        event_spans
            .entry(att.event.0.clone())
            .and_modify(|s| {
                if f < s.min {
                    s.min = f;
                }
                if t > s.max {
                    s.max = t;
                }
            })
            .or_insert_with(|| EventSpan { min: f, max: t, provenance: att.provenance.clone() });
    }

    for (from, to) in ranges {
        let Some(range) = locus_range(from, to) else { continue };
        let range_from = verse_key(&range.from.unit);
        let range_to = verse_key(&range.to.unit);

        let mut spoken_by_grounds = BTreeSet::new();
        spoken_by_grounds.insert(Ground::Scripture(range.clone()));
        ctx.graph.spoken_by.push(SpokenBy {
            locus: range.clone(),
            speaker: speaker.clone(),
            provenance: SPOKEN_BY_PROVENANCE.to_string(),
            justification: Justification { text: None, grounds: spoken_by_grounds },
        });
        stats.spoken_by_rows += 1;

        // SpokenAt derivation (module doc comment): every event whose own
        // reconstructed bounding span fully contains this SpokenBy range
        // AND carries >=1 LocatedAt row contributes candidate places;
        // deduped by distinct place across ALL contributing events for
        // this one range (decision 3: "emit one row per distinct place").
        stats.spoken_at_ranges_total += 1;
        let mut places_seen: BTreeSet<String> = BTreeSet::new();
        for (event_id, span) in &event_spans {
            let Some(places) = located_at_by_event.get(event_id) else { continue };
            if !(range_from >= span.min && range_to <= span.max) {
                continue;
            }
            let Some(attested_range) = locus_range(span.min, span.max) else { continue };
            for place in places {
                if !places_seen.insert(place.0.clone()) {
                    continue; // decision 3: one row per DISTINCT place, honestly
                }
                let mut grounds = BTreeSet::new();
                grounds.insert(Ground::Scripture(attested_range.clone()));
                grounds.insert(Ground::Scripture(range.clone()));
                ctx.graph.spoken_at.push(SpokenAt {
                    locus: range.clone(),
                    place: place.clone(),
                    provenance: span.provenance.clone(),
                    justification: Justification { text: Some(format!("derived: falls within {event_id}'s own attested range")), grounds },
                });
                stats.spoken_at_rows += 1;
            }
        }
        if !places_seen.is_empty() {
            stats.spoken_at_ranges_covered += 1;
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, CanonBook, Event, Place};
    use atlas_etl::red_letter::{RedLetterCorpus, RedLetterVerse};
    use std::collections::HashMap;

    fn canon_matthew() -> Canon {
        // Matthew (book index 39) with real-shaped chapter counts, enough
        // for these tests' own small verse ranges.
        let mut chapters = vec![25u16; 28]; // placeholder width; only ch.4/5 matter below
        chapters[3] = 25; // Matt 4 has 25 verses (real)
        chapters[4] = 48; // Matt 5 has 48 verses (real)
        Canon { books: vec![CanonBook { code: "MAT".into(), name: "Matthew".into(), chapters }] }
    }

    fn corpus_with(verses: Vec<RedLetterVerse>) -> RedLetterCorpus {
        RedLetterCorpus { verses, stats: Default::default() }
    }

    fn rv(chapter: u16, verse: u16, span_len: usize) -> RedLetterVerse {
        RedLetterVerse { book_index: 39, chapter, verse, spans: vec![(0, span_len)] }
    }

    /// Every real compile ALWAYS has a Jesus Person node by the time this
    /// adapter runs (`person_adapter::normalize` runs earlier in the SAME
    /// pass) -- this module's own defensive-guard doc comment (`normalize`
    /// above) explains the one degraded fixture (`window_law.rs`) that
    /// doesn't. Every test in THIS file wants the ordinary, real-shaped
    /// precondition, so `ctx_with` inserts it once here, centrally.
    fn insert_jesus_node(ctx: &mut BuildCtx) {
        use atlas_graph_types::node::{Node, NodePayload};
        let id = PersonId::new(JESUS_PERSON_ID.to_string()).erase();
        ctx.graph.nodes.insert(
            id.clone(),
            Node { id, payload: NodePayload::Person { label: "Jesus".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None }, provenance: "test".into() },
        );
    }

    fn ctx_with<'a>(canon: &'a Canon, verses: &'a HashMap<String, String>, atlas: &'a AtlasData, corpus: &'a RedLetterCorpus) -> BuildCtx<'a> {
        let mut ctx = BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas);
        ctx.red_letter = Some(corpus);
        insert_jesus_node(&mut ctx);
        ctx
    }

    #[test]
    fn absent_red_letter_bundle_is_a_true_no_op() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        assert!(ctx.red_letter.is_none());
        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 0);
        assert!(ctx.graph.spoken_by.is_empty());
        assert!(ctx.graph.spoken_at.is_empty());
    }

    /// The defensive guard's own proof (`normalize`'s own doc comment):
    /// real red-letter data present, but NO Jesus Person node in the
    /// graph -- zero rows, never a dangling reference. Deliberately does
    /// NOT use `ctx_with` (which inserts the node) -- this is the one test
    /// that wants it ABSENT.
    #[test]
    fn no_jesus_person_node_means_no_rows_at_all_never_a_dangling_reference() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let corpus = corpus_with(vec![rv(5, 3, 10), rv(5, 4, 10)]);
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        ctx.red_letter = Some(&corpus);
        assert!(ctx.graph.nodes.is_empty(), "sanity: no Jesus node (or any node) exists in this fixture");

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 0);
        assert_eq!(stats.spoken_at_rows, 0);
        assert!(ctx.graph.spoken_by.is_empty());
        assert!(ctx.graph.spoken_at.is_empty());
    }

    #[test]
    fn normalize_builds_one_spoken_by_row_per_maximal_contiguous_range() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        // Two contiguous verses (5:3, 5:4) then a gap to 5:10 -- must
        // produce exactly TWO SpokenBy rows, not three individual ones.
        let corpus = corpus_with(vec![rv(5, 3, 10), rv(5, 4, 10), rv(5, 10, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 2, "ranges keep the table honest to discourse shape -- not one row per verse");
        assert_eq!(ctx.graph.spoken_by.len(), 2);

        let first = &ctx.graph.spoken_by[0];
        assert_eq!(first.locus.from.unit, VerseRef { book: 39, chapter: 5, verse: 3 });
        assert_eq!(first.locus.to.unit, VerseRef { book: 39, chapter: 5, verse: 4 });
        assert_eq!(first.speaker.0, JESUS_PERSON_ID);
        assert_eq!(first.provenance, SPOKEN_BY_PROVENANCE);
        assert!(first.justification.grounds.contains(&Ground::Scripture(first.locus.clone())), "SpokenBy's own justification grounds in its own locus (decision 3)");

        assert!(ctx.graph.spoken_at.is_empty(), "no Event/LocatedAt data in this fixture -- zero SpokenAt rows, honestly, never fabricated");
    }

    fn atlas_with_located_event(event_id: &str, place_id: &str, verses: &[&str]) -> AtlasData {
        let places = vec![Place { id: place_id.into(), name: "Test Place".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![Event {
            id: event_id.into(),
            label: "Test Event".into(),
            when: atlas_core::time::TimeRange::new(-5, 33).unwrap(),
            places: vec![place_id.into()],
            verses: verses.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }];
        AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish()
    }

    #[test]
    fn spoken_at_derives_a_place_when_the_range_falls_inside_a_located_events_attested_range() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = atlas_with_located_event("sermon-event", "mountain", &["MAT.5.1", "MAT.5.2", "MAT.5.3", "MAT.5.4", "MAT.5.5"]);
        let corpus = corpus_with(vec![rv(5, 3, 10), rv(5, 4, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);
        crate::event_world::normalize(&mut ctx); // builds attests/located_at BEFORE this adapter runs, matching real pipeline order

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 1);
        assert_eq!(stats.spoken_at_rows, 1);
        assert_eq!(stats.spoken_at_ranges_covered, 1);
        assert_eq!(stats.spoken_at_ranges_total, 1);

        let row = &ctx.graph.spoken_at[0];
        assert_eq!(row.place.0, "mountain");
        assert!(row.justification.text.as_deref().unwrap_or("").contains("sermon-event"));
    }

    #[test]
    fn spoken_at_is_honestly_empty_when_no_located_event_contains_the_range() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        // The event attests only 4:1-4:2 -- the red range (4:19) falls
        // OUTSIDE it, so no SpokenAt row must be produced (never guessed).
        let atlas = atlas_with_located_event("unrelated-event", "somewhere", &["MAT.4.1", "MAT.4.2"]);
        let corpus = corpus_with(vec![rv(4, 19, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);
        crate::event_world::normalize(&mut ctx);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 1);
        assert_eq!(stats.spoken_at_rows, 0);
        assert_eq!(stats.spoken_at_ranges_covered, 0);
        assert_eq!(stats.spoken_at_ranges_total, 1, "the range still counts toward the coverage denominator, honestly, even with zero hits");
    }

    #[test]
    fn spoken_at_dedupes_two_events_that_resolve_to_the_same_place_into_one_row() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let places = vec![Place { id: "mountain".into(), name: "Test Place".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![
            Event { id: "wide-event".into(), label: "Wide".into(), when: atlas_core::time::TimeRange::new(-5, 33).unwrap(), places: vec!["mountain".into()], verses: vec!["MAT.5.1".into(), "MAT.5.2".into(), "MAT.5.3".into(), "MAT.5.4".into(), "MAT.5.5".into()], ..Default::default() },
            Event { id: "narrow-event".into(), label: "Narrow".into(), when: atlas_core::time::TimeRange::new(-5, 33).unwrap(), places: vec!["mountain".into()], verses: vec!["MAT.5.3".into(), "MAT.5.4".into()], ..Default::default() },
        ];
        let atlas = AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let corpus = corpus_with(vec![rv(5, 3, 10), rv(5, 4, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);
        crate::event_world::normalize(&mut ctx);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 1);
        assert_eq!(stats.spoken_at_rows, 1, "both events resolve to the SAME place -- one row, not two identical ones (decision 3)");
    }

    #[test]
    fn spoken_by_and_spoken_at_rows_lower_into_the_directed_index_both_ways() {
        use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
        use atlas_graph_types::explore::{EdgeQuery, Explorable, PositionRef};
        use atlas_graph_types::id::{NodeKind as NK, Position};
        use atlas_graph_types::node::{Node, NodePayload};

        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = atlas_with_located_event("sermon-event", "mountain", &["MAT.5.1", "MAT.5.2", "MAT.5.3", "MAT.5.4"]);
        let corpus = corpus_with(vec![rv(5, 3, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);
        crate::event_world::normalize(&mut ctx);
        normalize(&mut ctx);

        // Real nodes for the TextUnit/Person/Place endpoints this pass
        // needs, matching graph.rs's own comments_on/located_at index
        // tests' shape (a minimal, hand-built node table, not a full
        // pipeline run).
        let verse_id = atlas_graph_types::id::AnyNodeId { kind: NK::TextUnit, raw: "bible/39.5.3".into() };
        ctx.graph.nodes.insert(
            verse_id.clone(),
            Node { id: verse_id.clone(), payload: NodePayload::TextUnit { corpus: "bible", renderings: Default::default() }, provenance: "test".into() },
        );
        let jesus_id = PersonId::new(JESUS_PERSON_ID.to_string()).erase();
        ctx.graph.nodes.insert(
            jesus_id.clone(),
            Node { id: jesus_id.clone(), payload: NodePayload::Person { label: "Jesus".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None }, provenance: "test".into() },
        );
        let place_id = atlas_graph_types::id::PlaceId::new("mountain").erase();
        ctx.graph.nodes.insert(place_id.clone(), Node { id: place_id.clone(), payload: NodePayload::Place { canonical: "Mountain".into(), lat: 0.0, lon: 0.0, aliases: vec![], description: None }, provenance: "test".into() });

        ctx.graph.build_indexes();

        let forward_by = EdgeKind::Directed(RelationId::SpokenBy, Direction::Forward);
        let page = PositionRef(Position::Node(verse_id.clone())).edges(&ctx.graph, &EdgeQuery { kind: forward_by, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1, "the verse's own forward 'spoken-by' frontier reaches Jesus");
        assert_eq!(page.entries[0].node, Position::Node(jesus_id.clone()));

        let inverse_by = EdgeKind::Directed(RelationId::SpokenBy, Direction::Inverse);
        let back = PositionRef(Position::Node(jesus_id)).edges(&ctx.graph, &EdgeQuery { kind: inverse_by, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1, "Jesus's own inverse 'speech-of' frontier lists the verse back");
        assert_eq!(back.entries[0].edge, page.entries[0].edge, "the SAME edge id, from either end -- the bijection witness");

        let forward_at = EdgeKind::Directed(RelationId::SpokenAt, Direction::Forward);
        let at_page = PositionRef(Position::Node(verse_id)).edges(&ctx.graph, &EdgeQuery { kind: forward_at, cursor: None, limit: 10 });
        assert_eq!(at_page.entries.len(), 1, "the verse's own forward 'spoken-at' frontier reaches the mountain");
        assert_eq!(at_page.entries[0].node, Position::Node(place_id));
    }
}
