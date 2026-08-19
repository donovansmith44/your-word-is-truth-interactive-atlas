use atlas_core::{time::TimeRange, wire::*};
use std::{fs, path::Path};

fn sample_scene() -> Scene {
    Scene {
        mode: "time".into(),
        window: Some(TimeRange::new(-1450, -1400).unwrap()),
        sref: None,
        places: vec![ScenePlace {
            id: "jericho".into(), name: "Jericho".into(), display_name: "Jericho".into(), lat: 31.8703, lon: 35.4436,
            brightness: 2,
            events: vec![SceneEvent {
                id: "ev_jericho_falls".into(), label: "The walls of Jericho fall".into(),
                when: TimeRange::new(-1406, -1406).unwrap(),
                verse_groups: vec![VerseGroup { book: "JOS".into(), chapter: 6,
                    verses: vec!["JOS.6.1".into(), "JOS.6.20".into()], count: 27 }],
            }],
        }],
        arrows: vec![SceneArrow { narrative: "conquest".into(), color: "#7C3AED".into(),
            from_place: "gilgal".into(), to_place: "jericho".into(),
            from_event: "ev_camp_gilgal".into(), to_event: "ev_jericho_falls".into(), order: 2 }],
        narratives: vec![SceneNarrative { id: "conquest".into(), name: "The Conquest".into(),
            color: "#7C3AED".into(), legs_in_scene: 4 }],
    }
}

#[test]
fn golden_scene_fixture() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/golden-scene.json");
    let json = serde_json::to_string_pretty(&sample_scene()).unwrap();
    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &json).unwrap();
    }
    let on_disk = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    assert_eq!(on_disk.trim(), json.trim(), "wire format drifted from committed golden fixture");
    let back: Scene = serde_json::from_str(&on_disk).unwrap();
    assert_eq!(back, sample_scene());
}
