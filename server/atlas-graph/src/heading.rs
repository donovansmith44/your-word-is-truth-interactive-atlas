//! M-C2 requirement 1 / the decisive-title law, re-homed as a graph query.
//!
//! Re-implements `atlas_core::data::AtlasData::finish()`'s own
//! `heading_worthy` test, `heading_anchors_for`, and `heading_precedence`
//! (CONTRACT.md's own HEADING-WORTHY RULE + 3-tier precedence paragraph)
//! directly over graph `Event` nodes -- computed ONCE, at `GraphService::
//! assemble` time (an event-count-sized pass, ~1,700 events -- the same
//! "cheap, once, not per-request" class as `bible_position`/`era_ids`),
//! never per-request: `handlers::chapter`'s own per-verse heading lookup
//! (up to 176 calls per chapter, Psalm 119) needs O(1) access, the exact
//! reason the pre-M-C2 `AtlasData.verse_heading` index existed. Kept in
//! LOCKSTEP with CONTRACT.md and the atlas-core original (both describe the
//! identical rule; if either changes, so must this module, in the same
//! commit -- the atlas-core original itself stays untouched and still
//! green, still the law for `AtlasData`-sourced surfaces that have not yet
//! migrated, mirroring the M-B/M-C precedent of re-homing a law onto the
//! graph without deleting the atlas-core version until every consumer has).
//!
//! DETERMINISM IMPROVEMENT, disclosed: the atlas-core original's own
//! precedence tuple doc comment states a true 4-tier tie among the layer/
//! kind/chronology tiers "is not expected to ever actually occur for two
//! distinct real events" (CONTRACT.md's own WITHIN-LAYER ANCHOR COLLISIONS
//! validator fails ETL loud on the one case that WOULD produce one -- two
//! real containers claiming the identical anchor) and, on the rare
//! four-way tie, resolves by incidental iteration/vec order (first-wins).
//! This re-homing adds the event id itself as a fifth, always-distinct
//! tier, so the winner is a pure function of CONTENT for every input,
//! never of node-table iteration order (`Graph.nodes` is a `BTreeMap`,
//! already order-stable by id -- this tier makes that stability load-
//! bearing rather than incidental). Changes nothing for any real curated
//! case; only makes the theoretical tie-break provably deterministic.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use atlas_graph_types::chrono::ResolvedPlacement;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::NodeKind;
use atlas_graph_types::node::{EventWitnessPayload, NodePayload};

use crate::kjv_adapter::KJV_TRANSLATION;

/// Mirrors `atlas_core::data::HeadingEntry` (event id + title + kind) plus
/// ONE field it does not carry -- `continuation` (M-D1 requirement 1,
/// CHAPTER-BOUNDARY CONTINUATION, below): the resolved pericope heading
/// anchored at one verse, `continuation` true iff this verse is NOT the
/// container's own true first-covered verse, but a LATER chapter this same
/// container's coverage continues into. The atlas-core original has no
/// counterpart field (this module's own doc comment already discloses why
/// continuation headings are graph-side only: the atlas-core path is a
/// dead, tested reference oracle for the base anchor law, never a live
/// consumer this new law needs to reach).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadingEntry {
    pub event_id: String,
    pub title: String,
    pub kind: String,
    pub continuation: bool,
}

type Precedence = (u8, u8, Reverse<i32>, Reverse<i32>, Reverse<String>);

/// M-D1 requirement 1 (owner live report #2, GEN.6 class, verbatim:
/// "genesis 6 the first verses have no container label... i'm assuming
/// this isn't an isolated case"): the CANONICALLY FIRST verse in `verses`
/// -- minimum by (book, chapter, verse), i.e. reading-spine order -- never
/// merely the first one encountered in CURATED/IMPORTED array order.
/// `VerseId` carries no `Ord` of its own (a deliberate, narrow choice --
/// widening a shared type's derive list is a bigger blast radius than this
/// one law needs), so the tuple key is built by hand here, the same
/// "compare by content, not position" fix in both branches below.
fn canonically_first(verses: &[String]) -> Option<String> {
    verses
        .iter()
        .filter_map(|v| atlas_core::refs::VerseId::parse_canonical(v).ok().map(|vid| ((vid.book.0, vid.chapter, vid.verse), v)))
        .min_by_key(|(key, _)| *key)
        .map(|(_, v)| v.clone())
}

