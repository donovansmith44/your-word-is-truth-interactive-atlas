//! SVEB-1: the Svebilius parser against the REAL vendored files.
//!
//! Same discipline as `concord_real_data.rs`/`brainfuel_real_data.rs`: the
//! unit tests in `svebilius.rs` pin the grammar on hand-written fragments;
//! this pins the parse against the actual source, so a re-vendor that
//! changes the file's shape fails here rather than silently producing
//! fewer edges.

use std::collections::HashMap;
use std::path::PathBuf;

use atlas_etl::svebilius;

/// The pinned commit the catechism repo is vendored at -- the SAME sha
/// `data/fetch-raw.ps1` and `data/curated/catechism-mapping.toml` name.
const CATECHISM_SHA: &str = "0be24fee92e6333f817c4c2a08f99cf7c5274295";

fn svebilius_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/raw/catechism-mapping")
        .join(format!("catechism-{CATECHISM_SHA}"))
        .join("svebilius/en")
}

/// The real compiled KJV text, which `canonicalize_ref` needs both to
/// expand ranges and to reject a ref naming a verse this atlas lacks.
fn real_verses() -> HashMap<String, String> {
    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
        .expect("data/raw + data/curated must compile")
        .data
        .verses
}

#[test]
fn the_exposition_parses_to_its_own_printed_shape() {
    let verses = real_verses();
    let (units, stats, _renderings) =
        svebilius::read_all(&svebilius_dir(), &verses).expect("the vendored Svebilius files must parse");

    // 314 numbered questions across the seven Q&A sections -- counted off
    // the real file, not assumed.
    assert_eq!(stats.qa_units, 314, "Q&A unit count moved: {stats:?}");

    // Every section in SECTIONS is 1-based and in source order.
    for u in &units {
        assert!(
            (1..=svebilius::SECTIONS.len() as u8).contains(&u.section),
            "unit {}.{} has an out-of-range section",
            u.section,
            u.unit
        );
    }

    // The two prose sections carry units with no question, and the seven
    // Q&A sections carry only units that have one.
    for u in &units {
        if u.section >= 8 {
            assert!(u.question.is_none(), "Sveb {}.{} in a prose section should carry no question", u.section, u.unit);
        } else {
            assert!(u.question.is_some(), "Sveb {}.{} in a Q&A section should carry a question", u.section, u.unit);
        }
    }
}

#[test]
fn the_sources_own_numbering_gap_is_carried_not_closed() {
    let verses = real_verses();
    let (_units, stats, _) = svebilius::read_all(&svebilius_dir(), &verses).unwrap();

    // Section 4 (the Lord's Prayer) prints 53 questions but numbers to 54:
    // the source itself skips 53. Renumbering would put this edition
    // permanently out of step with every printed copy, so the gap stands
    // and is reported. If a re-vendor ever fixes the source, THIS is the
    // test that should be updated -- deliberately, with the new source in
    // hand.
    assert_eq!(stats.numbering_gaps, vec![(4, 53)], "numbering gaps moved: {:?}", stats.numbering_gaps);
}

#[test]
fn answers_yield_real_resolvable_scripture_edges() {
    let verses = real_verses();
    let (units, stats, _) = svebilius::read_all(&svebilius_dir(), &verses).unwrap();

    // The whole point of the batch: the Exposition contributes real edges.
    assert!(stats.verse_links > 250, "expected a substantial verse harvest, got {}", stats.verse_links);

    // Every harvested ref is canonical and actually exists in this atlas's
    // compiled text -- `canonicalize_ref` guarantees it, and this asserts
    // the guarantee rather than trusting it.
    for u in &units {
        for v in &u.verses {
            assert!(verses.contains_key(v), "Sveb {}.{} cites {v}, which is not in the compiled KJV", u.section, u.unit);
        }
    }

    // A spot-check against the real text: the Preface's second question
    // grounds "why are you called a Christian" in Galatians 3:27 and
    // Acts 4:12, both printed inline in its answer.
    let q2 = units
        .iter()
        .find(|u| u.section == 1 && u.unit == 2)
        .expect("Preface question 2 must parse");
    assert!(q2.verses.contains(&"GAL.3.27".to_string()), "{:?}", q2.verses);
    assert!(q2.verses.contains(&"ACT.4.12".to_string()), "{:?}", q2.verses);
}

#[test]
fn unresolved_citations_are_disclosed_not_dropped() {
    let verses = real_verses();
    let (_units, stats, _) = svebilius::read_all(&svebilius_dir(), &verses).unwrap();

    // Not asserted to be empty -- an abbreviation the canon table does not
    // carry is a real possibility in a 1745 translation, and the honest
    // treatment is to REPORT it. What is asserted is that the reporting
    // channel works and that the failure rate is small enough that the
    // parse is trustworthy rather than mostly-broken.
    let harvested = stats.verse_links + stats.unresolved.len();
    assert!(
        stats.unresolved.len() * 20 < harvested,
        "too many unresolved citations ({} of {harvested}): {:?}",
        stats.unresolved.len(),
        &stats.unresolved[..stats.unresolved.len().min(10)]
    );
}

#[test]
fn the_juslenius_rendering_covers_the_catechisms_own_headings() {
    let verses = real_verses();
    let (_units, _stats, renderings) = svebilius::read_all(&svebilius_dir(), &verses).unwrap();

    // The Ten Commandments alone account for ten headings; the edition
    // also prints the Creed, the Lord's Prayer, and the sacraments.
    assert!(renderings.len() >= 10, "expected the full catechism, got {} headings", renderings.len());

    let first = renderings
        .iter()
        .find(|r| r.heading == "The First Commandment")
        .expect("The First Commandment must be rendered");
    // This edition's wording differs from the Triglotta's ("We should fear,
    // love, and trust in God above all things") -- which is exactly why it
    // is worth carrying as a parallel rendering rather than deduplicated
    // against the text already in hand.
    assert!(first.text.contains("fear and love God above all things"), "{}", first.text);
}
