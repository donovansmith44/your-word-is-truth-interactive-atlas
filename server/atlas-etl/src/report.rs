//! Formats the ETL coverage/health report. Pure -- `Report` in, `String` out -- so it is testable without
//! touching disk.

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
    /// Distinct `From` verses with at least one surviving cross-reference.
    pub cross_ref_sources: usize,
    /// Theographic person records compiled into memory: graph-only from birth, never a compiled file.
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
    pub polities: Vec<PolityStats>,
    pub landmarks_count: usize,
    /// The curated land mask's coverage: how many named regions, how many rings across them -- a region
    /// may carry more than one -- and the total point count.
    pub land_mask_regions: usize,
    pub land_mask_rings: usize,
    pub land_mask_points: usize,
    pub catechism_parts: usize,
    pub catechism_items: usize,
    /// How many items are reachable from at least one verse: an item counts if EITHER its own item-level
    /// verses OR any of its questions' verses is non-empty.
    pub catechism_items_reachable: usize,
    /// Distinct verse ids linking into the catechism from EITHER citation source.
    pub catechism_distinct_verses: usize,
    /// Per-part reachability, `(part title, reachable items, total items)`, in the curated part order.
    pub catechism_per_part: Vec<(String, usize, usize)>,
}

/// Per-polity report line: how many eras it carries, and the total point count across every ring of every
/// one of those eras.
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
    writeln!(s, "  places (graph-only, places.json retired at M-C2)      {}", r.counts.places).unwrap();
    writeln!(s, "  events (graph-only, events.json retired at M-C2)      {}", r.counts.events).unwrap();
    writeln!(s, "  narratives (graph-only, narratives.json retired at M-C2) {}", r.counts.narratives).unwrap();
    writeln!(s, "  eras (curated)    {} eras (graph-only; eras.json retired at M-C)", r.counts.eras).unwrap();
    writeln!(s, "  books-meta.json   {} rows", r.counts.books_meta).unwrap();
    writeln!(s, "  verses (graph-only, verses-kjv.json retired at M-C2)   {}", r.counts.verses).unwrap();
    writeln!(s, "  cross-refs (graph-only, cross-refs.json retired at M-C2) {} source verses", r.counts.cross_ref_sources).unwrap();
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
