//! Fills `description` on every Place/Person/PeopleGroup node from Easton's, in trust order: the
//! person's own source record, then the attested single-entity match, then an exact case-insensitive
//! name match. Never fabricated -- a filled description is one whole source string, or `None`.

use std::collections::{HashMap, HashSet};

use atlas_core::data::EastonEntry;
use atlas_graph_types::node::NodePayload;

use crate::pipeline::BuildCtx;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DescriptionStats {
    pub person_total: usize,
    pub person_tier_a: usize,
    pub person_tier_b: usize,
    pub person_tier_c: usize,
    pub place_total: usize,
    pub place_tier_b: usize,
    pub place_tier_c: usize,
    pub people_group_total: usize,
    pub people_group_tier_c: usize,
}

impl DescriptionStats {
    pub fn person_filled(&self) -> usize {
        self.person_tier_a + self.person_tier_b + self.person_tier_c
    }
    pub fn place_filled(&self) -> usize {
        self.place_tier_b + self.place_tier_c
    }
    pub fn people_group_filled(&self) -> usize {
        self.people_group_tier_c
    }
}

struct EastonLexicon<'a> {
    /// tier (b), Person: Theographic person slug (e.g. "aaron_1", the EXACT
    /// id space a compiled `PersonId` already uses) -> dictText.
    person_by_slug: HashMap<String, &'a str>,
    /// tier (b), Place: lowercased Theographic place display name -> dictText.
    place_by_name: HashMap<String, &'a str>,
    /// tier (c), any kind: lowercased `dict_lookup` -> dictText.
    by_dict_lookup: HashMap<String, &'a str>,
}

/// Builds a `key -> text` map, dropping entirely any key that maps to more than one DISTINCT text:
/// ambiguity means no description rather than a guess. Repeat rows with identical text are kept.
fn collision_checked<'a>(pairs: impl Iterator<Item = (String, &'a str)>) -> HashMap<String, &'a str> {
    let mut map: HashMap<String, &'a str> = HashMap::new();
    let mut ambiguous: HashSet<String> = HashSet::new();
    for (key, text) in pairs {
        if ambiguous.contains(&key) {
            continue;
        }
        match map.get(key.as_str()) {
            None => {
                map.insert(key, text);
            }
            Some(existing) if *existing == text => {}
            Some(_) => {
                map.remove(&key);
                ambiguous.insert(key);
            }
        }
    }
    map
}

fn build_lexicon(entries: &[EastonEntry]) -> EastonLexicon<'_> {
    let person_pairs = entries.iter().filter_map(|e| e.person_slug.as_deref().map(|slug| (slug.to_string(), e.dict_text.as_str())));
    let place_pairs = entries.iter().filter_map(|e| e.place_name.as_deref().map(|name| (name.to_string(), e.dict_text.as_str())));
    let lookup_pairs = entries
        .iter()
        .filter(|e| !e.dict_lookup.trim().is_empty())
        .map(|e| (e.dict_lookup.to_lowercase(), e.dict_text.as_str()));

    EastonLexicon { person_by_slug: collision_checked(person_pairs), place_by_name: collision_checked(place_pairs), by_dict_lookup: collision_checked(lookup_pairs) }
}

