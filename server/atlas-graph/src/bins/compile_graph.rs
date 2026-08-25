//! Batch M-C, controller decision 4: THE COMPILE STEP -- "a compile step
//! (extend atlas-etl's entry point or a new atlas-graph bin -- your call,
//! disclosed) produces it [the serialized artifact]." A NEW BIN here, not
//! an extension of atlas-etl's own binary: atlas-etl's `main.rs` already
//! owns the curated-TOML+Theographic parse/merge/validate pipeline that
//! produces `data/compiled/*.json` (this binary's own OWN INPUT, via
//! `AtlasData::load`); the graph-artifact compile step is a SEPARATE
//! concern layered on top (raw KJV/xrefs + curated eras.toml + the
//! already-compiled `AtlasData` -> one graph artifact file), which reads
//! more naturally as its own small tool than as a mode flag inside an
//! already-large ETL binary.
//!
//! ```text
//! atlas-graph-compile --data-dir data/compiled --out data/compiled/graph.bin
//! ```
//! (`--data-dir` names the SAME `data/compiled` every other tool in this
//! workspace uses as the anchor; `raw/`/`curated/` are its siblings under
//! the shared `data/` parent, exactly the derivation `atlas-server`'s own
//! `main.rs` and `GraphService::build` already use.)
//!
//! M-C2: this binary's own INPUT `AtlasData` now comes from
//! `atlas_etl::compile::compile(raw_dir, curated_dir)` (a real raw+curated
//! compile, the same orchestration `atlas-etl`'s own binary runs) rather
//! than `AtlasData::load(&data_dir)` reading `data/compiled/*.json` --
//! five of those files (places/events/narratives/verses-kjv/cross-refs)
//! are the M-C2 deletion event's own target and no longer exist.
//!
//! ADMISSION (design §9a: "implementation #2 passes the same law as #1"):
//! before writing anything, this binary independently re-builds the SAME
//! graph a second time from the identical sources and runs
//! `atlas_graph_types::store::assert_answers_match` between the dump's own
//! round-tripped reconstruction and that second build -- the artifact
//! this binary produces has PASSED the SAME conformance law
//! `tests/artifact_conformance.rs` proves in CI, not merely "should."
//! This is the expensive check (measured: ~3-15s over the full real
//! graph); it happens HERE, once, at compile time -- never at server
//! startup, which is exactly what keeps the LOAD-TIME ceiling (controller
//! decision 6) achievable.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use atlas_graph_types::store::{GraphPublisher, MemStore};

fn parse_args(args: &[String]) -> Result<(PathBuf, PathBuf)> {
    let mut data_dir: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
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
            other => anyhow::bail!("unrecognized argument: {other}"),
        }
        i += 1;
    }
    let data_dir = data_dir.context("--data-dir is required, e.g. --data-dir ../data/compiled")?;
    let out = out.context("--out is required, e.g. --out ../data/compiled/graph.bin")?;
    Ok((data_dir, out))
}

