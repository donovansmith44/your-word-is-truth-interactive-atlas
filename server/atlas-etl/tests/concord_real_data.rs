use atlas_etl::concord;

mod common;

use common::{curated_dir, raw_dir};

fn corpus() -> concord::ConcordCorpus {
    concord::read_all(&raw_dir().join("concord"), &curated_dir()).expect("data/raw/concord must exist -- run data/fetch-raw.ps1 first")
}

#[test]
fn ten_documents_parse_clean_with_the_expected_per_document_paragraph_counts() {
    let c = corpus();
    assert_eq!(c.documents.len(), 10);

    let counts: Vec<(u8, &str, usize, usize)> = c.documents.iter().map(|d| (d.part, d.key, d.articles.len(), d.articles.iter().map(|a| a.paragraphs.len()).sum())).collect();

    let expected: Vec<(u8, &str, usize, usize)> = vec![
        (1, "preface", 1, 25),
        (2, "ecumenical-creeds", 3, 11),
        (3, "augsburg-confession", 30, 451),
        (4, "defense", 26, 1162),
        (5, "smalcald-articles", 28, 222),
        (6, "power-and-primacy", 3, 114),
        (7, "small-catechism", 9, 70),
        (8, "large-catechism", 7, 748),
        (9, "epitome", 13, 265),
        (10, "solid-declaration", 14, 734),
    ];
    assert_eq!(counts, expected, "per-document (part, key, article_count, paragraph_count)");

    let total_paragraphs: usize = counts.iter().map(|(_, _, _, p)| p).sum();
    assert_eq!(c.stats.paragraphs, total_paragraphs);
    assert_eq!(c.stats.documents, 10);
    assert_eq!(c.stats.skipped_articles, 3, "the three non-Triglot Small Catechism articles on concord-exclusions.toml");
}

#[test]
fn spine_order_is_canonical_part_then_article_then_paragraph_ascending() {
    let c = corpus();
    let seq: Vec<(u8, u16, u16)> = c.iter_paragraphs().map(|(part, article, _a, p)| (part, article, p.paragraph)).collect();
    let mut sorted = seq.clone();
    sorted.sort();
    assert_eq!(seq, sorted, "iter_paragraphs already walks in canonical (part, article, paragraph) order -- no re-sort needed downstream");
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
    let article_nums: Vec<u16> = doc.articles.iter().map(|a| a.article).collect();
    assert_eq!(article_nums, (1..=28).collect::<Vec<_>>());
    let of_sin = doc.articles.iter().find(|a| a.slug == "/smalcald-articles/iii/of-sin/").unwrap();
    assert!(of_sin.paragraphs[0].text.starts_with("Here we must confess, as Paul says in Rom. 5:12, that sin originated"));
}

const SMALL_CATECHISM_TITLES_AS_SERVED: [&str; 9] = [
    "Luther's Preface to the Small Catechism",
    "I. The Ten Commandments",
    "II. The Creed",
    "III. The Lord's Prayer",
    "IV. The Sacrament of Holy Baptism",
    "V. Confession",
    "VI. The Sacrament of the Altar",
    "Daily Prayers",
    "Table of Duties",
];

#[test]
fn the_small_catechism_articles_are_numbered_by_chief_part_and_the_appendices_are_not() {
    // Arrange
    let c = corpus();
    // Act
    let titles: Vec<&str> = c.documents.iter().find(|d| d.key == "small-catechism").unwrap().articles.iter().map(|a| a.title.as_str()).collect();
    // Assert
    assert_eq!(titles, SMALL_CATECHISM_TITLES_AS_SERVED);
}

#[test]
fn the_served_concord_is_triglot_only_no_excluded_piece_and_no_non_triglot_marker_is_served() {
    // Arrange
    let exclusions = concord::parse_concord_exclusions(&std::fs::read_to_string(curated_dir().join("concord-exclusions.toml")).unwrap()).unwrap();
    // Act
    let c = corpus();
    // Assert
    concord::no_excluded_material_is_served(&c, &exclusions).unwrap();
    concord::no_served_text_carries_a_non_triglot_marker(&c, &exclusions.markers).unwrap();
}

#[test]
fn apology_xviii_serves_the_triglot_paragraphs_67_to_76_each_under_its_own_source_number() {
    // Arrange
    let c = corpus();
    // Act
    let of_free_will: Vec<(u16, String)> = c.documents.iter().find(|d| d.key == "defense").unwrap().articles.iter().find(|a| a.slug == "/defense/of-free-will/").unwrap().paragraphs.iter().map(|p| (p.paragraph, p.source_label.clone())).collect();
    // Assert
    assert_eq!(of_free_will, (67..=76).map(|n| (n, n.to_string())).collect::<Vec<_>>());
}

#[test]
fn the_apostles_creed_serves_its_three_triglot_paragraphs_and_no_note() {
    // Arrange
    let c = corpus();
    // Act
    let creed: Vec<String> = c.documents.iter().find(|d| d.key == "ecumenical-creeds").unwrap().articles[0].paragraphs.iter().map(|p| p.text.clone()).collect();
    // Assert
    assert_eq!(creed.len(), 3);
    assert_eq!(creed[2], "I believe in the Holy Ghost; the holy catholic Church, the communion of saints; the forgiveness of sins; the resurrection of the body; and the life everlasting. Amen.");
}
