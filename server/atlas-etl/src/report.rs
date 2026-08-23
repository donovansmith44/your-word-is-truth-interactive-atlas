//! Formats the ETL coverage/health report. Pure (`Report` in, `String`
//! out) so it's unit-testable without touching disk; `main.rs` writes the
//! returned text to both stdout and `data/compiled/report.txt`.

use std::fmt::Write as _;

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
    /// Batch P (the extensibility proof): Theographic person records
    /// compiled onto `AtlasData.people` -- graph-only from birth, no
    /// compiled JSON file at any point (see `AtlasData.people`'s own doc
    /// comment).
    pub people: usize,
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
    /// Batch B2: one line per curated polity, sorted by id (see
    /// `PolityStats`).
    pub polities: Vec<PolityStats>,
    pub landmarks_count: usize,
    /// Batch R requirement 1 ("borders become part of the plate"): the
    /// curated land mask's own coverage figures -- how many named regions,
    /// how many rings across them (a region may itself carry >1 ring, though
    /// none do yet), and the total point count (the same "how much
    /// hand-authoring" figure `PolityStats::points` already reports for
    /// polities).
    pub land_mask_regions: usize,
    pub land_mask_rings: usize,
    pub land_mask_points: usize,
    /// Batch F ("the small catechism"): how many chief parts and total
    /// items compiled from `data/curated/catechism.toml` -- the same coarse
    /// "how much curated content is there" figure `landmarks_count`/
    /// `land_mask_regions` already report for their own curated files.
    pub catechism_parts: usize,
    pub catechism_items: usize,
    /// Batch F2 (requirement 5, "report the before/after"): how many of
    /// `catechism_items` are reachable from >=1 verse today -- an item
    /// counts if EITHER its own item-level `verses` (Luther's embedded
    /// citations, Batch F) OR any of its `questions[].verses` (the repo
    /// mapping / Deut5 supplement, Batch F2) is non-empty. Was 6 (of 33)
    /// before this batch.
    pub catechism_items_reachable: usize,
    /// Batch F2: distinct verse ids that link into the catechism from
    /// EITHER citation source (the compiled `verse_to_catechism` index's
    /// own key count).
    pub catechism_distinct_verses: usize,
    /// Batch F2: per-part reachability, `(part title, reachable items,
    /// total items)`, in `catechism.toml`'s own part order.
    pub catechism_per_part: Vec<(String, usize, usize)>,
}

/// Batch B2: per-polity report line -- how many eras it carries, and the
/// total point count across every ring of every one of those eras (a coarse
/// "how much hand-authoring does this polity represent" figure, the spirit
/// of what the retired border-snapshot report's own point-reduction line
/// used to convey, now with nothing simplified away to reduce).
#[derive(Debug, Clone, Default)]
pub struct PolityStats {
    pub id: String,
    pub eras: usize,
    pub points: usize,
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
    // M-C2 deletion event: places.json/events.json/narratives.json/
    // verses-kjv.json/cross-refs.json retire (their data lives only on
    // the graph, GraphStore-served -- see batch-mc2-report.md's own
    // deletion inventory), the SAME "count stays real and worth
    // reporting, filename claim retires" treatment M-C's own eras.json
    // retirement already established just below.
    writeln!(s, "  places (graph-only, places.json retired at M-C2)      {}", r.counts.places).unwrap();
    writeln!(s, "  events (graph-only, events.json retired at M-C2)      {}", r.counts.events).unwrap();
    writeln!(s, "  narratives (graph-only, narratives.json retired at M-C2) {}", r.counts.narratives).unwrap();
    writeln!(s, "  eras (curated)    {} eras (graph-only; eras.json retired at M-C)", r.counts.eras).unwrap();
    writeln!(s, "  books-meta.json   {} rows", r.counts.books_meta).unwrap();
    writeln!(s, "  verses (graph-only, verses-kjv.json retired at M-C2)   {}", r.counts.verses).unwrap();
    writeln!(s, "  cross-refs (graph-only, cross-refs.json retired at M-C2) {} source verses", r.counts.cross_ref_sources).unwrap();
    // Batch P: graph-only from the start, never a compiled JSON file (see
    // AtlasData.people's own doc comment) -- reported the same "count stays
    // real and worth reporting" way as the M-C2-retired fields just above,
    // minus the "retired" framing (nothing here was ever a standalone file).
    writeln!(s, "  people (graph-only, no compiled file -- Batch P)       {}", r.counts.people).unwrap();
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

    let total_eras: usize = r.polities.iter().map(|p| p.eras).sum();
    let total_points: usize = r.polities.iter().map(|p| p.points).sum();
    writeln!(s, "Polities ({} polities, {total_eras} eras, {total_points} points total):", r.polities.len()).unwrap();
    for p in &r.polities {
        writeln!(s, "  {}: {} era(s), {} points", p.id, p.eras, p.points).unwrap();
    }
    writeln!(s).unwrap();

    writeln!(s, "Landmarks:").unwrap();
    writeln!(s, "  {} curated landmarks compiled", r.landmarks_count).unwrap();
    writeln!(s).unwrap();

    writeln!(s, "Land mask (Batch R requirement 1):").unwrap();
    writeln!(
        s,
        "  {} region(s), {} ring(s), {} points total",
        r.land_mask_regions, r.land_mask_rings, r.land_mask_points
    )
    .unwrap();
    writeln!(s).unwrap();

    writeln!(s, "Catechism (Batch F, extended Batch F2):").unwrap();
    writeln!(s, "  {} chief part(s), {} item(s) total", r.catechism_parts, r.catechism_items).unwrap();
    writeln!(
        s,
        "  {}/{} items reachable from >=1 verse (was 6/33 before Batch F2's own repo mapping + Deut5 supplement)",
        r.catechism_items_reachable, r.catechism_items
    )
    .unwrap();
    writeln!(s, "  {} distinct verse(s) link into the catechism", r.catechism_distinct_verses).unwrap();
    for (title, reachable, total) in &r.catechism_per_part {
        writeln!(s, "    {title}: {reachable}/{total} reachable").unwrap();
    }

    s
}
