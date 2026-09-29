mod common;

use atlas_graph::GraphService;

fn verse_text(svc: &GraphService, book_code: &str, chapter: u16, verse: u16) -> String {
    let book_index = atlas_core::canon::resolve_alias(book_code).unwrap_or_else(|| panic!("'{book_code}' must resolve to a canonical book")).0;
    let id = atlas_graph::kjv_adapter::verse_node_id(book_index, chapter, verse);
    atlas_graph::window::render(&svc.snapshot(), &id).unwrap_or_else(|| panic!("{book_code}.{chapter}.{verse} must render from the real artifact"))
}

#[test]
fn spot_law_verses_read_back_from_the_real_committed_artifact() {
    let svc = common::committed_service();

    assert_eq!(
        verse_text(svc, "PSA", 110, 1),
        "A Psalm of David. The LORD said unto my Lord, Sit thou at my right hand, until I make thine enemies thy footstool."
    );

    assert_eq!(verse_text(svc, "PSA", 23, 1), "A Psalm of David. The LORD is my shepherd; I shall not want.");

    assert_eq!(
        verse_text(svc, "GEN", 2, 4),
        "These are the generations of the heavens and of the earth when they were created, in the day that the LORD God made the earth and the heavens,"
    );

    assert_eq!(
        verse_text(svc, "EZK", 2, 4),
        "For they are impudent children and stiffhearted. I do send thee unto them; and thou shalt say unto them, Thus saith the Lord GOD."
    );

    assert_eq!(
        verse_text(svc, "PSA", 68, 4),
        "Sing unto God, sing praises to his name: extol him that rideth upon the heavens by his name JAH, and rejoice before him."
    );
}

#[test]
fn superscription_exclusions_carry_our_canonical_casing_untouched_in_the_real_artifact() {
    let svc = common::committed_service();

    let (_canon, our_verses) = atlas_etl::kjv::parse(&common::kjv_json()).expect("kjv.json must parse");

    for (dot_ref, book, chapter, verse) in [("PSA.70.1", "PSA", 70u16, 1u16), ("PSA.92.1", "PSA", 92, 1), ("ACT.9.29", "ACT", 9, 29)] {
        let ours = our_verses.get(dot_ref).unwrap_or_else(|| panic!("{dot_ref} must exist in our own canonical kjv.json"));
        let artifact_text = verse_text(svc, book, chapter, verse);
        assert_eq!(
            &artifact_text, ours,
            "{dot_ref} is a brainfuel::SUPERSCRIPTION_EXCLUSIONS entry -- must carry OUR OWN canonical casing, untouched, in the real shipped artifact"
        );
    }
}