/// Mirrors `atlas_core::data::heading_anchors_for`'s own SHAPE (one anchor
/// per witness when explicit witnesses exist; else one anchor per book
/// touched by the container's own top-level `verses`) but fixes the
/// ROOT-CAUSE bug both branches shared: each anchor is now the book's own
/// CANONICALLY FIRST covered verse (`canonically_first` above), not merely
/// the first one this container's own curated/imported data happened to
/// list first. Concretely: `theo-32` ("God decides to destroy every living
/// thing") covers GEN.6.1-7, but its original Theographic verse link was
/// GEN.6.7 and W1's own enrichment pass APPENDED 6.1-6 afterward -- the OLD
/// code anchored at GEN.6.7 (first in the array), leaving GEN.6.1-6 read
/// unlabeled; this fix anchors at GEN.6.1 (first in reading order), the
/// SAME defect class the brief's own root-cause note names as systemic
/// ("any enriched event whose original link was not the passage's first
/// verse is mis-anchored"), fixed for BOTH branches (the witness branch had
/// the identical defect via `.find(first parseable)`).
fn heading_anchors_for(verses: &[String], witnesses: &[EventWitnessPayload]) -> Vec<String> {
    if !witnesses.is_empty() {
        return witnesses
            .iter()
            .filter_map(|w| w.translations.get(KJV_TRANSLATION))
            .filter_map(|vs| canonically_first(vs))
            .collect();
    }

    // No explicit witnesses -- one anchor per book actually touched by
    // `verses`: that book's own canonically first verse, not merely the
    // first one encountered in curated/imported array order. Two passes
    // (group by book, then take each book's own minimum) rather than one --
    // `seen_books` still preserves first-SEEN-book order for the returned
    // Vec's own iteration order (harmless: each book's anchor lands at a
    // DIFFERENT verse, so inter-book ordering here never decides a
    // collision -- only intra-book anchor CHOICE does, which is what this
    // fix corrects).
    let mut seen_books: Vec<String> = Vec::new();
    let mut best: std::collections::HashMap<String, (u16, u16, String)> = std::collections::HashMap::new();
    for v in verses {
        let Ok(vid) = atlas_core::refs::VerseId::parse_canonical(v) else { continue };
        let book = vid.book.code().to_string();
        match best.get(&book) {
            None => {
                seen_books.push(book.clone());
                best.insert(book, (vid.chapter, vid.verse, v.clone()));
            }
            Some((c, ve, _)) if (vid.chapter, vid.verse) < (*c, *ve) => {
                best.insert(book, (vid.chapter, vid.verse, v.clone()));
            }
            Some(_) => {}
        }
    }
    seen_books.into_iter().filter_map(|b| best.remove(&b).map(|(_, _, v)| v)).collect()
}

/// M-D1 requirement 1 ("CHAPTER-BOUNDARY CONTINUATION": "when a chapter's
/// first verse sits mid-container (container anchored in a previous
/// chapter), render the container's heading at the chapter top with a
/// quiet continuation marker... No covered chapter may open with unlabeled
/// verses"): for the SAME per-witness/per-book groups `heading_anchors_for`
/// anchors, every chapter STRICTLY AFTER that group's own anchor chapter
/// (the minimum chapter present, since a group's own primary anchor is
/// always its canonically-first verse) that the group's own coverage
/// includes AT the chapter's own opening verse (verse 1, always the
/// chapter boundary in KJV versification -- no canon lookup needed) is a
/// CONTINUATION candidate: this same container's heading continues there.
/// A gap chapter the group does NOT cover at its own verse 1 (coverage
/// resumes mid-chapter, or not at all) is correctly NOT a continuation
/// point for this container -- whichever OTHER heading-worthy container
/// truly opens that chapter (whole-Bible coverage, Batch W5, guarantees
/// one exists) supplies its own primary anchor or continuation there
/// instead; this function only ever answers for ITS OWN container.
fn continuation_candidates_for(verses: &[String], witnesses: &[EventWitnessPayload]) -> Vec<String> {
    let groups: Vec<Vec<atlas_core::refs::VerseId>> = if !witnesses.is_empty() {
        witnesses
            .iter()
            .filter_map(|w| w.translations.get(KJV_TRANSLATION))
            .map(|vs| vs.iter().filter_map(|v| atlas_core::refs::VerseId::parse_canonical(v).ok()).collect())
            .collect()
    } else {
        let mut order: Vec<u8> = Vec::new();
        let mut by_book: std::collections::HashMap<u8, Vec<atlas_core::refs::VerseId>> = std::collections::HashMap::new();
        for v in verses {
            if let Ok(vid) = atlas_core::refs::VerseId::parse_canonical(v) {
                if !by_book.contains_key(&vid.book.0) {
                    order.push(vid.book.0);
                }
                by_book.entry(vid.book.0).or_default().push(vid);
            }
        }
        order.into_iter().filter_map(|b| by_book.remove(&b)).collect()
    };

    let mut out = Vec::new();
    for group in groups {
        let Some(anchor_chapter) = group.iter().map(|v| v.chapter).min() else { continue };
        let book = group[0].book;
        let covered_chapters: BTreeSet<u16> = group.iter().map(|v| v.chapter).collect();
        for chapter in covered_chapters {
            if chapter <= anchor_chapter {
                continue; // the anchor's own chapter is a PRIMARY anchor, never a continuation
            }
            if group.iter().any(|v| v.chapter == chapter && v.verse == 1) {
                out.push(format!("{}.{}.1", book.code(), chapter));
            }
        }
    }
    out
}

