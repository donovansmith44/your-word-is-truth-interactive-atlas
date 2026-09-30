use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

use atlas_core::data::AtlasData;
use atlas_core::history::resolve_display_name;
use atlas_graph_types::edge::{MentionedEntity, Mentions};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{PersonId, PlaceId};
use atlas_graph_types::text::{TextLocus, TextRef, TokenSpan, VerseRef};

use crate::kjv_adapter::{self, KJV_TRANSLATION};
use crate::pipeline::BuildCtx;
use crate::tokens::{self, tokenize, Token};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub entity: MentionedEntity,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameSegment {
    pub chars: Range<usize>,
    pub entity: MentionedEntity,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MentionSpanStats {
    pub located: usize,
    pub unlocatable: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MentionSpan {
    pub entity: MentionedEntity,
    pub words: TokenSpan,
}

impl MentionSpan {
    pub fn of(row: &Mentions) -> Option<MentionSpan> {
        row.locus.span.clone().map(|words| MentionSpan { entity: row.entity.clone(), words })
    }
}

pub fn locate_mentions(ctx: &mut BuildCtx) -> MentionSpanStats {
    let names = NameBook::of(ctx.atlas);
    let spans = located_spans(&ctx.graph, &names);
    let mut stats = MentionSpanStats::default();
    let rows = std::mem::take(&mut ctx.graph.mentions);
    for (row, searched) in rows.into_iter().zip(spans) {
        match searched {
            Some(Search { found, missed }) if !found.is_empty() => {
                stats.located += found.len();
                stats.unlocatable += missed;
                ctx.graph.mentions.extend(found.into_iter().map(|span| Mentions { locus: TextLocus { at: row.locus.at.clone(), span: Some(span) }, ..row.clone() }));
            }
            Some(_) => {
                stats.unlocatable += 1;
                ctx.graph.mentions.push(row);
            }
            None => ctx.graph.mentions.push(row),
        }
    }
    stats
}

pub fn scan(text: &str, names: &[Name]) -> Vec<NameSegment> {
    let chars: Vec<char> = text.chars().collect();
    let (places, others): (Vec<&Name>, Vec<&Name>) = names.iter().partition(|n| matches!(n.entity, MentionedEntity::Place(_)));
    let mut candidates: Vec<NameSegment> = places.into_iter().chain(others).flat_map(|name| occurrences(&chars, name)).collect();
    candidates.sort_by_key(|c| (c.chars.start, Reverse(c.chars.len())));
    let mut accepted: Vec<NameSegment> = Vec::new();
    for candidate in candidates {
        if !accepted.iter().any(|a| candidate.chars.start < a.chars.end && a.chars.start < candidate.chars.end) {
            accepted.push(candidate);
        }
    }
    accepted
}

pub fn locate(segment: &NameSegment, words: &[Token]) -> Option<TokenSpan> {
    tokens::words_covering(&segment.chars, words, KJV_TRANSLATION)
}

fn occurrences(chars: &[char], name: &Name) -> Vec<NameSegment> {
    let wanted: Vec<char> = name.text.chars().collect();
    if wanted.is_empty() {
        return Vec::new();
    }
    let is_letter_at = |i: usize| chars.get(i).is_some_and(|c| c.is_alphabetic());
    chars
        .windows(wanted.len())
        .enumerate()
        .filter(|&(start, window)| window == wanted.as_slice() && (start == 0 || !is_letter_at(start - 1)) && !is_letter_at(start + wanted.len()))
        .map(|(start, _)| NameSegment { chars: start..start + wanted.len(), entity: name.entity.clone() })
        .collect()
}

#[derive(Clone)]
struct Search {
    found: Vec<TokenSpan>,
    missed: usize,
}

fn located_spans(graph: &Graph, names: &NameBook) -> Vec<Option<Search>> {
    let mut rows_by_verse: BTreeMap<&VerseRef, Vec<usize>> = BTreeMap::new();
    for (i, row) in graph.mentions.iter().enumerate() {
        if let (TextRef::Bible(verse), None, MentionedEntity::Place(_) | MentionedEntity::Person(_)) = (&row.locus.at, &row.locus.span, &row.entity) {
            rows_by_verse.entry(verse).or_default().push(i);
        }
    }
    let mut spans = vec![None; graph.mentions.len()];
    for (verse, rows) in rows_by_verse {
        let text = graph.nodes.get(&kjv_adapter::verse_node_id(verse.book, verse.chapter, verse.verse)).and_then(kjv_adapter::kjv_text).unwrap_or_default();
        let tokens = tokenize(text);
        let searched: Vec<Name> = rows
            .iter()
            .flat_map(|&i| {
                let entity = &graph.mentions[i].entity;
                names.of_entity(entity).iter().map(|text| Name { entity: entity.clone(), text: text.clone() })
            })
            .collect();
        let occurrences: Vec<(MentionedEntity, Option<TokenSpan>)> = scan(text, &searched).into_iter().map(|s| (s.entity.clone(), locate(&s, &tokens))).collect();
        for i in rows {
            let entity = &graph.mentions[i].entity;
            let of_entity = occurrences.iter().filter(|(e, _)| e == entity);
            spans[i] = Some(Search { found: of_entity.clone().filter_map(|(_, span)| span.clone()).collect(), missed: of_entity.filter(|(_, span)| span.is_none()).count() });
        }
    }
    spans
}

struct NameBook {
    places: HashMap<PlaceId, Vec<String>>,
    persons: HashMap<PersonId, Vec<String>>,
}

impl NameBook {
    fn of(atlas: &AtlasData) -> NameBook {
        let places = atlas
            .places
            .iter()
            .map(|p| {
                let display = resolve_display_name(&p.name, atlas.place_history_for(&p.id), None, atlas.place_name_alias_for(&p.id));
                (PlaceId::new(p.id.clone()), std::iter::once(display).chain(crate::event_world::kjv_aliases_of(atlas, &p.id)).collect())
            })
            .collect();
        let persons = atlas.people.iter().map(|p| (PersonId::new(p.id.clone()), std::iter::once(p.name.clone()).chain(p.also_called.iter().cloned()).collect())).collect();
        NameBook { places, persons }
    }

    fn of_entity(&self, entity: &MentionedEntity) -> &[String] {
        let found = match entity {
            MentionedEntity::Place(p) => self.places.get(p),
            MentionedEntity::Person(p) => self.persons.get(p),
            MentionedEntity::PeopleGroup(_) | MentionedEntity::Event(_) => None,
        };
        found.map(Vec::as_slice).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use atlas_core::data::{AtlasData, Canon, Person, Place, PlaceNameAlias};
    use atlas_graph_types::edge::{MentionedEntity, Mentions};
    use atlas_graph_types::id::{EventId, PeopleGroupId};
    use atlas_graph_types::text::{BibleLocus, TextLocus, VerseRef};

    use super::*;
    use crate::kjv_adapter::{self, KjvVerse, KJV_TRANSLATION};

    const PROVENANCE: &str = "test";
    const GEN_13_18: VerseRef = VerseRef { book: 0, chapter: 13, verse: 18 };

    #[test]
    fn a_place_named_twice_in_a_verse_is_mentioned_once_per_occurrence() {
        // Arrange
        let atlas = atlas(vec![place("hebron", "Hebron")], vec![], vec![]);
        let rows = vec![verse_level(MentionedEntity::Place(PlaceId::new("hebron")))];
        // Act
        let located = locate_in("And Abram came to Hebron, and dwelt in Hebron.", &atlas, rows);
        // Assert
        assert_eq!(
            located,
            (
                vec![
                    at_words(MentionedEntity::Place(PlaceId::new("hebron")), 4, 4),
                    at_words(MentionedEntity::Place(PlaceId::new("hebron")), 8, 8),
                ],
                MentionSpanStats { located: 2, unlocatable: 0 }
            )
        );
    }

    #[test]
    fn a_person_the_verse_does_not_name_stays_verse_level_and_is_counted() {
        // Arrange
        let atlas = atlas(vec![], vec![person("abraham", "Abraham", &[])], vec![]);
        let rows = vec![verse_level(MentionedEntity::Person(PersonId::new("abraham")))];
        // Act
        let located = locate_in("And he removed his tent.", &atlas, rows);
        // Assert
        assert_eq!(located, (vec![verse_level(MentionedEntity::Person(PersonId::new("abraham")))], MentionSpanStats { located: 0, unlocatable: 1 }));
    }

    #[test]
    fn a_name_that_begins_a_joined_word_is_unlocatable() {
        // Arrange
        let atlas = atlas(vec![], vec![person("tubal", "Tubal", &[])], vec![]);
        let rows = vec![verse_level(MentionedEntity::Person(PersonId::new("tubal")))];
        // Act
        let located = locate_in("And Zillah, she also bare Tubal–cain.", &atlas, rows);
        // Assert
        assert_eq!(located, (vec![verse_level(MentionedEntity::Person(PersonId::new("tubal")))], MentionSpanStats { located: 0, unlocatable: 1 }));
    }

    #[test]
    fn a_name_that_ends_a_joined_word_is_unlocatable() {
        // Arrange
        let atlas = atlas(vec![place("sheba", "Sheba")], vec![], vec![]);
        let rows = vec![verse_level(MentionedEntity::Place(PlaceId::new("sheba")))];
        // Act
        let located = locate_in("And they came to Beer–Sheba.", &atlas, rows);
        // Assert
        assert_eq!(located, (vec![verse_level(MentionedEntity::Place(PlaceId::new("sheba")))], MentionSpanStats { located: 0, unlocatable: 1 }));
    }

    #[test]
    fn an_occurrence_off_whole_words_beside_a_located_one_is_counted_unlocatable() {
        // Arrange
        let atlas = atlas(vec![place("sheba", "Sheba")], vec![], vec![]);
        let rows = vec![verse_level(MentionedEntity::Place(PlaceId::new("sheba")))];
        // Act
        let located = locate_in("And they came to Sheba and to Beer–Sheba.", &atlas, rows);
        // Assert
        assert_eq!(located, (vec![at_words(MentionedEntity::Place(PlaceId::new("sheba")), 4, 4)], MentionSpanStats { located: 1, unlocatable: 1 }));
    }

    #[test]
    fn a_name_of_two_words_spans_both() {
        // Arrange
        let atlas = atlas(vec![], vec![person("simon_peter", "Simon Peter", &[])], vec![]);
        let rows = vec![verse_level(MentionedEntity::Person(PersonId::new("simon_peter")))];
        // Act
        let located = locate_in("And Simon Peter answered and said.", &atlas, rows);
        // Assert
        assert_eq!(located, (vec![at_words(MentionedEntity::Person(PersonId::new("simon_peter")), 1, 2)], MentionSpanStats { located: 1, unlocatable: 0 }));
    }

    #[test]
    fn a_place_is_searched_by_its_name_without_the_number_that_tells_it_apart() {
        // Arrange
        let atlas = atlas(vec![place("hebron-2", "Hebron 2")], vec![], vec![]);
        let rows = vec![verse_level(MentionedEntity::Place(PlaceId::new("hebron-2")))];
        // Act
        let located = locate_in("And they went up to Hebron.", &atlas, rows);
        // Assert
        assert_eq!(located, (vec![at_words(MentionedEntity::Place(PlaceId::new("hebron-2")), 5, 5)], MentionSpanStats { located: 1, unlocatable: 0 }));
    }

    #[test]
    fn a_place_is_searched_by_every_name_the_kjv_gives_it() {
        // Arrange
        let atlas = atlas(
            vec![place("kiriath-arba", "Kiriath-arba")],
            vec![],
            vec![alias("kiriath-arba", "Kirjath–arba"), alias("kiriath-arba", "Kirjath–arba’s city")],
        );
        let rows = vec![verse_level(MentionedEntity::Place(PlaceId::new("kiriath-arba")))];
        // Act
        let located = locate_in("And Sarah died in Kirjath–arba; and in Kirjath–arba’s city.", &atlas, rows);
        // Assert
        assert_eq!(
            located,
            (
                vec![
                    at_words(MentionedEntity::Place(PlaceId::new("kiriath-arba")), 4, 4),
                    at_words(MentionedEntity::Place(PlaceId::new("kiriath-arba")), 7, 8),
                ],
                MentionSpanStats { located: 2, unlocatable: 0 }
            )
        );
    }

    #[test]
    fn a_person_is_searched_by_every_name_they_are_also_called() {
        // Arrange
        let atlas = atlas(vec![], vec![person("abraham", "Abraham", &["Abram"])], vec![]);
        let rows = vec![verse_level(MentionedEntity::Person(PersonId::new("abraham")))];
        // Act
        let located = locate_in("And Abram went up out of Egypt.", &atlas, rows);
        // Assert
        assert_eq!(located, (vec![at_words(MentionedEntity::Person(PersonId::new("abraham")), 1, 1)], MentionSpanStats { located: 1, unlocatable: 0 }));
    }

    #[test]
    fn a_name_both_a_place_and_a_person_bear_goes_to_the_place() {
        // Arrange
        let atlas = atlas(vec![place("sheba", "Sheba")], vec![person("sheba_1", "Sheba", &[])], vec![]);
        let rows = vec![verse_level(MentionedEntity::Person(PersonId::new("sheba_1"))), verse_level(MentionedEntity::Place(PlaceId::new("sheba")))];
        // Act
        let located = locate_in("And the queen of Sheba came.", &atlas, rows);
        // Assert
        assert_eq!(
            located,
            (
                vec![verse_level(MentionedEntity::Person(PersonId::new("sheba_1"))), at_words(MentionedEntity::Place(PlaceId::new("sheba")), 4, 4)],
                MentionSpanStats { located: 1, unlocatable: 1 }
            )
        );
    }

    #[test]
    fn a_mention_at_a_verse_the_graph_does_not_hold_stays_verse_level_and_is_counted() {
        // Arrange
        let atlas = atlas(vec![place("hebron", "Hebron")], vec![], vec![]);
        let elsewhere = Mentions { locus: TextLocus::from(BibleLocus::whole(VerseRef { book: 0, chapter: 23, verse: 19 })), ..verse_level(MentionedEntity::Place(PlaceId::new("hebron"))) };
        // Act
        let located = locate_in("And Abram came to Hebron.", &atlas, vec![elsewhere.clone()]);
        // Assert
        assert_eq!(located, (vec![elsewhere], MentionSpanStats { located: 0, unlocatable: 1 }));
    }

    #[test]
    fn people_groups_and_events_stay_verse_level_and_are_not_counted() {
        // Arrange
        let atlas = atlas(vec![], vec![], vec![]);
        let rows = vec![verse_level(MentionedEntity::PeopleGroup(PeopleGroupId::new("canaanites"))), verse_level(MentionedEntity::Event(EventId::new("the-flood")))];
        // Act
        let located = locate_in("And the Canaanite was then in the land.", &atlas, rows.clone());
        // Assert
        assert_eq!(located, (rows, MentionSpanStats { located: 0, unlocatable: 0 }));
    }

    fn locate_in(verse: &str, atlas: &AtlasData, rows: Vec<Mentions>) -> (Vec<Mentions>, MentionSpanStats) {
        let (canon, verses) = (Canon { books: vec![] }, HashMap::new());
        let mut ctx = BuildCtx::new(&canon, &verses, None, "", atlas);
        let node = kjv_adapter::verse_node(&KjvVerse { book_index: GEN_13_18.book, chapter: GEN_13_18.chapter, verse: GEN_13_18.verse, text: verse.to_string() });
        ctx.graph.nodes.insert(node.id.clone(), node);
        ctx.graph.mentions = rows;
        let stats = locate_mentions(&mut ctx);
        (ctx.graph.mentions, stats)
    }

    fn atlas(places: Vec<Place>, people: Vec<Person>, aliases: Vec<PlaceNameAlias>) -> AtlasData {
        let mut atlas = AtlasData::new(Canon { books: vec![] }, places, vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        atlas.people = people;
        for a in aliases {
            atlas.place_name_aliases.entry(a.id.clone()).or_default().push(a);
        }
        atlas
    }

    fn place(id: &str, name: &str) -> Place {
        Place { id: id.into(), name: name.into(), lat: 0.0, lon: 0.0, verse_links: vec![] }
    }

    fn person(id: &str, name: &str, also_called: &[&str]) -> Person {
        Person { id: id.into(), name: name.into(), also_called: also_called.iter().map(|n| n.to_string()).collect(), ..Person::default() }
    }

    fn alias(id: &str, kjv: &str) -> PlaceNameAlias {
        PlaceNameAlias { id: id.into(), translations: HashMap::from([(KJV_TRANSLATION.to_string(), kjv.to_string())]), verses: vec![] }
    }

    fn verse_level(entity: MentionedEntity) -> Mentions {
        Mentions { locus: TextLocus::from(BibleLocus::whole(GEN_13_18)), entity, provenance: PROVENANCE.into() }
    }

    fn at_words(entity: MentionedEntity, first: u16, last: u16) -> Mentions {
        let span = tokens::span(KJV_TRANSLATION, first, last).expect("first <= last");
        Mentions { locus: TextLocus::from(BibleLocus { unit: GEN_13_18, span: Some(span) }), entity, provenance: PROVENANCE.into() }
    }
}
