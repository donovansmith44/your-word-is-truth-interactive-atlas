//! The compile step: `atlas-graph-compile --data-dir data/compiled`, with `raw/` and `curated/` as that
//! directory's siblings. Before writing anything it rebuilds the graph a second time from the identical
//! sources and runs the conformance law between them, so that expensive check happens here, never at startup.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use atlas_graph::sqlite::source::sibling_dir;
use atlas_graph_types::store::{GraphPublisher, MemStore};

/// The sections' home IS `--data-dir`; `--sections-cache <dir>` overrides the unpack cache the uncompressed
/// files are written into, which defaults to `<data-dir>/../cache/sections`.
fn parse_args(args: &[String]) -> Result<(PathBuf, Option<PathBuf>)> {
    let mut data_dir: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut sections_cache: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data-dir" => {
                i += 1;
                data_dir = Some(PathBuf::from(args.get(i).context("--data-dir requires a value")?));
            }
            "--out" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).context("--out requires a value")?));
            }
            "--sections-cache" => {
                i += 1;
                sections_cache = Some(PathBuf::from(args.get(i).context("--sections-cache requires a value")?));
            }
            other => anyhow::bail!("unrecognized argument: {other}"),
        }
        i += 1;
    }
    let data_dir = data_dir.context("--data-dir is required, e.g. --data-dir ../data/compiled")?;
    if let Some(out) = out {
        println!("atlas-graph-compile: --out {} is retired (DB-5: graph.bin is no longer written; the sections under --data-dir are the artifact)", out.display());
    }
    Ok((data_dir, sections_cache))
}

