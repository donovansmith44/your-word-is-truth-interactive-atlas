mod common;

use atlas_graph_types::id::{PersonId, PlaceId};
use atlas_graph_types::node::NodePayload;

#[test]
fn hebron_and_moses_carry_non_empty_descriptions_over_the_real_compiled_data() {
    let inputs = common::PipelineInputs::read();

    let ctx = inputs.run();

    let hebron_id = PlaceId::new("hebron").erase();
    let hebron = ctx.graph.nodes.get(&hebron_id).expect("a compiled Place node with id 'hebron' must exist over the real geo data");
    let hebron_desc = match &hebron.payload {
        NodePayload::Place { canonical, description, .. } => {
            assert_eq!(canonical, "Hebron");
            description.clone()
        }
        other => panic!("expected NodePayload::Place for 'hebron', got {other:?}"),
    };
    assert!(hebron_desc.as_deref().is_some_and(|s| !s.trim().is_empty()), "Hebron must carry a non-empty description over the real compiled data, got {hebron_desc:?}");
    println!("HEBRON description ({} chars): {}", hebron_desc.as_ref().unwrap().len(), &hebron_desc.as_ref().unwrap()[..hebron_desc.as_ref().unwrap().len().min(120)]);

    let moses_id = PersonId::new("moses_2108").erase();
    let moses = ctx.graph.nodes.get(&moses_id).expect("a compiled Person node with id 'moses_2108' must exist over the real Theographic people data");
    let moses_desc = match &moses.payload {
        NodePayload::Person { label, description, .. } => {
            assert_eq!(label, "Moses");
            description.clone()
        }
        other => panic!("expected NodePayload::Person for 'moses_1', got {other:?}"),
    };
    assert!(moses_desc.as_deref().is_some_and(|s| !s.trim().is_empty()), "Moses must carry a non-empty description over the real compiled data, got {moses_desc:?}");
    println!("MOSES description ({} chars): {}", moses_desc.as_ref().unwrap().len(), &moses_desc.as_ref().unwrap()[..moses_desc.as_ref().unwrap().len().min(120)]);
}

#[test]
fn description_fill_rates_over_the_real_compiled_data_are_reported_honestly() {
    let atlas = common::real_atlas();
    let inputs = common::PipelineInputs::read();

    let ctx = inputs.run();
    let s = &ctx.description_stats;

    println!("ENT-1a DESCRIPTION FILL RATES (real compiled data):");
    println!(
        "  persons: {}/{} filled ({:.1}%) -- tier a {}, tier b {}, tier c {}",
        s.person_filled(),
        s.person_total,
        100.0 * s.person_filled() as f64 / s.person_total.max(1) as f64,
        s.person_tier_a,
        s.person_tier_b,
        s.person_tier_c
    );
    println!(
        "  places:  {}/{} filled ({:.1}%) -- tier b {}, tier c {}",
        s.place_filled(),
        s.place_total,
        100.0 * s.place_filled() as f64 / s.place_total.max(1) as f64,
        s.place_tier_b,
        s.place_tier_c
    );
    println!(
        "  people-groups: {}/{} filled ({:.1}%) -- tier c {}",
        s.people_group_filled(),
        s.people_group_total,
        100.0 * s.people_group_filled() as f64 / s.people_group_total.max(1) as f64,
        s.people_group_tier_c
    );

    assert_eq!(s.person_total, atlas.people.len() - atlas.people_group_reclassify.len(), "every compiled person must be counted exactly once -- MINUS PG-1a's own nine reclassified Gen-10 gentilics, which are PeopleGroup nodes now, not Person");
    assert!(s.person_filled() >= 2000, "person fill count regressed below a sane floor: {} of {}", s.person_filled(), s.person_total);
    assert!(s.place_filled() >= 600, "place fill count regressed below a sane floor: {} of {}", s.place_filled(), s.place_total);
    assert_eq!(s.people_group_total, 38, "PG-1a's own three-source PeopleGroup node total (23 Theographic + 6 curated seeds + 9 reclassified) -- update this in the SAME commit as a future PG-batch that changes the group roster");
    println!("PG-1a group fill rate (real compiled data): {}/{} filled ({:.1}%) -- tier c {}", s.people_group_filled(), s.people_group_total, 100.0 * s.people_group_filled() as f64 / s.people_group_total.max(1) as f64, s.people_group_tier_c);
}
