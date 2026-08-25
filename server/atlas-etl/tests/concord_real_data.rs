//! Batch CORP-2a, requirement 9 (per-document paragraph counts asserted
//! from the parse tally; fidelity spot-checks byte-compared against the
//! real source): runs `atlas_etl::concord::read_all` over the real,
//! committed `data/raw/concord/*.html` (vendored via `data/fetch-raw.ps1`
//! -- see `data/raw/README.md`'s own Concord section) and pins the real
//! counts this batch's report quotes verbatim.

use std::path::Path;

use atlas_etl::concord;

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw/concord")
}

fn corpus() -> concord::ConcordCorpus {
    concord::read_all(&root()).expect("data/raw/concord must exist -- run data/fetch-raw.ps1 first")
}

#[test]
fn ten_documents_parse_clean_with_the_expected_per_document_paragraph_counts() {
    let c = corpus();
    assert_eq!(c.documents.len(), 10);

    let counts: Vec<(u8, &str, usize, usize)> = c.documents.iter().map(|d| (d.part, d.key, d.articles.len(), d.articles.iter().map(|a| a.paragraphs.len()).sum())).collect();

    // Pinned against the real vendored pages (batch-corp2a-report.md
    // quotes this exact table). A change here means the SOURCE changed
    // (or the parser did) -- investigate, never just bump the number.
    let expected: Vec<(u8, &str, usize, usize)> = vec![
        (1, "preface", 1, 25),
        (2, "ecumenical-creeds", 3, 13),
        (3, "augsburg-confession", 30, 451),
        (4, "defense", 26, 1164),
        (5, "smalcald-articles", 28, 222),
        (6, "power-and-primacy", 3, 114),
        (7, "small-catechism", 10, 91),
        (8, "large-catechism", 7, 748),
        (9, "epitome", 13, 265),
        (10, "solid-declaration", 14, 734),
    ];
    assert_eq!(counts, expected, "per-document (part, key, article_count, paragraph_count)");

    let total_paragraphs: usize = counts.iter().map(|(_, _, _, p)| p).sum();
    assert_eq!(c.stats.paragraphs, total_paragraphs);
    assert_eq!(c.stats.documents, 10);
    assert_eq!(c.stats.skipped_articles, 2, "the two Small Catechism site-furniture articles (prefatory notes + PDF plug)");
}

#[test]
fn spine_order_is_canonical_part_then_article_then_paragraph_ascending() {
    let c = corpus();
    let seq: Vec<(u8, u16, u16)> = c.iter_paragraphs().map(|(part, article, _a, p)| (part, article, p.paragraph)).collect();
    let mut sorted = seq.clone();
    sorted.sort();
    assert_eq!(seq, sorted, "iter_paragraphs already walks in canonical (part, article, paragraph) order -- no re-sort needed downstream");
    // Distinct triples -- every paragraph position is unique (no collision
    // survived the parser's own remap-and-disclose collision guard).
    let distinct: std::collections::BTreeSet<_> = seq.iter().collect();
    assert_eq!(distinct.len(), seq.len(), "every (part, article, paragraph) triple is unique");
}

#[test]
fn the_sc_first_commandment_paragraph_is_verbatim_and_matches_the_existing_catechism_toml_wording() {
    let c = corpus();
    let sc = c.documents.iter().find(|d| d.key == "small-catechism").unwrap();
    let ten_commandments = sc.articles.iter().find(|a| a.slug == "/small-catechism/ten-commandments/").unwrap();
    let first = ten_commandments.paragraphs.iter().find(|p| p.paragraph == 1).unwrap();
    assert_eq!(
        first.text,
        "Thou shalt have no other gods. What does this mean? \u{2013}Answer: We should fear, love, and trust in God above all things."
    );
    // catechism.toml's own First Commandment (Wikisource-sourced, straight
    // apostrophes/dashes; this parse is bookofconcord.org-sourced, curly/
    // en-dash) -- independently-transcribed, same 1921 translation: the
    // load-bearing WORDS agree exactly modulo that disclosed punctuation
    // convention (LICENSES.md's own "Editorial handling").
    assert!(first.text.contains("We should fear, love, and trust in God above all things"));
}

