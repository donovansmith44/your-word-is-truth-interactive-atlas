use std::collections::BTreeMap;

use atlas_etl::kretzmann::{self, Calendar, DeviationClass};

mod common;

use common::raw_dir;

fn real_corpus() -> kretzmann::KretzmannCorpus {
    let kjv_verses = real_kjv_verses();
    kretzmann::read_all(&raw_dir().join("kretzmann"), &kjv_verses).expect("read_all must succeed over the real vendored corpus (data/fetch-raw.ps1 must have run)")
}

fn real_kjv_verses() -> std::collections::HashMap<String, String> {
    let kjv_json = std::fs::read_to_string(raw_dir().join("kjv.json")).expect("data/raw/kjv.json must exist");
    let (_canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
    verses
}

fn real_canonical() -> BTreeMap<(u8, u16, u16), String> {
    let dir = raw_dir();
    let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let (canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
    let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist");
    let (restored, _report) = atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &verses);

    let mut canonical = BTreeMap::new();
    for book in &canon.books {
        let book_index = atlas_core::canon::resolve_alias(&book.code).expect("every compiled book code resolves").0;
        for (chapter_idx, &verse_count) in book.chapters.iter().enumerate() {
            let chapter = (chapter_idx + 1) as u16;
            for v in 1..=verse_count {
                let key = format!("{}.{}.{}", book.code, chapter, v);
                if let Some(text) = restored.get(&key) {
                    canonical.insert((book_index, chapter, v), text.clone());
                }
            }
        }
    }
    canonical
}

#[test]
fn real_corpus_covers_all_1189_pages_with_the_pinned_unit_and_fragment_totals() {
    let corpus = real_corpus();
    assert_eq!(corpus.stats.pages, 1189, "comprehensive = all 66 books, every chapter page (controller decision 1)");
    assert_eq!(corpus.chapters.len(), 1189);
    assert_eq!(corpus.stats.units, 50602);
    assert_eq!(corpus.stats.fragments, 61374);
    assert_eq!(corpus.stats.footnotes, 257);
    assert_eq!(corpus.stats.footnotes_in_lemma, 0, "a footnote landing inside an excised lemma/quote span was never observed in the real corpus");
    assert_eq!(corpus.stats.over_excisions, 1046);
    assert_eq!(corpus.stats.inline_verse_markers, 8);
    assert_eq!(corpus.stats.disclosures.len(), 1069);
}

#[test]
fn gen_1_2_is_the_named_multi_unit_verse_spot_check() {
    let corpus = real_corpus();
    let gen1 = corpus.chapters.iter().find(|c| c.book_index == 0 && c.chapter == 1).expect("Genesis 1 must be in the corpus");
    let v2_fragments: Vec<&str> = gen1.fragments.iter().filter(|f| f.verse == 2).map(|f| f.text.as_str()).collect();
    assert_eq!(
        v2_fragments,
        vec!["And the earth was without form and void.", "And darkness was upon the face of the deep.", "And the Spirit of God moved upon the face of the waters.",],
        "GEN 1:2's own three-fragment split -- the flagship multi-unit-verse case (batch brief's own required quote)"
    );
    let v2_units: Vec<&kretzmann::KretzUnit> = gen1.units.iter().filter(|u| u.verse_from == 2 && u.verse_to == 2).collect();
    assert_eq!(v2_units.len(), 3, "three separate CommentaryItems, each comments-on the SAME verse 2 -- legitimate per the verse-mapped-index law");
}

#[test]
fn psa_110_1_the_restored_case_verse_reconciles_under_the_disclosed_mechanical_class() {
    let corpus = real_corpus();
    let canonical = real_canonical();
    let psa110 = corpus.chapters.iter().find(|c| c.book_index == 18 && c.chapter == 110).expect("Psalm 110 must be in the corpus");

    let canon_v1 = canonical.get(&(18u8, 110u16, 1u16)).expect("PSA.110.1 must be in the real canonical map");
    assert!(canon_v1.contains("The LORD said unto my Lord"), "the RESTORED canonical text must carry the Tetragrammaton case convention: {canon_v1:?}");

    let v1_fragment_count = psa110.fragments.iter().filter(|f| f.verse == 1).count();
    assert!(v1_fragment_count >= 4, "superscription + 'The Lord said...' + 'Sit Thou...' + 'until I make...' -- got {v1_fragment_count}");

    let mut single = BTreeMap::new();
    single.insert((18u8, 110u16, 1u16), canon_v1.clone());
    let report = kretzmann::check_conservation(&psa110.fragments.iter().filter(|f| f.verse == 1).cloned().collect::<Vec<_>>(), &single);
    assert_eq!(report.mismatches.len(), 0, "PSA 110:1 must reconcile: {:#?}", report.mismatches);
    assert_eq!(report.exact + report.mechanical + report.mechanical_spelling, 1);
}

#[test]
fn jhn_3_16_the_type_b_gospel_chapter_spot_check_reconciles() {
    let corpus = real_corpus();
    let canonical = real_canonical();
    let jhn3 = corpus.chapters.iter().find(|c| c.book_index == 42 && c.chapter == 3).expect("John 3 must be in the corpus");
    let frag = jhn3.fragments.iter().find(|f| f.verse == 16).expect("JHN 3:16 must have an excised fragment (Type B block quote)");
    assert!(frag.text.contains("For God so loved the world"), "got: {:?}", frag.text);

    let mut single = BTreeMap::new();
    single.insert((42u8, 3u16, 16u16), canonical.get(&(42u8, 3u16, 16u16)).unwrap().clone());
    let report = kretzmann::check_conservation(&[frag.clone()], &single);
    assert_eq!(report.mismatches.len(), 0, "JHN 3:16 must reconcile: {:#?}", report.mismatches);

    let unit = jhn3.units.iter().find(|u| u.verse_from <= 16 && u.verse_to >= 16).expect("a unit covering verse 16 must exist");
    assert!(unit.text.len() > 500, "JHN 3:14-17's own real commentary is substantial prose, got {} chars", unit.text.len());
    assert!(unit.text.contains("brazen serpent"), "Kretzmann's own typological discussion must survive verbatim in the stored prose");
}

#[test]
fn kretz_accept_1_conservation_law_over_the_whole_real_corpus_has_the_pinned_shape() {
    let corpus = real_corpus();
    let canonical = real_canonical();
    assert_eq!(canonical.len(), 31102, "the whole KJV, restored");

    let all_fragments: Vec<kretzmann::ExcisedFragment> = corpus.chapters.iter().flat_map(|c| c.fragments.iter().cloned()).collect();
    assert_eq!(all_fragments.len(), 61374);

    let report = kretzmann::check_conservation(&all_fragments, &canonical);

    assert_eq!(report.checked, 31040);
    assert_eq!(report.exact, 2525);
    assert_eq!(report.mechanical, 23614);
    assert_eq!(report.mechanical_spelling, 1903);
    assert_eq!(report.mismatches.len(), 2998);
    assert_eq!(report.uncovered.len(), 62, "verses Kretzmann summarizes without a lemma of their own -- lawful (decision 3), not an error");

    assert_eq!(report.exact + report.mechanical + report.mechanical_spelling + report.mismatches.len(), report.checked);
    assert_eq!(report.checked + report.uncovered.len(), canonical.len());

    let gen_2_19 = report.mismatches.iter().find(|m| m.book_index == 0 && m.chapter == 2 && m.verse == 19).expect("GEN 2:19 must be a real, disclosed mismatch");
    assert_eq!(gen_2_19.class, DeviationClass::Mismatch);
}

#[test]
fn every_date_clause_verbatim_is_a_real_substring_of_its_own_units_prose_over_the_whole_real_corpus() {
    let corpus = real_corpus();
    let mut total = 0usize;
    let mut by_calendar: BTreeMap<&str, usize> = BTreeMap::new();
    let mut approx_count = 0usize;
    for chapter in &corpus.chapters {
        for unit in &chapter.units {
            for clause in kretzmann::extract_date_clauses(&unit.text) {
                assert!(unit.text.contains(&clause.verbatim), "unit {} own clause {:?} must be a real substring of its own stored prose", unit.id, clause.verbatim);
                let roundtrip = kretzmann::extract_date_clauses(&clause.verbatim);
                assert_eq!(roundtrip.len(), 1, "clause {:?} must round-trip to exactly one clause parsed from itself alone", clause.verbatim);
                assert_eq!(roundtrip[0].calendar, clause.calendar);
                assert_eq!(roundtrip[0].year, clause.year);
                assert_eq!(roundtrip[0].approx, clause.approx);
                total += 1;
                if clause.approx {
                    approx_count += 1;
                }
                *by_calendar
                    .entry(match clause.calendar {
                        Calendar::Bc => "BC",
                        Calendar::Ad => "AD",
                        Calendar::Am => "AM",
                    })
                    .or_insert(0) += 1;
            }
        }
    }
    assert_eq!(total, 84);
    assert_eq!(by_calendar.get("BC").copied().unwrap_or(0), 43);
    assert_eq!(by_calendar.get("AD").copied().unwrap_or(0), 41);
    assert_eq!(by_calendar.get("AM").copied().unwrap_or(0), 0, "zero Anno Mundi clauses found in the real corpus -- disclosed, not assumed absent");
    assert!(approx_count > 0, "at least one 'about' approximation must be found in real prose this size");
}

fn strong_span_starting_with(html: &str, anchor_prefix: &str) -> String {
    let marker = format!("<strong>{anchor_prefix}");
    let start = html.find(&marker).unwrap_or_else(|| panic!("no <strong>{anchor_prefix}... span found in the real source"));
    let inner_start = start + "<strong>".len();
    let end = html[inner_start..].find("</strong>").unwrap_or_else(|| panic!("no closing </strong> after {anchor_prefix:?}"));
    html[inner_start..inner_start + end].to_string()
}

const INLINE_VERSE_MARKER_INSTANCES: &[(u8, u16, u16, u16, &str)] = &[
    (39, 26, 61, 60, "and said, This fellow said, I am able to destroy the Temple of God"),
    (39, 27, 40, 39, "and saying, Thou that destroyest the Temple"),
    (41, 2, 35, 34, "(yea, a sword shall pierce through thy own soul also)"),
    (41, 17, 21, 20, "neither shall they say, Lo here"),
    (41, 19, 42, 41, "saying, If thou hadst known"),
    (41, 19, 46, 45, "saying unto them, It is written"),
    (41, 20, 2, 1, "and spake unto Him, saying, Tell us, by what authority"),
    (41, 20, 36, 35, "neither can they die any more"),
];

#[test]
fn inline_verse_marker_instances_reclassify_to_their_own_verse_not_the_preceding_ones_prose() {
    let corpus = real_corpus();
    for &(book_index, chapter, swallowed_verse, host_verse, distinctive_phrase) in INLINE_VERSE_MARKER_INSTANCES {
        let ch = corpus.chapters.iter().find(|c| c.book_index == book_index && c.chapter == chapter).unwrap_or_else(|| panic!("book_index {book_index} chapter {chapter} must be in the corpus"));
        let frag = ch
            .fragments
            .iter()
            .find(|f| f.verse == swallowed_verse && f.text.contains(distinctive_phrase))
            .unwrap_or_else(|| panic!("verse {swallowed_verse} (book_index {book_index}, chapter {chapter}) must carry its own real excised fragment containing {distinctive_phrase:?}"));
        assert!(frag.text.len() > 10, "the swallowed verse's own fragment must be real, non-trivial content, got {:?}", frag.text);
        for u in ch.units.iter().filter(|u| u.verse_from <= host_verse && u.verse_to >= host_verse) {
            assert!(
                !u.text.contains(distinctive_phrase),
                "book_index {book_index} chapter {chapter}: host verse {host_verse}'s own unit {} must NOT still carry verse {swallowed_verse}'s own text {distinctive_phrase:?} as if it were Kretzmann's prose -- unit text {:?}",
                u.id,
                u.text
            );
        }
    }
}

#[test]
fn mat_26_60_to_61_inline_verse_marker_lands_as_v61_lemma_not_v60_prose_end_to_end() {
    let html = std::fs::read_to_string(raw_dir().join("kretzmann/matthew/26.html")).expect("data/raw/kretzmann/matthew/26.html must exist");
    assert!(html.contains(" v. 61 and said, This fellow said, I am able to destroy the Temple of God, and to build it in three days. "), "the real source's own inline marker text must still read as originally traced");
    assert!(!html.contains(r#"<sup id="v61">"#), "MAT 26:61 must still lack a real <sup> marker in the source -- otherwise this test's own premise no longer holds");

    let corpus = real_corpus();
    let canonical = real_canonical();
    let mat26 = corpus.chapters.iter().find(|c| c.book_index == 39 && c.chapter == 26).expect("Matthew 26 must be in the corpus");

    let v60_frag = mat26.fragments.iter().find(|f| f.verse == 60).expect("MAT 26:60 must have its own excised fragment");
    assert!(!v60_frag.text.contains("This fellow said"), "verse 60's own fragment must not run on into verse 61's own text: {:?}", v60_frag.text);
    let canon_60 = canonical.get(&(39u8, 26u16, 60u16)).expect("MAT.26.60 must be in the real canonical map");
    let report_60 = kretzmann::check_conservation(&[v60_frag.clone()], &BTreeMap::from([((39u8, 26u16, 60u16), canon_60.clone())]));
    assert_eq!(report_60.mismatches.len(), 0, "MAT 26:60 must reconcile cleanly against its own canonical text: {:#?}", report_60.mismatches);

    let v61_frag = mat26.fragments.iter().find(|f| f.verse == 61).expect("MAT 26:61 must now have its own excised fragment (fix round 2)");
    assert!(v61_frag.text.to_lowercase().starts_with("and said"), "verse 61's own fragment must open with its own real text, got {:?}", v61_frag.text);
    let canon_61 = canonical.get(&(39u8, 26u16, 61u16)).expect("MAT.26.61 must be in the real canonical map");
    let report_61 = kretzmann::check_conservation(&[v61_frag.clone()], &BTreeMap::from([((39u8, 26u16, 61u16), canon_61.clone())]));
    assert_eq!(report_61.mismatches.len(), 0, "MAT 26:61 must reconcile cleanly against ITS OWN canonical text, not verse 60's: {:#?}", report_61.mismatches);

    for u in mat26.units.iter().filter(|u| u.verse_from <= 60 && u.verse_to >= 60) {
        assert!(!u.text.contains("This fellow said"), "MAT 26:60's own stored prose must no longer carry verse 61's own swallowed KJV text: {:?}", u.text);
    }
}

#[test]
fn over_excision_guard_recovers_exo_20_12_and_rut_4_11_prose_verbatim_against_the_source_html() {
    let corpus = real_corpus();

    let exo20_html = std::fs::read_to_string(raw_dir().join("kretzmann/exodus/20.html")).expect("data/raw/kretzmann/exodus/20.html must exist");
    let exo_span = strong_span_starting_with(&exo20_html, "with heart, mouth");
    let exo = corpus.chapters.iter().find(|c| c.book_index == 1 && c.chapter == 20).expect("Exodus 20 must be in the corpus");
    let exo_unit = exo
        .units
        .iter()
        .find(|u| u.verse_from <= 12 && u.verse_to >= 12 && u.text.starts_with("with heart, mouth"))
        .expect("EXO 20:12's own recovered-prose unit must exist");
    let exo_frag = exo.fragments.iter().find(|f| f.verse == 12 && f.text.starts_with("that thy days")).expect("EXO 20:12's own recovered lemma fragment must exist");
    let exo_recovered = exo_unit.text.split(" It is the first commandment").next().unwrap();
    assert_eq!(format!("{exo_recovered} {}", exo_frag.text), exo_span, "EXO 20:12: recovered prose + excised lemma must reconstruct the real source <strong> span exactly");

    let rut4_html = std::fs::read_to_string(raw_dir().join("kretzmann/ruth/4.html")).expect("data/raw/kretzmann/ruth/4.html must exist");
    let rut_span = strong_span_starting_with(&rut4_html, "The Lord make the woman");
    let rut = corpus.chapters.iter().find(|c| c.book_index == 7 && c.chapter == 4).expect("Ruth 4 must be in the corpus");
    let rut_unit = rut
        .units
        .iter()
        .find(|u| u.verse_from <= 11 && u.verse_to >= 11 && u.text.starts_with("literally, that is about to come"))
        .expect("RUT 4:11's own recovered-infix unit must exist");
    let rut_frag = rut.fragments.iter().find(|f| f.verse == 11 && f.text.contains("did build the house of Israel")).expect("RUT 4:11's own recovered lemma fragment must exist");
    let rut_recovered = rut_unit.text.split(" as the mothers").next().unwrap();
    let split_at = rut_frag.text.find("into thine house,").expect("the genuine KJV prefix half must be present") + "into thine house,".len();
    let (rut_prefix, rut_suffix) = rut_frag.text.split_at(split_at);
    assert_eq!(
        format!("{rut_prefix} {rut_recovered} {}", rut_suffix.trim_start()),
        rut_span,
        "RUT 4:11: genuine-KJV-prefix + recovered-infix-prose + genuine-KJV-suffix must reconstruct the real source <strong> span exactly"
    );
}

#[test]
fn stored_prose_never_contains_its_own_excised_fragment_text() {
    let corpus = real_corpus();
    let mut checked = 0usize;
    for chapter in &corpus.chapters {
        for unit in &chapter.units {
            if unit.kind != kretzmann::UnitKind::Verse || unit.verse_from != unit.verse_to {
                continue;
            }
            for frag in &chapter.fragments {
                if frag.verse != unit.verse_from || frag.text.len() < 30 {
                    continue;
                }
                checked += 1;
                assert!(
                    !unit.text.contains(&frag.text),
                    "unit {} (verse {}) stored prose contains its own excised fragment text verbatim -- fragment {:?} inside unit text {:?}",
                    unit.id,
                    unit.verse_from,
                    frag.text,
                    unit.text
                );
            }
        }
    }
    assert!(checked > 30_000, "the full-corpus sweep must have actually run at real scale, got {checked} unit/fragment pairs checked");

    let exo = corpus.chapters.iter().find(|c| c.book_index == 1 && c.chapter == 20).expect("Exodus 20 must be in the corpus");
    let exo_frag = exo.fragments.iter().find(|f| f.verse == 12 && f.text.starts_with("that thy days")).expect("EXO 20:12's own recovered lemma fragment must exist");
    for u in exo.units.iter().filter(|u| u.verse_from <= 12 && u.verse_to >= 12) {
        assert!(!u.text.contains(&exo_frag.text), "EXO 20:12's own recovered prose must not contain its own excised lemma text");
    }
    let rut = corpus.chapters.iter().find(|c| c.book_index == 7 && c.chapter == 4).expect("Ruth 4 must be in the corpus");
    let rut_frag = rut.fragments.iter().find(|f| f.verse == 11 && f.text.contains("did build the house of Israel")).expect("RUT 4:11's own recovered lemma fragment must exist");
    for u in rut.units.iter().filter(|u| u.verse_from <= 11 && u.verse_to >= 11) {
        assert!(!u.text.contains(&rut_frag.text), "RUT 4:11's own recovered prose must not contain its own excised lemma text");
    }
    let mat = corpus.chapters.iter().find(|c| c.book_index == 39 && c.chapter == 26).expect("Matthew 26 must be in the corpus");
    let mat_frag = mat.fragments.iter().find(|f| f.verse == 60 && f.text.starts_with("but found none")).expect("MAT 26:60's own recovered lemma fragment must exist");
    for u in mat.units.iter().filter(|u| u.verse_from <= 60 && u.verse_to >= 60) {
        assert!(!u.text.contains(&mat_frag.text), "MAT 26:60's own recovered prose must not contain its own excised lemma text");
    }
}

#[test]
fn kretz_accept_2_composed_reading_view_strips_to_exactly_the_whole_canonical_bible() {
    let corpus = real_corpus();
    let canonical = real_canonical();
    assert_eq!(canonical.len(), 31102, "all 31,102 verses, including the 70 Kretzmann summarizes without a lemma of their own");

    let segments = kretzmann::compose_reading_view(&canonical, &corpus);
    let verse_segment_count = segments.iter().filter(|s| matches!(s, kretzmann::ReadingViewSegment::Verse(_))).count();
    assert_eq!(verse_segment_count, 31102, "exactly one Verse segment per canonical verse -- spine coverage, no skip, no duplicate");

    let comment_segment_count = segments.iter().filter(|s| matches!(s, kretzmann::ReadingViewSegment::Comment(_))).count();
    assert!(comment_segment_count > 50_000, "the real corpus's own comments must actually be present in the composed view, got {comment_segment_count}");

    let stripped = kretzmann::strip_comment_blocks(&segments);
    let whole_bible: String = canonical.values().map(|s| s.as_str()).collect();
    assert_eq!(stripped, whole_bible, "EXACT byte-compare, no equivalence tiers, no residual");
}

#[test]
fn every_one_of_the_66_books_has_at_least_one_unit() {
    let corpus = real_corpus();
    let mut units_by_book: BTreeMap<u8, usize> = BTreeMap::new();
    for chapter in &corpus.chapters {
        *units_by_book.entry(chapter.book_index).or_insert(0) += chapter.units.len();
    }
    assert_eq!(units_by_book.len(), 66, "every one of the 66 books must contribute at least one unit");
    for (book, count) in &units_by_book {
        assert!(*count > 0, "book_index {book} carries zero units");
    }
}