fn main() -> Result<()> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let (data_dir, sections_cache) = parse_args(&raw)?;

    let raw_dir = sibling_dir(&data_dir, "raw");
    let curated_dir = sibling_dir(&data_dir, "curated");
    // `data/exports/` is a committed directory, unlike the gitignored raw tree.
    let exports_dir = sibling_dir(&data_dir, "exports");

    let atlas = atlas_etl::compile::compile(&raw_dir, &curated_dir).with_context(|| format!("compiling {} + {}", raw_dir.display(), curated_dir.display()))?.data;
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).with_context(|| format!("reading {}", raw_dir.join("kjv.json").display()))?;
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt"))
        .with_context(|| format!("reading {}", raw_dir.join("xrefs/cross_references.txt").display()))?;
    let eras = atlas.eras.clone();

    // The real compile step HARD-REQUIRES every vendored corpus, unlike the dev fallback that degrades
    // gracefully for a fixture directory: an artifact compiled without one would silently ship less text
    // than it claims.
    let brainfuel_root = raw_dir.join("brain-fuel-bible");
    println!("atlas-graph-compile: reading vendored brain-fuel editions from {}...", brainfuel_root.display());
    let brainfuel = atlas_etl::brainfuel::read_all(&brainfuel_root).with_context(|| format!("reading {}", brainfuel_root.display()))?;
    println!(
        "atlas-graph-compile: brain-fuel {} OT + {} NT chapters, {} verse rows, present renderings {:?}, absent markers {:?}, anomalies {}",
        brainfuel.stats.ot_chapters,
        brainfuel.stats.nt_chapters,
        brainfuel.rows.len(),
        brainfuel.stats.per_edition_present,
        brainfuel.stats.per_edition_absent,
        brainfuel.stats.anomalies,
    );

    // A separate, disclosed recomputation of the case restoration purely for this log, reusing the verses
    // already in scope. The restored map is captured because the red-letter alignment below needs it.
    let (restored_verses, case_restoration) = atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &atlas.verses);
    println!(
        "atlas-graph-compile: KJV-CASE restoration -- {} compared, {} case-restored (whole-verse), {} already agreeing, \
         {} superscription-tail-restored, {} excluded (brain-fuel artifacts), {} mirror-case (disclosed, untouched), \
         {} skipped (non-superscription folded-text mismatch, untouched)",
        case_restoration.compared,
        case_restoration.restored,
        case_restoration.already_agreeing,
        case_restoration.superscription_restored,
        case_restoration.excluded,
        case_restoration.mirror_case_found,
        case_restoration.skipped_mismatch,
    );

    let concord_root = raw_dir.join("concord");
    println!("atlas-graph-compile: reading vendored Concord (Book of Concord) data from {}...", concord_root.display());
    let concord_corpus = atlas_etl::concord::read_all(&concord_root, &curated_dir).with_context(|| format!("reading {}", concord_root.display()))?;
    let sc_overlap_path = curated_dir.join("concord-sc-overlap.toml");
    let sc_overlap_text = std::fs::read_to_string(&sc_overlap_path).with_context(|| format!("reading {}", sc_overlap_path.display()))?;
    let sc_overlap = atlas_etl::concord::parse_sc_overlap(&sc_overlap_text).with_context(|| format!("parsing {}", sc_overlap_path.display()))?;
    println!(
        "atlas-graph-compile: concord {} documents, {} articles, {} paragraphs ({} skipped non-confessional articles, {} disclosed structural anomalies), {} SC-overlap rows",
        concord_corpus.stats.documents,
        concord_corpus.stats.articles,
        concord_corpus.stats.paragraphs,
        concord_corpus.stats.skipped_articles,
        concord_corpus.stats.disclosures.len(),
        sc_overlap.len(),
    );
    let concord_bundle = atlas_graph::concord_adapter::ConcordBundle { corpus: concord_corpus, sc_overlap };

    let kretzmann_root = raw_dir.join("kretzmann");
    println!("atlas-graph-compile: reading vendored Kretzmann (Popular Commentary of the Bible) data from {}...", kretzmann_root.display());
    let kretzmann_corpus = atlas_etl::kretzmann::read_all(&kretzmann_root, &atlas.verses).with_context(|| format!("reading {}", kretzmann_root.display()))?;
    println!(
        "atlas-graph-compile: kretzmann {} pages, {} units, {} excised fragments, {} footnotes ({} disclosed structural anomalies)",
        kretzmann_corpus.stats.pages, kretzmann_corpus.stats.units, kretzmann_corpus.stats.fragments, kretzmann_corpus.stats.footnotes, kretzmann_corpus.stats.disclosures.len(),
    );

    let red_letter_root = raw_dir.join("red-letter");
    println!("atlas-graph-compile: reading vendored red-letter (KJV OSIS, words of Christ) data from {}...", red_letter_root.display());
    let red_letter_corpus = atlas_etl::red_letter::read_all(&red_letter_root, &restored_verses).with_context(|| format!("reading {}", red_letter_root.display()))?;
    println!(
        "atlas-graph-compile: red-letter {} verse-set, {} source spans ({} exact, {} case-insensitive, {} not-found, disclosed)",
        red_letter_corpus.stats.verses_with_source_markup,
        red_letter_corpus.stats.source_spans_total,
        red_letter_corpus.stats.exact,
        red_letter_corpus.stats.case_insensitive,
        red_letter_corpus.stats.not_found,
    );

    println!("atlas-graph-compile: reading vendored lexicon + morphology from {}...", brainfuel_root.display());
    let lexicon_corpus = atlas_etl::lexicon::read_all(&brainfuel_root).with_context(|| format!("reading {}/lexicon,morph", brainfuel_root.display()))?;
    println!(
        "atlas-graph-compile: LEX-1 corpus {} Greek + {} Hebrew entries; {} NT + {} OT tokens ({} + {} unmatched, by design); {} chapter files",
        lexicon_corpus.stats.entries_grc,
        lexicon_corpus.stats.entries_hbo,
        lexicon_corpus.stats.tokens_nt,
        lexicon_corpus.stats.tokens_ot,
        lexicon_corpus.stats.unmatched_nt,
        lexicon_corpus.stats.unmatched_ot,
        lexicon_corpus.stats.files,
    );

    println!("atlas-graph-compile: building implementation #1 (from raw sources)...");
    let build_start = Instant::now();
    let (graph_a, stats, event_world_stats, chrono) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter_and_lexicon(
        &kjv_json,
        &xrefs_tsv,
        &atlas,
        &eras,
        Some(&brainfuel),
        Some(&concord_bundle),
        Some(&kretzmann_corpus),
        Some(&red_letter_corpus),
        Some(&lexicon_corpus),
    )
    .context("building the graph from raw sources")?;
    println!(
        "atlas-graph-compile: {} text units, {} cites edges, {} events ({} dated), {} places, {} narratives, {} anchors -- build time {:?}",
        stats.kjv_verses, stats.cites_rows, event_world_stats.events, event_world_stats.dated_events, event_world_stats.places, event_world_stats.narratives, event_world_stats.anchors,
        build_start.elapsed()
    );
    println!(
        "atlas-graph-compile: LEX-1 -- {} LexiconEntry nodes, {} Occurs rows; skipped {} unmatched tokens, {} without an entry, {} off canon",
        stats.lexicon.entries, stats.lexicon.occurs, stats.lexicon.tokens_unmatched, stats.lexicon.tokens_without_entry, stats.lexicon.tokens_off_canon
    );
    let chronology = atlas_graph::Chronology::from_derivation(chrono);

    println!("atlas-graph-compile: ADMISSION -- rebuilding implementation #1 a second time (independent model)...");
    let (mut graph_b, ..) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter_and_lexicon(
        &kjv_json,
        &xrefs_tsv,
        &atlas,
        &eras,
        Some(&brainfuel),
        Some(&concord_bundle),
        Some(&kretzmann_corpus),
        Some(&red_letter_corpus),
        Some(&lexicon_corpus),
    )
    .context("building the independent model graph")?;
    graph_b.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut graph_b);

    let mut graph_a_indexed = graph_a;
    graph_a_indexed.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut graph_a_indexed);

    let admit_start = Instant::now();
    atlas_graph_types::store::assert_answers_match(&graph_a_indexed, &graph_b);
    println!("atlas-graph-compile: ADMISSION passed (assert_answers_match, full graph) in {:?}", admit_start.elapsed());

    // The KJV sub-verse span table is folded into the kjv section's own table: there is no file.
    let red_letter_spans: std::collections::HashMap<String, Vec<(usize, usize)>> =
        atlas_graph::red_letter_spans::spans_by_dot_ref(&red_letter_corpus, &restored_verses).into_iter().collect();
    println!("atlas-graph-compile: {} verses carry a sub-verse red-letter span", red_letter_spans.len());

    println!("atlas-graph-compile: building C2/C3 map-system exports (gazetteer + chronology)...");
    let gazetteer_places = atlas_graph::exports::gazetteer_places(&graph_a_indexed);
    let chronology_events = atlas_graph::exports::chronology_events(&graph_a_indexed, &chronology);
    let chronology_spans = atlas_graph::exports::chronology_spans(&graph_a_indexed);
    let chronology_anchor_rows = atlas_graph::exports::chronology_anchors(&graph_a_indexed, &atlas.chronology_anchors);
    let kretzmann_rows = atlas_graph::exports::kretzmann_date_rows(&graph_a_indexed);

    // The non-graph section tables ride BOTH graphs into the root -- one publishes the version the exports
    // stamp, the other is what the sections are written from -- so the artifact's version, the manifest's
    // root and the server's are one number. Read from the same files the server reads, never from memory.
    let sources_path = data_dir.join("sources.json");
    let sources: atlas_core::sources::SourcesDocument = serde_json::from_str(
        &std::fs::read_to_string(&sources_path).with_context(|| format!("reading {} (run `cargo run -p atlas-etl --bin gen_sources` from server/ first)", sources_path.display()))?,
    )
    .with_context(|| format!("parsing {}", sources_path.display()))?;
    let extras = atlas_graph::sqlite::extras::compute(&graph_a_indexed, &chronology.chrono, &red_letter_spans, &atlas, &sources, &lexicon_corpus.tokens)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("folding the sidecars and projections (DB-4b/DB-5)")?;
    extras.attach(&mut graph_a_indexed);
    extras.attach(&mut graph_b);
    println!(
        "atlas-graph-compile: DB-4b -- {} extra tables attached ({} rows)",
        extras.tables.len(),
        extras.tables.iter().map(|t| t.rows.len()).sum::<usize>()
    );
    let mut version_store = MemStore::default();
    let graph_version = version_store.publish(graph_a_indexed);
    let version_hex = atlas_graph::version_hex(graph_version);

    let gazetteer_export = atlas_graph::exports::GazetteerExport {
        format_version: atlas_graph::exports::GAZETTEER_FORMAT_VERSION,
        atlas_version_root: version_hex.clone(),
        places: gazetteer_places,
    };
    let chronology_export = atlas_graph::exports::ChronologyExport {
        format_version: atlas_graph::exports::CHRONOLOGY_FORMAT_VERSION,
        atlas_version_root: version_hex.clone(),
        events: chronology_events,
        spans: chronology_spans,
        anchors: chronology_anchor_rows,
    };

    std::fs::create_dir_all(&exports_dir).with_context(|| format!("creating {}", exports_dir.display()))?;
    let gazetteer_json = serde_json::to_string_pretty(&gazetteer_export).map_err(|e| anyhow::anyhow!("{e}")).context("serializing gazetteer.json")?;
    let chronology_json = serde_json::to_string_pretty(&chronology_export).map_err(|e| anyhow::anyhow!("{e}")).context("serializing chronology.json")?;
    let gazetteer_path = exports_dir.join("gazetteer.json");
    let chronology_path = exports_dir.join("chronology.json");
    std::fs::write(&gazetteer_path, format!("{gazetteer_json}\n")).with_context(|| format!("writing {}", gazetteer_path.display()))?;
    std::fs::write(&chronology_path, format!("{chronology_json}\n")).with_context(|| format!("writing {}", chronology_path.display()))?;
    println!(
        "atlas-graph-compile: wrote {} ({} places) and {} ({} events, {} spans, {} anchors) -- atlas_version_root={}",
        gazetteer_path.display(),
        gazetteer_export.places.len(),
        chronology_path.display(),
        chronology_export.events.len(),
        chronology_export.spans.len(),
        chronology_export.anchors.len(),
        version_hex
    );

    let kretzmann_export = atlas_graph::exports::KretzmannChronologyExport {
        format_version: atlas_graph::exports::KRETZMANN_CHRONOLOGY_FORMAT_VERSION,
        atlas_version_root: version_hex.clone(),
        status: "tentative-extraction".to_string(),
        rows: kretzmann_rows,
    };
    let kretzmann_json = serde_json::to_string_pretty(&kretzmann_export).map_err(|e| anyhow::anyhow!("{e}")).context("serializing kretzmann-chronology.json")?;
    let kretzmann_path = exports_dir.join("kretzmann-chronology.json");
    std::fs::write(&kretzmann_path, format!("{kretzmann_json}\n")).with_context(|| format!("writing {}", kretzmann_path.display()))?;
    println!("atlas-graph-compile: wrote {} ({} tentative date rows)", kretzmann_path.display(), kretzmann_export.rows.len());

    // The sections are written LAST, after the artifact, the spans and every export are on disk, so a
    // section failure never leaves the served artifact unwritten.
    let layout = match sections_cache {
        Some(cache) => atlas_graph::sqlite::source::SectionLayout { compiled_dir: data_dir.clone(), cache_dir: cache },
        None => atlas_graph::sqlite::source::SectionLayout::under(&data_dir),
    };
    println!(
        "atlas-graph-compile: DB-4b -- writing SQLite sections to {} (cache {}) ...",
        layout.sections_dir().display(),
        layout.cache_dir.display()
    );
    let t = Instant::now();
    let compiler = format!("atlas-graph-compile {} (rustc {})", env!("CARGO_PKG_VERSION"), "1.97.1");
    let (manifest, written) = atlas_graph::sqlite::writer::write_sections(&graph_b, &extras, &compiler, &layout)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("writing the SQLite sections")?;
    for w in &written {
        println!(
            "atlas-graph-compile:   {:<10} {:>7} nodes {:>8} rows {:>7} extra {:>8} edges {:>11} -> {:>10} bytes ({})  logical {}  ({:?})",
            w.section.name(),
            w.node_count,
            w.row_count,
            w.extra_row_count,
            w.edge_count,
            w.uncompressed_bytes,
            w.bytes,
            if w.reused_blob { "blob reused" } else { "zstd-19" },
            w.logical,
            w.elapsed
        );
    }
    println!("atlas-graph-compile: DB-4b -- sections written in {:?}; manifest root {}", t.elapsed(), manifest.root);
    // The root-equality proof on the real graph, at compile time: the manifest's root is the version the
    // exports carry and the server publishes.
    anyhow::ensure!(
        manifest.root == version_hex,
        "DB-4b: manifest root {} != the published version {} -- the sections and the in-memory graph disagree",
        manifest.root,
        version_hex
    );
    println!("atlas-graph-compile: DB-4b ADMISSION -- SqliteSnapshot (through CommittedZstdSource) vs the model graph ...");
    let t = Instant::now();
    let snap = atlas_graph::sqlite::snapshot::SqliteSnapshot::open_with_workers(
        &layout.manifest_path(),
        &atlas_graph::sqlite::source::CommittedZstdSource { layout: layout.clone() },
        atlas_graph::sqlite::snapshot::SqliteSnapshot::admission_workers(),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
    .context("opening the written sections")?;
    for w in &written {
        let conn = atlas_graph::sqlite::open_read_only(&w.path).map_err(|e| anyhow::anyhow!("{e}"))?;
        let recomputed = atlas_graph::sqlite::logical::logical_hash(
            &atlas_graph::sqlite::logical::logical_dump_of_db(&conn, w.section).map_err(|e| anyhow::anyhow!("{e}"))?,
        );
        anyhow::ensure!(
            recomputed == w.logical,
            "DB-4b: {} logical hash from tables {} != from partition {}",
            w.section.name(),
            recomputed,
            w.logical
        );
    }
    atlas_graph_types::store::assert_answers_match(&snap, &graph_b);
    println!(
        "atlas-graph-compile: DB-4b ADMISSION passed (assert_answers_match over SqliteSnapshot + per-section logical hashes + root equality) in {:?}",
        t.elapsed()
    );

    Ok(())
}
