//! Formats the ETL coverage/health report. Pure (`Report` in, `String`
//! out) so it's unit-testable without touching disk; `main.rs` writes the
//! returned text to both stdout and `data/compiled/report.txt`.

use std::fmt::Write as _;

use crate::borders::SnapshotStats;

#[derive(Debug, Clone, Default)]
pub struct Counts {
    pub canon_books: usize,
    pub places: usize,
    pub events: usize,
    pub narratives: usize,
    pub eras: usize,
    pub books_meta: usize,
    pub verses: usize,
    /// Number of distinct `From` verses that have at least one surviving
    /// cross-reference (i.e. `cross_refs.len()`, the compiled map's key count).
    pub cross_ref_sources: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub counts: Counts,
    /// % of raw Theographic event records that got a usable date.
    pub pct_events_dated: f64,
    /// % of compiled KJV verses reachable from >=1 geocoded place's `verse_links`.
    pub pct_verses_geocoded: f64,
    pub narrative_leg_counts: Vec<(String, usize)>,
    /// One line per base slug that collided, e.g. `"antioch -> antioch, antioch-2"`.
    pub slug_collisions: Vec<String>,
    pub warnings: Vec<String>,
    pub xref_dropped_unparseable: usize,
    pub xref_dropped_self: usize,
    pub xref_dropped_missing_first_verse: usize,
    /// Per-snapshot border stats, sorted by year (see `borders::process_snapshot`).
    pub border_snapshots: Vec<SnapshotStats>,
    pub landmarks_count: usize,
}

fn write_list(s: &mut String, lines: &[String]) {
    if lines.is_empty() {
        writeln!(s, "  (none)").unwrap();
        return;
    }
    for line in lines {
        writeln!(s, "  {line}").unwrap();
    }
}

pub fn write(r: &Report) -> String {
    let mut s = String::new();

    writeln!(s, "Bible Atlas ETL report").unwrap();
    writeln!(s, "======================").unwrap();
    writeln!(s).unwrap();

    writeln!(s, "Compiled file counts:").unwrap();
    writeln!(s, "  canon.json        {} books", r.counts.canon_books).unwrap();
    writeln!(s, "  places.json       {} places", r.counts.places).unwrap();
    writeln!(s, "  events.json       {} events", r.counts.events).unwrap();
    writeln!(s, "  narratives.json   {} narratives", r.counts.narratives).unwrap();
    writeln!(s, "  eras.json         {} eras", r.counts.eras).unwrap();
    writeln!(s, "  books-meta.json   {} rows", r.counts.books_meta).unwrap();
    writeln!(s, "  verses-kjv.json   {} verses", r.counts.verses).unwrap();
    writeln!(s, "  cross-refs.json   {} source verses", r.counts.cross_ref_sources).unwrap();
    writeln!(s).unwrap();

    writeln!(s, "Coverage:").unwrap();
    writeln!(s, "  {:.1}% of Theographic events have a usable date", r.pct_events_dated).unwrap();
    writeln!(s, "  {:.1}% of KJV verses have >=1 geocoded place", r.pct_verses_geocoded).unwrap();
    writeln!(s).unwrap();

    writeln!(s, "Narrative leg counts:").unwrap();
    let leg_lines: Vec<String> = r.narrative_leg_counts.iter().map(|(id, n)| format!("{id}: {n} legs")).collect();
    write_list(&mut s, &leg_lines);
    writeln!(s).unwrap();

    writeln!(s, "Geo slug collisions:").unwrap();
    write_list(&mut s, &r.slug_collisions);
    writeln!(s).unwrap();

    writeln!(s, "Cross-reference drops:").unwrap();
    writeln!(s, "  {} unparseable rows", r.xref_dropped_unparseable).unwrap();
    writeln!(s, "  {} self-references", r.xref_dropped_self).unwrap();
    writeln!(s, "  {} targets whose first verse is missing from the compiled KJV text", r.xref_dropped_missing_first_verse).unwrap();
    writeln!(s).unwrap();

    writeln!(s, "Warnings:").unwrap();
    write_list(&mut s, &r.warnings);
    writeln!(s).unwrap();

    writeln!(s, "Border snapshots ({} years):", r.border_snapshots.len()).unwrap();
    let mut points_before = 0usize;
    let mut points_after = 0usize;
    for snap in &r.border_snapshots {
        writeln!(
            s,
            "  {}: {} features kept (of {} in the raw snapshot), {} -> {} points",
            snap.year, snap.features_kept, snap.features_in, snap.points_before_simplify, snap.points_after_simplify
        )
        .unwrap();
        points_before += snap.points_before_simplify;
        points_after += snap.points_after_simplify;
    }
    let point_reduction_pct = if points_before == 0 { 0.0 } else { 100.0 * (1.0 - points_after as f64 / points_before as f64) };
    writeln!(s, "  total point reduction from simplification: {point_reduction_pct:.1}% ({points_before} -> {points_after})")
        .unwrap();
    writeln!(s).unwrap();

    writeln!(s, "Landmarks:").unwrap();
    writeln!(s, "  {} curated landmarks compiled", r.landmarks_count).unwrap();

    s
}
