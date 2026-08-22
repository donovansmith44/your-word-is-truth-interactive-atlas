//! GraphVersion: the "one stamp invalidates all caches" identity of a built
//! graph (design doc §6: "Windows are cacheable by (cursor, N) keyed to a
//! graph version stamp"; types doc §6: `pub struct GraphVersion(Hash);`).
//!
//! NOT a graph-types type -- `GraphVersion`/`Cursor`/`Opaque` are prose in
//! the types spec but are absent from the compiled `graph-types` crate
//! (confirmed: no such items in `graph-types/src/*.rs`), so "the crate wins"
//! (brief law) means there is nothing to extend here, only server-side
//! infrastructure to add. This is a skeleton stand-in matching graph-types'
//! own documented simplification for `ContentHash` ("a 64-bit std hash;
//! production: a real multihash") -- the graph is immutable between builds
//! (one build per server run in M-A), so a single hash computed once at
//! build time is sufficient to invalidate ETags across a restart.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use atlas_graph_types::graph::Graph;
use atlas_graph_types::node::NodePayload;

use crate::kjv_adapter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphVersion(pub u64);

impl GraphVersion {
    /// Hashes the bible reading spine's own ordered (id, KJV rendering)
    /// pairs plus the cites-row count -- any content change to the built
    /// graph (a different KJV source, a different cross-references source)
    /// changes this stamp. Order-sensitive by construction (hashing the
    /// spine in its own order), so the reconstruction law's own ordering is
    /// part of what this stamp certifies too.
    pub fn compute(g: &Graph) -> Self {
        let mut h = DefaultHasher::new();
        if let Some(spine) = g.reading.get(kjv_adapter::BIBLE_CORPUS) {
            for id in &spine.order {
                id.raw.hash(&mut h);
                if let Some(node) = g.nodes.get(id) {
                    if let NodePayload::TextUnit { renderings, .. } = &node.payload {
                        for (layer, text) in renderings {
                            layer.0.hash(&mut h);
                            text.hash(&mut h);
                        }
                    }
                }
            }
        }
        g.cross_refs.len().hash(&mut h);
        GraphVersion(h.finish())
    }

    /// The wire/ETag form -- fixed-width lowercase hex, so equal stamps are
    /// byte-identical strings (no whitespace/case ambiguity for
    /// `If-None-Match` comparisons).
    pub fn as_hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

impl std::fmt::Display for GraphVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::build_graph_from_sources;

    const KJV_A: &str = r#"{"translation":"KJV","books":[{"name":"Genesis","chapters":[{"chapter":1,"verses":[{"verse":1,"text":"In the beginning."}]}]}]}"#;
    const KJV_B: &str = r#"{"translation":"KJV","books":[{"name":"Genesis","chapters":[{"chapter":1,"verses":[{"verse":1,"text":"A different beginning."}]}]}]}"#;
    const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

    #[test]
    fn same_source_same_stamp_different_source_different_stamp() {
        let (g1, _) = build_graph_from_sources(KJV_A, NO_XREFS).unwrap();
        let (g2, _) = build_graph_from_sources(KJV_A, NO_XREFS).unwrap();
        let (g3, _) = build_graph_from_sources(KJV_B, NO_XREFS).unwrap();
        assert_eq!(GraphVersion::compute(&g1).as_hex(), GraphVersion::compute(&g2).as_hex());
        assert_ne!(GraphVersion::compute(&g1).as_hex(), GraphVersion::compute(&g3).as_hex());
    }
}
