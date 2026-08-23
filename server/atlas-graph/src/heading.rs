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
use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::NodeKind;
use atlas_graph_types::node::{EventWitnessPayload, NodePayload};

use crate::kjv_adapter::KJV_TRANSLATION;

/// Mirrors `atlas_core::data::HeadingEntry` exactly (event id + title +
/// kind) -- the resolved pericope heading anchored at one verse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadingEntry {
    pub event_id: String,
    pub title: String,
    pub kind: String,
}

type Precedence = (u8, u8, Reverse<i32>, Reverse<i32>, Reverse<String>);

/// Mirrors `atlas_core::data::heading_anchors_for` exactly: if explicit
/// witnesses exist, one anchor per witness (its own first parseable KJV
/// verse); else one anchor per book actually touched by the container's
/// own top-level `verses`, first-seen-book order.
fn heading_anchors_for(verses: &[String], witnesses: &[EventWitnessPayload]) -> Vec<String> {
    if !witnesses.is_empty() {
        return witnesses
            .iter()
            .filter_map(|w| w.translations.get(KJV_TRANSLATION))
            .filter_map(|vs| vs.iter().find(|v| atlas_core::refs::VerseId::parse_canonical(v).is_ok()))
            .cloned()
            .collect();
    }

    let mut seen_books: Vec<String> = Vec::new();
    let mut anchors: Vec<String> = Vec::new();
    for v in verses {
        let Ok(vid) = atlas_core::refs::VerseId::parse_canonical(v) else { continue };
        let book = vid.book.code().to_string();
        if !seen_books.contains(&book) {
            seen_books.push(book);
            anchors.push(v.clone());
        }
    }
    anchors
}

/// Mirrors `atlas_core::data::heading_precedence`'s own 3-tier rule (layer,
/// kind, chronology) plus the disclosed 5th determinism tier (event id --
/// this module's own doc comment).
fn precedence(layer: u8, kind: &str, from_year: i32, order_key: i32, event_id: &str) -> Precedence {
    let kind_bit: u8 = if kind == "event" { 1 } else { 0 };
    (layer, kind_bit, Reverse(from_year), Reverse(order_key), Reverse(event_id.to_string()))
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
/// one deliberate improvement). Order-independent over `graph.nodes`'s own
/// iteration (a `BTreeMap`, so already deterministic either way): the
/// winner at each anchor is the objective MAXIMUM precedence among every
/// heading-worthy event claiming it, never a first-wins accident of scan
/// order.
pub fn build_heading_index(graph: &Graph) -> BTreeMap<String, HeadingEntry> {
    let narrative_legs = narrative_leg_event_ids(graph);
    let mut winners: BTreeMap<String, (Precedence, HeadingEntry)> = BTreeMap::new();

    for (id, node) in &graph.nodes {
        if id.kind != NodeKind::Event {
            continue;
        }
        let NodePayload::Event {
            label,
            kind,
            from_year,
            order_key,
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
        let prec = precedence(layer, kind, *from_year, *order_key, &id.raw);

        for anchor in heading_anchors_for(verses, witnesses) {
            let should_replace = match winners.get(&anchor) {
                None => true,
                Some((incumbent, _)) => prec > *incumbent,
            };
            if should_replace {
                let entry = HeadingEntry { event_id: id.raw.clone(), title: label.clone(), kind: kind.clone() };
                winners.insert(anchor, (prec.clone(), entry));
            }
        }
    }

    winners.into_iter().map(|(verse, (_, entry))| (verse, entry)).collect()
}
