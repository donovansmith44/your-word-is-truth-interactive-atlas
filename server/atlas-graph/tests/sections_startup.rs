mod common;

use std::time::{Duration, Instant};

use atlas_graph::service::GraphService;

const STARTUP_CEILING: Duration = Duration::from_secs(4);

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn the_served_path_starts_under_the_ceiling() {
    let compiled = common::compiled_dir();
    let _ = GraphService::from_sections(&compiled).expect("the committed sections open");
    let t0 = Instant::now();
    let (graph, data, sources) = GraphService::from_sections(&compiled).expect("the committed sections open");
    let t_open = t0.elapsed();
    let t1 = Instant::now();
    let data = data.finish();
    let t_finish = t1.elapsed();
    let t2 = Instant::now();
    graph.scene_source(&data);
    let t_scene = t2.elapsed();
    let total = t0.elapsed();
    println!(
        "DB-4c STARTUP: from_sections {t_open:?}, finish {t_finish:?}, scene priming {t_scene:?}, total {total:?} (ceiling {STARTUP_CEILING:?}); {} sources, {} events",
        sources.sources.len(),
        graph.event_world_stats.events
    );
    assert!(graph.absent_sections().is_empty(), "every shipped section is present in data/compiled");
    assert!(total <= STARTUP_CEILING, "served startup took {total:?}, over the {STARTUP_CEILING:?} ceiling");
}