#[test]
fn ecumenical_creeds_has_no_native_numbering_disclosed_for_all_three_creeds() {
    let c = corpus();
    let creeds = c.documents.iter().find(|d| d.key == "ecumenical-creeds").unwrap();
    assert_eq!(creeds.articles.len(), 3);
    for a in &creeds.articles {
        for p in &a.paragraphs {
            assert_eq!(p.source_label, "", "no source-native label for any Ecumenical Creeds paragraph");
        }
    }
    let synthetic_disclosures = c.stats.disclosures.iter().filter(|d| d.starts_with("ecumenical-creeds") && d.contains("no native paragraph numbering")).count();
    assert_eq!(synthetic_disclosures, 3, "one disclosure per creed article");
}

#[test]
fn skipped_articles_are_named_in_the_disclosures() {
    let c = corpus();
    assert!(c.stats.disclosures.iter().any(|d| d.contains("prefaratory-notes") && d.contains("skipped")));
    assert!(c.stats.disclosures.iter().any(|d| d.contains("small-catechism-pdf") && d.contains("skipped")));
}

#[test]
fn smalcald_articles_extras_are_spliced_after_their_own_part_blurb_in_toc_order() {
    // The Smalcald Articles exception (module doc comment): Parts I/II/III
    // own root-page sections are one-paragraph blurbs; the real 4+4+15
    // articles are spliced in from `smalcald-sub/*.html`, right after
    // their own Part.
    let c = corpus();
    let doc = c.documents.iter().find(|d| d.key == "smalcald-articles").unwrap();
    let slugs: Vec<&str> = doc.articles.iter().map(|a| a.slug.as_str()).collect();
    assert_eq!(doc.articles.len(), 28, "preface + 3 part-blurbs + 4+4+15 spliced articles + signatures");
    assert_eq!(slugs[0], "/smalcald-articles/preface/");
    assert_eq!(slugs[1], "/smalcald-articles/i/", "Part I's own blurb stays (real, if brief, editorial text)");
    assert_eq!(&slugs[2..6], &["/smalcald-articles/i/nature-of-god/", "/smalcald-articles/i/the-father/", "/smalcald-articles/i/the-son/", "/smalcald-articles/i/the-work-of-salvation/"]);
    assert_eq!(slugs[6], "/smalcald-articles/ii/");
    assert_eq!(&slugs[7..11], &["/smalcald-articles/ii/first-and-chief-article/", "/smalcald-articles/ii/of-the-mass/", "/smalcald-articles/ii/of-chapters-and-cloisters/", "/smalcald-articles/ii/of-the-papacy/"]);
    assert_eq!(slugs[11], "/smalcald-articles/iii/");
    assert_eq!(slugs[12], "/smalcald-articles/iii/of-sin/");
    assert_eq!(slugs[26], "/smalcald-articles/iii/of-human-tradition/", "the 15th and last Part III sub-article");
    assert_eq!(slugs[27], "/smalcald-articles/signatories/");
    // Article numbers are sequential 1..28 in this final, spliced order.
    let article_nums: Vec<u16> = doc.articles.iter().map(|a| a.article).collect();
    assert_eq!(article_nums, (1..=28).collect::<Vec<_>>());
    // The spliced content is real: "Article I - Of Sin" opens with Paul's
    // own Romans 5:12 citation (verbatim spot-check against the real page).
    let of_sin = doc.articles.iter().find(|a| a.slug == "/smalcald-articles/iii/of-sin/").unwrap();
    assert!(of_sin.paragraphs[0].text.starts_with("Here we must confess, as Paul says in Rom. 5:12, that sin originated"));
}
