
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use atlas_core::data::EventKind;
use atlas_graph_types::chrono::ResolvedPlacement;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::NodeKind;
use atlas_graph_types::node::{EventWitnessPayload, NodePayload};

use crate::kjv_adapter::KJV_TRANSLATION;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The pericope heading that belongs above one verse: the container it names, its title, that container's kind, and whether this verse merely continues coverage that began in an earlier chapter rather than opening it.")]
pub struct Heading {
    pub event_id: String,
    pub title: String,
    pub kind: EventKind,
    pub is_continuation: bool,
}

type Precedence = (u8, u8, Reverse<i32>, Reverse<i32>, Reverse<String>);

fn canonically_first(verses: &[String]) -> Option<String> {
    verses
        .iter()
        .filter_map(|v| atlas_core::refs::VerseId::parse_canonical(v).ok().map(|vid| ((vid.book.0, vid.chapter, vid.verse), v)))
        .min_by_key(|(key, _)| *key)
        .map(|(_, v)| v.clone())
}

fn heading_anchors_for(verses: &[String], witnesses: &[EventWitnessPayload]) -> Vec<String> {
    if !witnesses.is_empty() {
        return witnesses
            .iter()
            .filter_map(|w| w.translations.get(KJV_TRANSLATION))
            .filter_map(|vs| canonically_first(vs))
            .collect();
    }

    // Two passes -- group by book, then take each book's own minimum -- rather than one. The
    // first-seen book order the returned Vec keeps is harmless: each book's anchor lands at a
    // different verse, so only intra-book anchor CHOICE can ever decide a collision.
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
                continue;
            }
            if group.iter().any(|v| v.chapter == chapter && v.verse == 1) {
                out.push(format!("{}.{}.1", book.code(), chapter));
            }
        }
    }
    out
}

fn precedence(layer: u8, kind: EventKind, year: i32, seq: i32, event_id: &str) -> Precedence {
    let kind_bit: u8 = if kind == EventKind::Event { 1 } else { 0 };
    (layer, kind_bit, Reverse(year), Reverse(seq), Reverse(event_id.to_string()))
}

const UNDATED_SENTINEL: (i32, i32) = (-4004, 0);

fn resolved_year_seq(id: &str, resolved: &HashMap<String, ResolvedPlacement>) -> (i32, i32) {
    match resolved.get(id) {
        Some(rp) => (rp.date.from.year.get(), rp.seq.0 as i32),
        None => UNDATED_SENTINEL,
    }
}

pub fn narrative_leg_event_ids(graph: &Graph) -> BTreeSet<String> {
    graph.succession.iter().flat_map(|row| row.chain.iter().map(|e| e.0.clone())).collect()
}

pub fn build_heading_index(graph: &Graph, resolved: &HashMap<String, ResolvedPlacement>) -> BTreeMap<String, Heading> {
    let narrative_legs = narrative_leg_event_ids(graph);
    let mut winners: BTreeMap<String, (Precedence, Heading)> = BTreeMap::new();
    let mut continuation_candidates: Vec<(String, Precedence, Heading)> = Vec::new();

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

        let kind = crate::legacy::event_kind(kind);
        let layer: u8 = if is_real_container { 1 } else { 0 };
        let (year, seq) = resolved_year_seq(&id.raw, resolved);
        let prec = precedence(layer, kind, year, seq, &id.raw);

        for anchor in heading_anchors_for(verses, witnesses) {
            let should_replace = match winners.get(&anchor) {
                None => true,
                Some((incumbent, _)) => prec > *incumbent,
            };
            if should_replace {
                let entry = Heading { event_id: id.raw.clone(), title: label.clone(), kind, is_continuation: false };
                winners.insert(anchor, (prec.clone(), entry));
            }
        }

        for cont in continuation_candidates_for(verses, witnesses) {
            let entry = Heading { event_id: id.raw.clone(), title: label.clone(), kind, is_continuation: true };
            continuation_candidates.push((cont, prec.clone(), entry));
        }
    }

    // Competing CONTINUATION candidates at one still-open verse -- two containers both continuing
    // through it -- resolve by the same precedence tuple the primary pass uses.
    for (verse, prec, entry) in continuation_candidates {
        if continuation_displaces(winners.get(&verse), &prec) {
            winners.insert(verse, (prec, entry));
        }
    }

    winners.into_iter().map(|(verse, (_, entry))| (verse, entry)).collect()
}

fn continuation_displaces(incumbent: Option<&(Precedence, Heading)>, candidate: &Precedence) -> bool {
    match incumbent {
        None => true,
        Some((_, existing)) if !existing.is_continuation => false,
        Some((placed, _)) => candidate > placed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THE_SAME_LAYER: u8 = 1;

    fn heading(event_id: &str, is_continuation: bool) -> Heading {
        Heading { event_id: event_id.into(), title: event_id.into(), kind: EventKind::Event, is_continuation }
    }

    #[test]
    fn a_continuation_never_displaces_a_heading_that_opens_at_the_verse() {
        // Arrange
        let opener = (precedence(0, EventKind::General, 3000, 9999, "a_weak_opener"), heading("a_weak_opener", false));
        let continuing = precedence(THE_SAME_LAYER, EventKind::Event, -4004, 0, "z_strong_continuation");

        // Act
        let displaces = continuation_displaces(Some(&opener), &continuing);

        // Assert
        assert!(!displaces);
    }

    #[test]
    fn between_two_continuations_at_one_verse_the_stronger_precedence_wins() {
        // Arrange
        let weaker = (precedence(0, EventKind::General, 3000, 9999, "a_weak_continuation"), heading("a_weak_continuation", true));
        let stronger = precedence(THE_SAME_LAYER, EventKind::Event, -4004, 0, "z_strong_continuation");

        // Act
        let displaces = (continuation_displaces(Some(&weaker), &stronger), continuation_displaces(Some(&weaker), &weaker.0));

        // Assert
        assert_eq!(displaces, (true, false));
    }

    #[test]
    fn the_kind_tier_puts_a_dated_event_ahead_of_a_general_passage_whatever_their_chronology() {
        // Arrange
        let general = precedence(THE_SAME_LAYER, EventKind::General, 3000, 9999, "a_general_passage");
        let event = precedence(THE_SAME_LAYER, EventKind::Event, -4004, 0, "z_dated_event");

        // Act
        let winner = std::cmp::max(general, event.clone());

        // Assert
        assert_eq!(winner, event);
    }
}
