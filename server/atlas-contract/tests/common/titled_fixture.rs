#![allow(dead_code)]
use atlas_core::data::{demo_fixture, AtlasData};

pub fn titled_demo_fixture() -> AtlasData {
    let registry = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/curated/sources.toml");
    let mut data = demo_fixture();
    data.provenance_titles = atlas_etl::sources::parse_sources(&std::fs::read_to_string(registry).unwrap()).unwrap().provenance_titles().unwrap();
    data
}