/// Mirrors `atlas_core::data::heading_precedence`'s own 3-tier rule (layer,
/// kind, chronology) plus the disclosed 5th determinism tier (event id --
/// this module's own doc comment).
///
/// M-D3 (owner ruling R1 propagation): `year`/`seq` replace the former
/// `from_year`/`order_key` payload reads -- sourced from the event's own
/// `ResolvedPlacement` (`event_world::ChronologyDerivation.resolved`, the
/// timeline's one authority) instead, via `build_heading_index`'s own new
/// parameter. `seq` (the event's own position in the reconstructed global
/// timeline, `event_world::derive_chronology`'s own doc comment: "assigned
/// DIRECTLY from the reconstructed timeline position... exact, not a
/// shortcut") is a faithful, STRICTER substitute for `order_key` here: the
/// timeline itself is already sorted by `(from_year, order_key, array
/// position)`, so comparing `seq` alone reproduces every `(from_year,
/// order_key)` ordering the old tuple decided, and never ties between two
/// distinct dated events (order_key defaulted to 0 for the overwhelming
/// majority, so the old tuple tied far more often, falling through to the
/// event-id tier below more frequently than this one now needs to).
fn precedence(layer: u8, kind: &str, year: i32, seq: i32, event_id: &str) -> Precedence {
    let kind_bit: u8 = if kind == "event" { 1 } else { 0 };
    (layer, kind_bit, Reverse(year), Reverse(seq), Reverse(event_id.to_string()))
}

/// Sentinel `(year, seq)` for a heading-worthy node with NO resolved
/// placement -- exclusively general-kind ("undated") events, which
/// `event_world::derive_chronology` correctly excludes from `resolved`
/// entirely (never a real dated event: `derive_chronology` resolves EVERY
/// `kind == "event"` id `timeline_order` collects). Mirrors
/// `atlas_core::time::TimeRange::undated()`'s own whole-atlas-span sentinel
/// (`-4004`) rather than inventing a new one -- the SAME "deliberately
/// undated, not a mystery number" idiom every other general-kind call site
/// in this workspace already uses. The exact value is functionally inert
/// here: `kind_bit` (0 for every general-kind node) already outranks the
/// chronology tier ahead of it, so this sentinel only ever breaks a tie
/// between two OTHER general-kind nodes, which then falls through to the
/// always-distinct event-id tier regardless.
const UNDATED_SENTINEL: (i32, i32) = (-4004, 0);

fn resolved_year_seq(id: &str, resolved: &HashMap<String, ResolvedPlacement>) -> (i32, i32) {
    match resolved.get(id) {
        Some(rp) => (rp.date.from.year.get(), rp.seq.0 as i32),
        None => UNDATED_SENTINEL,
    }
}