fn main() -> Result<()> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let (data_dir, out_path) = parse_args(&raw)?;

    let raw_dir = data_dir.parent().map(|p| p.join("raw")).unwrap_or_else(|| Path::new("../data/raw").to_path_buf());
    let curated_dir = data_dir.parent().map(|p| p.join("curated")).unwrap_or_else(|| Path::new("../data/curated").to_path_buf());
    // C2C3-EXPORT: `data/exports/` is a NEW committed directory (unlike
    // gitignored `data/raw`) -- the two cross-repo contract files
    // (gazetteer.json/chronology.json) land here, derived from `--data-dir`
    // the SAME way `raw_dir`/`curated_dir` already are.
    let exports_dir = data_dir.parent().map(|p| p.join("exports")).unwrap_or_else(|| Path::new("../data/exports").to_path_buf());

    // M-C2: `AtlasData::load(&data_dir)` retired as this binary's own
    // source -- the five files it read (places/events/narratives/
    // verses-kjv/cross-refs.json) are the deletion event's own target
    // (batch-mc2-report.md). `atlas_etl::compile::compile` is the SAME
    // raw+curated orchestration `atlas-etl`'s own binary runs, called here
    // directly instead of round-tripping through `data/compiled/*.json` --
    // it also already carries `data.eras` (compiled once, validated),
    // so the separate `eras.toml` re-parse this binary used to do on its
    // own retires too: one parse, one source, for both.
    let atlas = atlas_etl::compile::compile(&raw_dir, &curated_dir).with_context(|| format!("compiling {} + {}", raw_dir.display(), curated_dir.display()))?.data;
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).with_context(|| format!("reading {}", raw_dir.join("kjv.json").display()))?;
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt"))
        .with_context(|| format!("reading {}", raw_dir.join("xrefs/cross_references.txt").display()))?;
    let eras = atlas.eras.clone();

    // CORP-1a: the real compile step HARD-REQUIRES the vendored brain-fuel
    // data (unlike GraphService::build's own dev-fallback, which degrades
    // gracefully for a fixture raw_dir) -- a graph.bin compiled without it
    // would silently ship KJV-only text, defeating this batch's own
    // purpose. `data/raw/brain-fuel-bible/` (see data/raw/README.md).
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

    // Batch KJV-CASE: the Tetragrammaton LORD/Lord case-restoration pass
    // (owner ruling; batch-kjv-case-brief.md) actually runs INSIDE
    // `build_graph_from_sources_with_eras_and_brainfuel` below (see that
    // function's own doc comment) -- this is a SEPARATE, disclosed
    // recomputation purely for this startup log, reusing `atlas.verses`
    // (already in scope, unrestored, from the SAME `data/raw/kjv.json`
    // `kjv_json` above reads) so the operator sees the exact counts
    // without a third parse of that file. Batch KJV-CASE-2 (batch-kjv-
    // case2-brief.md) extended `restore_kjv_case` itself in place -- the
    // SAME call here now also reports the superscription-tail class.
    let (_, case_restoration) = atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &atlas.verses);
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

    // CORP-2a: the real compile step HARD-REQUIRES the vendored Concord
    // data + curated SC-overlap alignment (the SAME "graph.bin compiled
    // without it would silently ship an incomplete corpus" reasoning
    // CORP-1a's own brainfuel requirement above already established) --
    // unlike `GraphService::build`'s own dev fallback, which degrades
    // gracefully for a fixture `raw_dir` (`data/raw/concord/`, see
    // `data/raw/README.md`).
    let concord_root = raw_dir.join("concord");
    println!("atlas-graph-compile: reading vendored Concord (Book of Concord) data from {}...", concord_root.display());
    let concord_corpus = atlas_etl::concord::read_all(&concord_root).with_context(|| format!("reading {}", concord_root.display()))?;
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

    // KRETZ-1: the real compile step HARD-REQUIRES the vendored Kretzmann
    // data (the SAME "graph.bin compiled without it would silently ship an
    // incomplete corpus" reasoning CORP-1a/CORP-2a's own requirements above
    // already established) -- unlike `GraphService::build`'s own dev
    // fallback, which degrades gracefully for a fixture `raw_dir`
    // (`data/raw/kretzmann/`, see `data/raw/README.md`).
    let kretzmann_root = raw_dir.join("kretzmann");
    println!("atlas-graph-compile: reading vendored Kretzmann (Popular Commentary of the Bible) data from {}...", kretzmann_root.display());
    let kretzmann_corpus = atlas_etl::kretzmann::read_all(&kretzmann_root, &atlas.verses).with_context(|| format!("reading {}", kretzmann_root.display()))?;
    println!(
        "atlas-graph-compile: kretzmann {} pages, {} units, {} excised fragments, {} footnotes ({} disclosed structural anomalies)",
        kretzmann_corpus.stats.pages, kretzmann_corpus.stats.units, kretzmann_corpus.stats.fragments, kretzmann_corpus.stats.footnotes, kretzmann_corpus.stats.disclosures.len(),
    );

    println!("atlas-graph-compile: building implementation #1 (from raw sources)...");
    let build_start = Instant::now();
    let (graph_a, stats, event_world_stats, chrono) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(
        &kjv_json,
        &xrefs_tsv,
        &atlas,
        &eras,
        Some(&brainfuel),
        Some(&concord_bundle),
        Some(&kretzmann_corpus),
    )
    .context("building the graph from raw sources")?;
    println!(
        "atlas-graph-compile: {} text units, {} cites edges, {} events ({} dated), {} places, {} narratives, {} anchors -- build time {:?}",
        stats.kjv_verses, stats.cites_rows, event_world_stats.events, event_world_stats.dated_events, event_world_stats.places, event_world_stats.narratives, event_world_stats.anchors,
        build_start.elapsed()
    );
    let chronology = atlas_graph::Chronology::from_derivation(chrono);

    println!("atlas-graph-compile: dumping to the artifact DTO form...");
    let dump = atlas_graph::artifact::dump(&graph_a, &chronology, &stats, &event_world_stats).map_err(|e| anyhow::anyhow!("{e}")).context("dumping the built graph")?;

    println!("atlas-graph-compile: ADMISSION -- rebuilding implementation #1 a second time (independent model)...");
    let (mut graph_b, ..) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(
        &kjv_json,
        &xrefs_tsv,
        &atlas,
        &eras,
        Some(&brainfuel),
        Some(&concord_bundle),
        Some(&kretzmann_corpus),
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

    // Also prove the DUMP ITSELF round-trips losslessly (not just that the
    // pre-dump graph was internally consistent) -- decode what will
    // actually be written and admit THAT against the model too.
    let bytes = atlas_graph::artifact::encode(&dump).map_err(|e| anyhow::anyhow!("{e}"))?;
    let redecoded = atlas_graph::artifact::decode(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
    let (mut reconstructed, ..) = atlas_graph::artifact::to_service_parts(redecoded).map_err(|e| anyhow::anyhow!("{e}"))?;
    reconstructed.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut reconstructed);
    atlas_graph_types::store::assert_answers_match(&reconstructed, &graph_b);
    println!("atlas-graph-compile: ADMISSION passed for the encoded bytes themselves (round-trip, not just the pre-dump graph)");

    std::fs::create_dir_all(out_path.parent().unwrap_or_else(|| Path::new("."))).ok();
    std::fs::write(&out_path, &bytes).with_context(|| format!("writing {}", out_path.display()))?;
    println!("atlas-graph-compile: wrote {} ({} bytes)", out_path.display(), bytes.len());

    // C2C3-EXPORT (map-system contracts C2/C3, .superpowers/sdd/
    // 2026-08-17-bible-atlas-m1/c2c3-export-design.md): a NEW TERMINAL PASS
    // in this SAME binary, after admission -- built from `graph_a_indexed`,
    // the same admitted graph `dump`/`bytes`/graph.bin just came from, so
    // `atlas_version_root` embeds the identical `GraphVersion` the artifact
    // reports (drift is impossible by construction). Row-building borrows
    // `graph_a_indexed`; only the FINAL version-stamp step below consumes
    // it by value (nothing downstream needs the graph itself afterward).
    println!("atlas-graph-compile: building C2/C3 map-system exports (gazetteer + chronology)...");
    let gazetteer_places = atlas_graph::exports::gazetteer_places(&graph_a_indexed);
    let chronology_events = atlas_graph::exports::chronology_events(&graph_a_indexed, &chronology);
    let chronology_spans = atlas_graph::exports::chronology_spans(&graph_a_indexed);
    let chronology_anchor_rows = atlas_graph::exports::chronology_anchors(&graph_a_indexed, &atlas.chronology_anchors);
    // KRETZ-1 (THE DATE MINE): the SAME terminal-pass treatment, riding the
    // SAME already-admitted `graph_a_indexed` -- drift-impossible by the
    // SAME construction the C2C3 exports above already rely on. Computed
    // here, before `graph_a_indexed` moves into `version_store.publish`
    // below (its own last use).
    let kretzmann_rows = atlas_graph::exports::kretzmann_date_rows(&graph_a_indexed);

    // The SAME version derivation `GraphService`'s own constructors use
    // (service.rs: `MemStore::default(); store.publish(graph)`) -- the only
    // public path to a `GraphVersion` from an owned `Graph` (`store::
    // version_of` is a graph-types-private fn; graph-types stays untouched,
    // per the EXTEND-ONLY rule). Consumes `graph_a_indexed`: its last use.
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

    // KRETZ-1 (THE DATE MINE, owner order 2026-08-24: "extract the years
    // from Kretzmann and throw them somewhere as our tentative source of
    // truth that gets shared everywhere") -- its own export file,
    // `status: "tentative-extraction"` per the scouting memo's own header
    // (CHRON-CONV-1 adjudicates real placements from it later; this file
    // carries no placement authority of its own).
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

    Ok(())
}
