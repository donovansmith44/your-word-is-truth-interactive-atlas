#![allow(dead_code)]
use atlas_core::data::{demo_fixture, AtlasData};

pub fn titled_demo_fixture() -> AtlasData {
    titled(demo_fixture())
}

pub fn titled(mut data: AtlasData) -> AtlasData {
    let registry = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/curated/sources.toml");
    data.provenance_titles = atlas_etl::sources::admit_sources(&std::fs::read_to_string(registry).unwrap()).unwrap().provenance_titles().unwrap();
    data
}