/// Mutates every already-normalized Place/Person/PeopleGroup node's `description` in place: no new
/// nodes and no new relation rows, a payload fill only.
pub fn fill_descriptions(ctx: &mut BuildCtx) -> DescriptionStats {
    let atlas = ctx.atlas;
    let lex = build_lexicon(&atlas.easton);
    let person_dict_text: HashMap<&str, &str> = atlas.people.iter().filter_map(|p| p.dict_text.as_deref().map(|t| (p.id.as_str(), t))).collect();

    let mut stats = DescriptionStats::default();

    for node in ctx.graph.nodes.values_mut() {
        match &mut node.payload {
            NodePayload::Person { label, description, .. } => {
                stats.person_total += 1;
                if let Some(text) = person_dict_text.get(node.id.raw.as_str()).copied() {
                    *description = Some(text.to_string());
                    stats.person_tier_a += 1;
                } else if let Some(text) = lex.person_by_slug.get(node.id.raw.as_str()).copied() {
                    *description = Some(text.to_string());
                    stats.person_tier_b += 1;
                } else if let Some(text) = lex.by_dict_lookup.get(&label.to_lowercase()).copied() {
                    *description = Some(text.to_string());
                    stats.person_tier_c += 1;
                }
            }
            NodePayload::Place { canonical, description, .. } => {
                stats.place_total += 1;
                let key = canonical.to_lowercase();
                if let Some(text) = lex.place_by_name.get(&key).copied() {
                    *description = Some(text.to_string());
                    stats.place_tier_b += 1;
                } else if let Some(text) = lex.by_dict_lookup.get(&key).copied() {
                    *description = Some(text.to_string());
                    stats.place_tier_c += 1;
                }
            }
            NodePayload::PeopleGroup { label, description } => {
                stats.people_group_total += 1;
                if let Some(text) = lex.by_dict_lookup.get(&label.to_lowercase()).copied() {
                    *description = Some(text.to_string());
                    stats.people_group_tier_c += 1;
                }
            }
            _ => {}
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, EastonEntry, Person};
    use atlas_graph_types::id::{NodeKind, PeopleGroupId, PersonId, PlaceId};
    use atlas_graph_types::ingest::ProvenanceId;
    use atlas_graph_types::node::Node;
    use std::collections::HashMap;

    fn person(id: &str, name: &str, dict_text: Option<&str>) -> Person {
        Person { id: id.into(), name: name.into(), gender: None, birth_year: None, death_year: None, also_called: vec![], verse_links: vec![], dict_text: dict_text.map(str::to_string), ..Default::default() }
    }

    fn easton(dict_lookup: &str, dict_text: &str, match_type: &str, match_slugs: &str, person_slug: Option<&str>, place_name: Option<&str>) -> EastonEntry {
        EastonEntry {
            dict_lookup: dict_lookup.into(),
            dict_text: dict_text.into(),
            match_type: match_type.into(),
            match_slugs: match_slugs.into(),
            person_slug: person_slug.map(str::to_string),
            place_name: place_name.map(str::to_string),
        }
    }

    fn place_node(id: &str, canonical: &str) -> Node {
        Node { id: PlaceId::new(id).erase(), payload: NodePayload::Place { canonical: canonical.into(), lat: 0.0, lon: 0.0, aliases: vec![], description: None }, provenance: ProvenanceId::from("test") }
    }

    fn person_node(id: &str, label: &str) -> Node {
        Node {
            id: PersonId::new(id).erase(),
            payload: NodePayload::Person { label: label.into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None, first_year: None, last_year: None, eternal: false, eternal_grounds: vec![] },
            provenance: ProvenanceId::from("test"),
        }
    }

    fn people_group_node(id: &str, label: &str) -> Node {
        Node { id: PeopleGroupId::new(id).erase(), payload: NodePayload::PeopleGroup { label: label.into(), description: None }, provenance: ProvenanceId::from("test") }
    }

    fn atlas_with(people: Vec<Person>, easton: Vec<EastonEntry>) -> AtlasData {
        let mut d = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.people = people;
        d.easton = easton;
        d
    }

    fn ctx_with<'a>(canon: &'a Canon, verses: &'a HashMap<String, String>, atlas: &'a AtlasData) -> BuildCtx<'a> {
        BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas)
    }

    #[test]
    fn tier_a_person_dict_text_fills_verbatim() {
        let atlas = atlas_with(vec![person("moses_1", "Moses", Some("The great lawgiver of Israel."))], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PersonId::new("moses_1").erase(), person_node("moses_1", "Moses"));

        let stats = fill_descriptions(&mut ctx);
        assert_eq!(stats.person_tier_a, 1);
        assert_eq!(stats.person_filled(), 1);

        let node = ctx.graph.nodes.get(&PersonId::new("moses_1").erase()).unwrap();
        match &node.payload {
            NodePayload::Person { description, .. } => assert_eq!(description.as_deref(), Some("The great lawgiver of Israel.")),
            other => panic!("expected Person, got {other:?}"),
        }
    }

    #[test]
    fn no_match_at_any_tier_leaves_description_none() {
        let atlas = atlas_with(vec![person("ghost_1", "Ghostperson", None)], vec![easton("Unrelated", "Some unrelated entry.", "unmatched", "unmatched", None, None)]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PersonId::new("ghost_1").erase(), person_node("ghost_1", "Ghostperson"));
        ctx.graph.nodes.insert(PlaceId::new("nowhere").erase(), place_node("nowhere", "Nowhereland"));

        let stats = fill_descriptions(&mut ctx);
        assert_eq!(stats.person_filled(), 0);
        assert_eq!(stats.place_filled(), 0);

        let pnode = ctx.graph.nodes.get(&PersonId::new("ghost_1").erase()).unwrap();
        match &pnode.payload {
            NodePayload::Person { description, .. } => assert_eq!(*description, None),
            other => panic!("expected Person, got {other:?}"),
        }
        let plnode = ctx.graph.nodes.get(&PlaceId::new("nowhere").erase()).unwrap();
        match &plnode.payload {
            NodePayload::Place { description, .. } => assert_eq!(*description, None),
            other => panic!("expected Place, got {other:?}"),
        }
    }

    #[test]
    fn filled_description_is_byte_identical_to_the_source_string_never_built() {
        let src = "Line one.\n\n Line two, with an inline [Ex. 6:20](/exod#Exod.6.20) link -- verbatim.  ";
        let atlas = atlas_with(vec![person("aaron_1", "Aaron", Some(src))], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PersonId::new("aaron_1").erase(), person_node("aaron_1", "Aaron"));

        fill_descriptions(&mut ctx);
        let node = ctx.graph.nodes.get(&PersonId::new("aaron_1").erase()).unwrap();
        match &node.payload {
            NodePayload::Person { description, .. } => assert_eq!(description.as_deref(), Some(src), "must be the EXACT source string, no trim/rebuild/concatenation"),
            other => panic!("expected Person, got {other:?}"),
        }
    }

    #[test]
    fn tier_b_matchslugs_hit_fills_a_place() {
        let atlas = atlas_with(vec![], vec![easton("Ammonite", "The usual name of the descendants of Ammon, the son of Lot.", "place", "ammon_58", None, Some("ammon"))]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PlaceId::new("ammon").erase(), place_node("ammon", "Ammon"));

        let stats = fill_descriptions(&mut ctx);
        assert_eq!(stats.place_tier_b, 1);
        let node = ctx.graph.nodes.get(&PlaceId::new("ammon").erase()).unwrap();
        match &node.payload {
            NodePayload::Place { description, .. } => assert_eq!(description.as_deref(), Some("The usual name of the descendants of Ammon, the son of Lot.")),
            other => panic!("expected Place, got {other:?}"),
        }
    }

    #[test]
    fn tier_c_literal_dict_lookup_fills_a_place_when_tier_b_has_no_hit() {
        let atlas = atlas_with(vec![], vec![easton("Hebron", "A city in the mountains of Judah.", "unmatched", "unmatched", None, None)]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PlaceId::new("hebron").erase(), place_node("hebron", "Hebron"));

        let stats = fill_descriptions(&mut ctx);
        assert_eq!(stats.place_tier_c, 1);
        let node = ctx.graph.nodes.get(&PlaceId::new("hebron").erase()).unwrap();
        match &node.payload {
            NodePayload::Place { description, .. } => assert_eq!(description.as_deref(), Some("A city in the mountains of Judah.")),
            other => panic!("expected Place, got {other:?}"),
        }
    }

    #[test]
    fn tier_c_match_is_case_insensitive_but_exact_no_stemming_no_fuzzy() {
        let atlas = atlas_with(vec![], vec![easton("HEBRON", "A city.", "unmatched", "unmatched", None, None)]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PlaceId::new("hebron").erase(), place_node("hebron", "Hebron"));
        ctx.graph.nodes.insert(PlaceId::new("hebronite-town").erase(), place_node("hebronite-town", "Hebronite"));

        fill_descriptions(&mut ctx);
        let hebron = ctx.graph.nodes.get(&PlaceId::new("hebron").erase()).unwrap();
        match &hebron.payload {
            NodePayload::Place { description, .. } => assert_eq!(description.as_deref(), Some("A city.")),
            other => panic!("expected Place, got {other:?}"),
        }
        let hebronite = ctx.graph.nodes.get(&PlaceId::new("hebronite-town").erase()).unwrap();
        match &hebronite.payload {
            NodePayload::Place { description, .. } => assert_eq!(*description, None, "case-insensitive EXACT match only -- must not fuzzy-match a substring/stem"),
            other => panic!("expected Place, got {other:?}"),
        }
    }

    #[test]
    fn ambiguous_matchslugs_target_backs_off_tier_b_but_tier_c_can_still_resolve_it() {
        let atlas = atlas_with(
            vec![],
            vec![
                easton("Ammon", "The land of the children of Ammon.", "place", "ammon_58", None, Some("ammon")),
                easton("Ammonite", "The usual name of the descendants of Ammon, the son of Lot.", "place", "ammon_58", None, Some("ammon")),
            ],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PlaceId::new("ammon").erase(), place_node("ammon", "Ammon"));

        let stats = fill_descriptions(&mut ctx);
        assert_eq!(stats.place_tier_b, 0, "the colliding matchSlugs target must NOT be tier-b guessed");
        assert_eq!(stats.place_tier_c, 1, "tier c independently resolves it via the exact-name entry");
        let node = ctx.graph.nodes.get(&PlaceId::new("ammon").erase()).unwrap();
        match &node.payload {
            NodePayload::Place { description, .. } => assert_eq!(description.as_deref(), Some("The land of the children of Ammon.")),
            other => panic!("expected Place, got {other:?}"),
        }
    }

    #[test]
    fn ambiguous_dict_lookup_with_differing_text_resolves_to_none_not_a_guess() {
        let atlas = atlas_with(vec![], vec![easton("Salt", "Text A about salt.", "unmatched", "unmatched", None, None), easton("SALT", "Text B, a different entry.", "unmatched", "unmatched", None, None)]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PlaceId::new("salt").erase(), place_node("salt", "Salt"));

        let stats = fill_descriptions(&mut ctx);
        assert_eq!(stats.place_filled(), 0, "ambiguous key -> None, never an arbitrary pick");
    }

    #[test]
    fn tier_a_takes_priority_over_tier_b_and_c_for_a_person() {
        let atlas = atlas_with(
            vec![person("aaron_1", "Aaron", Some("Tier A text."))],
            vec![easton("Aaron", "Tier C text (must not win).", "person", "aaron_1", Some("aaron_1"), None)],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PersonId::new("aaron_1").erase(), person_node("aaron_1", "Aaron"));

        let stats = fill_descriptions(&mut ctx);
        assert_eq!(stats.person_tier_a, 1);
        assert_eq!(stats.person_tier_b, 0);
        assert_eq!(stats.person_tier_c, 0);
        let node = ctx.graph.nodes.get(&PersonId::new("aaron_1").erase()).unwrap();
        match &node.payload {
            NodePayload::Person { description, .. } => assert_eq!(description.as_deref(), Some("Tier A text.")),
            other => panic!("expected Person, got {other:?}"),
        }
    }

    #[test]
    fn people_group_fills_from_tier_c_only() {
        let atlas = atlas_with(vec![], vec![easton("Ammonites", "The people descended from Ammon.", "unmatched", "unmatched", None, None)]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        ctx.graph.nodes.insert(PeopleGroupId::new("ammonites").erase(), people_group_node("ammonites", "Ammonites"));

        let stats = fill_descriptions(&mut ctx);
        assert_eq!(stats.people_group_total, 1);
        assert_eq!(stats.people_group_tier_c, 1);
        let node = ctx.graph.nodes.get(&PeopleGroupId::new("ammonites").erase()).unwrap();
        match &node.payload {
            NodePayload::PeopleGroup { description, .. } => assert_eq!(description.as_deref(), Some("The people descended from Ammon.")),
            other => panic!("expected PeopleGroup, got {other:?}"),
        }
    }

    #[test]
    fn non_described_node_kinds_are_left_untouched() {
        let atlas = atlas_with(vec![], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let era_id = atlas_graph_types::id::EraId::new("iron-age").erase();
        ctx.graph.nodes.insert(era_id.clone(), Node { id: era_id, payload: NodePayload::Era { label: "Iron Age".into(), from_year: -1200, to_year: -586 }, provenance: ProvenanceId::from("test") });

        let stats = fill_descriptions(&mut ctx);
        assert_eq!(stats.person_total, 0);
        assert_eq!(stats.place_total, 0);
        assert_eq!(stats.people_group_total, 0);
    }

    #[test]
    fn node_kind_matches_stay_sane_across_the_three_kinds() {
        assert_eq!(PersonId::new("x").erase().kind, NodeKind::Person);
        assert_eq!(PlaceId::new("x").erase().kind, NodeKind::Place);
        assert_eq!(PeopleGroupId::new("x").erase().kind, NodeKind::PeopleGroup);
    }
}
