//! Batch KJV-CASE (owner ruling; batch-kjv-case-brief.md): reads the four
//! spot-law verses back from the REAL COMMITTED `data/compiled/graph.bin`
//! artifact itself -- not a freshly-built in-memory graph (every other
//! real-data test in this workspace exercises the BUILD path; this one
//! exercises the SHIPPED FILE, the actual thing `atlas-server` loads at
//! startup and the actual thing this batch's own report quotes PSA 110:1
//! from). Read-back discipline: a report claim about "the restored output"
//! is verified here against bytes on disk, not against this batch's own
//! intent.

use std::path::Path;

use atlas_graph::GraphService;

fn real_graph_bin() -> GraphService {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/graph.bin");
    GraphService::from_artifact(&path).expect("data/compiled/graph.bin must exist and load -- run the atlas-etl + atlas-graph-compile pipeline first")
}

fn verse_text(svc: &GraphService, book_code: &str, chapter: u16, verse: u16) -> String {
    let book_index = atlas_core::canon::resolve_alias(book_code).unwrap_or_else(|| panic!("'{book_code}' must resolve to a canonical book")).0;
    let id = atlas_graph::kjv_adapter::verse_node_id(book_index, chapter, verse);
    atlas_graph::window::render(&svc.snapshot(), &id).unwrap_or_else(|| panic!("{book_code}.{chapter}.{verse} must render from the real artifact"))
}

/// The batch brief's own four spot laws (controller decision 3), read back
/// from the actual shipped `graph.bin` -- the SAME four verified in
/// `atlas_etl::brainfuel::restore_kjv_case`'s own real-data test
/// (`brainfuel_real_data.rs`), here proven to have actually REACHED the
/// artifact this repository ships, not just the in-memory build.
#[test]
fn spot_law_verses_read_back_from_the_real_committed_artifact() {
    let svc = real_graph_bin();

    // PSA 110:1: a superscription-folded position, DISCLOSED as
    // unrestored -- our own kjv.json folds "A Psalm of David." into verse
    // 1's own text; brain-fuel's king_james column does not, so the two
    // are never case-fold-equal and the case-only law forbids touching
    // this position. Still carries the load-bearing "The Lord said unto
    // my Lord" -- both Tetragrammaton-referent forms in one verse -- just
    // with the FIRST occurrence's own case left as our source always had
    // it, honestly.
    assert_eq!(
        verse_text(&svc, "PSA", 110, 1),
        "A Psalm of David. The Lord said unto my Lord, Sit thou at my right hand, until I make thine enemies thy footstool."
    );

    // GEN 2:4-class: "LORD God" (YHWH Elohim) -- restored.
    assert_eq!(
        verse_text(&svc, "GEN", 2, 4),
        "These are the generations of the heavens and of the earth when they were created, in the day that the LORD God made the earth and the heavens,"
    );

    // EZK 2:4: "Lord GOD" (Adonai YHWH), the Ezekiel-class convention --
    // restored.
    assert_eq!(
        verse_text(&svc, "EZK", 2, 4),
        "For they are impudent children and stiffhearted. I do send thee unto them; and thou shalt say unto them, Thus saith the Lord GOD."
    );

    // PSA 68:4: JAH -- restored.
    assert_eq!(
        verse_text(&svc, "PSA", 68, 4),
        "Sing unto God, sing praises to his name: extol him that rideth upon the heavens by his name JAH, and rejoice before him."
    );
}
