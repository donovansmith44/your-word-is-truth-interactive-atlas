//! Batch KJV-CASE (owner ruling; batch-kjv-case-brief.md): reads the four
//! spot-law verses back from the REAL COMMITTED `data/compiled/graph.bin`
//! artifact itself -- not a freshly-built in-memory graph (every other
//! real-data test in this workspace exercises the BUILD path; this one
//! exercises the SHIPPED FILE, the actual thing `atlas-server` loads at
//! startup and the actual thing this batch's own report quotes PSA 110:1
//! from). Read-back discipline: a report claim about "the restored output"
//! is verified here against bytes on disk, not against this batch's own
//! intent.
//!
//! Batch KJV-CASE-2 (batch-kjv-case2-brief.md) adds a fifth verse, PSA
//! 23:1 (a second superscription-class Psalm, controller decision 4), and
//! changes PSA 110:1's own assertion from "disclosed as unrestored" to
//! "restored" -- see that test's own doc comment for the full history.

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
///
/// Batch KJV-CASE-2 (batch-kjv-case2-brief.md, owner ruling verbatim "do
/// it (superscript fix)"): PSA 110:1's own assertion below CHANGED from
/// this file's own pre-KJV-CASE-2 history -- it used to assert PSA 110:1
/// stayed UNRESTORED (a disclosed superscription-folded skip, pass 1's
/// own whole-verse gate cannot reach a folded-in superscription). The
/// TAIL-ALIGNMENT RULE now reaches it: the folded-in superscription "A
/// Psalm of David. " keeps its own canonical casing and characters
/// UNTOUCHED; the aligned tail's first Tetragrammaton occurrence ("the
/// Lord said") is promoted to "the LORD said"; the second occurrence
/// ("my Lord", Adonai, David's own lord) correctly stays initial-cap
/// only -- both forms now distinguished in the one verse, exactly as
/// controller decision 4 names it: "the flagship becomes a permanent
/// law."
#[test]
fn spot_law_verses_read_back_from_the_real_committed_artifact() {
    let svc = real_graph_bin();

    // PSA 110:1: superscription-tail-restored (batch KJV-CASE-2).
    assert_eq!(
        verse_text(&svc, "PSA", 110, 1),
        "A Psalm of David. The LORD said unto my Lord, Sit thou at my right hand, until I make thine enemies thy footstool."
    );

    // PSA 23:1: a second superscription-class Psalm restored (controller
    // decision 4: "at least one more superscription-class Psalm
    // restored") -- "A Psalm of David. " kept untouched; "the Lord" (the
    // ONLY Tetragrammaton occurrence in this verse) promoted to "the LORD".
    assert_eq!(verse_text(&svc, "PSA", 23, 1), "A Psalm of David. The LORD is my shepherd; I shall not want.");

    // GEN 2:4-class: "LORD God" (YHWH Elohim) -- restored (pass 1, unchanged).
    assert_eq!(
        verse_text(&svc, "GEN", 2, 4),
        "These are the generations of the heavens and of the earth when they were created, in the day that the LORD God made the earth and the heavens,"
    );

    // EZK 2:4: "Lord GOD" (Adonai YHWH), the Ezekiel-class convention --
    // restored (pass 1, unchanged).
    assert_eq!(
        verse_text(&svc, "EZK", 2, 4),
        "For they are impudent children and stiffhearted. I do send thee unto them; and thou shalt say unto them, Thus saith the Lord GOD."
    );

    // PSA 68:4: JAH -- restored (pass 1, unchanged).
    assert_eq!(
        verse_text(&svc, "PSA", 68, 4),
        "Sing unto God, sing praises to his name: extol him that rideth upon the heavens by his name JAH, and rejoice before him."
    );
}