/// `event_id -> true` iff it is a leg of ANY narrative -- callers build
/// this from `graph.succession`'s own row `chain`s (not from `follows-in`/
/// `precedes-in` EDGES: a solo-leg narrative, a real if rare shape, e.g.
/// `demo_fixture()`'s own `patriarchs-demo`, produces a `Succession` ROW
/// but zero succession EDGE pairs by construction -- `chain.windows(2)` on
/// a one-element chain is empty -- the same gap `handlers::
/// narrative_event_positions`'s own solo-leg fallback already discloses).
pub fn narrative_leg_event_ids(graph: &Graph) -> BTreeSet<String> {
    graph.succession.iter().flat_map(|row| row.chain.iter().map(|e| e.0.clone())).collect()
}

/// The full verse -> heading map, exactly mirroring `AtlasData::finish()`'s
/// own `verse_heading` construction (this module's own doc comment has the
/// one deliberate improvement) PLUS M-D1 requirement 1's own continuation
/// pass (below). Order-independent over `graph.nodes`'s own iteration (a
/// `BTreeMap`, so already deterministic either way): the winner at each
/// anchor is the objective MAXIMUM precedence among every heading-worthy
/// event claiming it, never a first-wins accident of scan order.
///
/// TWO PASSES, deliberately sequential, never interleaved: pass 1 resolves
/// every PRIMARY anchor exactly as before this batch (unchanged collision
/// rule); pass 2 resolves CONTINUATION candidates only at verses pass 1
/// left unclaimed -- a primary anchor is always the stronger claim (a
/// container's own true first-covered verse beats any other container's
/// mid-coverage continuation, unconditionally, never arbitrated by the
/// precedence tuple), so continuations can only ever FILL gaps, never
/// contest an already-decided verse. This is what keeps "decisive rule
/// still yields exactly one label" true even with continuations in play.
pub fn build_heading_index(graph: &Graph, resolved: &HashMap<String, ResolvedPlacement>) -> BTreeMap<String, HeadingEntry> {
    let narrative_legs = narrative_leg_event_ids(graph);
    let mut winners: BTreeMap<String, (Precedence, HeadingEntry)> = BTreeMap::new();
    let mut continuation_candidates: Vec<(String, Precedence, HeadingEntry)> = Vec::new();

    for (id, node) in &graph.nodes {
        if id.kind != NodeKind::Event {
            continue;
        }
        let NodePayload::Event {
            label,
            kind,
            verses,
            witnesses,
            robertson_section,
            acts_section,
            atlas_section,
            kjv_superscription,
            ..
        } = &node.payload
        else {
            continue;
        };

        let is_real_container =
            !witnesses.is_empty() || robertson_section.is_some() || acts_section.is_some() || atlas_section.is_some() || kjv_superscription.is_some();
        let heading_worthy = narrative_legs.contains(&id.raw) || is_real_container;
        if !heading_worthy {
            continue;
        }

        let layer: u8 = if is_real_container { 1 } else { 0 };
        let (year, seq) = resolved_year_seq(&id.raw, resolved);
        let prec = precedence(layer, kind, year, seq, &id.raw);

        for anchor in heading_anchors_for(verses, witnesses) {
            let should_replace = match winners.get(&anchor) {
                None => true,
                Some((incumbent, _)) => prec > *incumbent,
            };
            if should_replace {
                let entry = HeadingEntry { event_id: id.raw.clone(), title: label.clone(), kind: kind.clone(), continuation: false };
                winners.insert(anchor, (prec.clone(), entry));
            }
        }

        for cont in continuation_candidates_for(verses, witnesses) {
            let entry = HeadingEntry { event_id: id.raw.clone(), title: label.clone(), kind: kind.clone(), continuation: true };
            continuation_candidates.push((cont, prec.clone(), entry));
        }
    }

    // Pass 2: fill gaps only. A PRIMARY anchor already at this verse is an
    // ABSOLUTE, unconditional win (never arbitrated by precedence -- a
    // container's own true first-covered verse always beats any other
    // container's mid-coverage continuation); competing CONTINUATION
    // candidates at the same still-open verse (two different containers
    // both continuing through it) resolve by the identical precedence
    // tuple pass 1 uses -- the same layer/kind/chronology/id law, just
    // applied to the smaller "who continues here" question.
    for (verse, prec, entry) in continuation_candidates {
        let should_replace = match winners.get(&verse) {
            None => true,
            Some((_, existing)) if !existing.continuation => false,
            Some((incumbent, _)) => prec > *incumbent,
        };
        if should_replace {
            winners.insert(verse, (prec, entry));
        }
    }

    winners.into_iter().map(|(verse, (_, entry))| (verse, entry)).collect()
}
