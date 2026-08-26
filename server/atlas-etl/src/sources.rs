//! Parses `data/curated/sources.toml` (the Sources page's own curated
//! single source of truth, batch-s-brief.md requirement 3) and
//! cross-validates it 1:1 against LICENSES.md's own "## Per-source
//! table" -- fail-loud, both from the `gen_sources` binary (regenerating
//! `data/compiled/sources.json`) and from `tests/sources_validate.rs`, so
//! `cargo test --workspace` alone already catches drift, with no separate
//! step to remember to run.
//!
//! Deliberately its OWN module, never wired into [`crate::compile::
//! compile`]: this data has nothing to do with the Explorable Graph, and
//! the batch's own finalization block requires `graph.bin`/
//! `data/exports/` stay byte-untouched by anything this batch adds.

use anyhow::{anyhow, bail, Context, Result};
use atlas_core::sources::{SourceCategory, SourceEntry, SourcesDocument};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
struct SourcesFile {
    category: Vec<SourceCategory>,
    source: Vec<SourceEntry>,
}

/// Parses `data/curated/sources.toml`'s `[[category]]`/`[[source]]`
/// arrays. Purely structural (TOML shape + field types) -- see
/// [`validate_structure`] for the cross-reference checks (unique ids,
/// every source's own category is declared) and
/// [`validate_against_licenses`] for the LICENSES.md reconciliation;
/// kept separate so a caller that only needs the parsed data can run
/// exactly the checks it needs, the same discipline `curated::parse_*`
/// uses throughout this crate.
pub fn parse_sources(input: &str) -> Result<SourcesDocument> {
    let f: SourcesFile =
        toml::from_str(input).context("sources.toml: invalid TOML or does not match the [[category]]/[[source]] schema")?;
    Ok(SourcesDocument { categories: f.category, sources: f.source })
}

/// Structural cross-checks: no duplicate category/source ids, every
/// source's own `category` names a real declared category, and no
/// source carries an empty `licenses_row_key` (which would trivially,
/// silently match nothing in [`validate_against_licenses`]) -- the same
/// "declared, not merely inferred" discipline `validate::run_*` uses
/// throughout this crate for curated data.
pub fn validate_structure(doc: &SourcesDocument) -> Result<()> {
    let mut errors = Vec::new();

    let mut cat_ids: HashSet<&str> = HashSet::new();
    for c in &doc.categories {
        if !cat_ids.insert(c.id.as_str()) {
            errors.push(format!("duplicate category id '{}'", c.id));
        }
    }

    let mut source_ids: HashSet<&str> = HashSet::new();
    for s in &doc.sources {
        if !source_ids.insert(s.id.as_str()) {
            errors.push(format!("duplicate source id '{}'", s.id));
        }
        if !cat_ids.contains(s.category.as_str()) {
            errors.push(format!("source '{}' names undeclared category '{}'", s.id, s.category));
        }
        if s.licenses_row_key.trim().is_empty() {
            errors.push(format!("source '{}' has an empty licenses_row_key", s.id));
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    bail!("sources.toml structural validation failed with {} error(s):\n{}", errors.len(), errors.join("\n"));
}

/// Extracts the Source-column text of every data row in LICENSES.md's own
/// "## Per-source table" (between that heading and the next `## `
/// heading), skipping the header row and the `|---|---|---|` separator.
fn extract_per_source_table_rows(licenses_md: &str) -> Result<Vec<String>> {
    let heading = "## Per-source table";
    let start = licenses_md
        .find(heading)
        .ok_or_else(|| anyhow!("LICENSES.md has no '## Per-source table' heading -- did its structure change?"))?;
    let after = &licenses_md[start + heading.len()..];
    let end = after.find("\n## ").unwrap_or(after.len());
    let section = &after[..end];

    let mut rows = Vec::new();
    let mut data_rows_seen = 0usize;
    for line in section.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        data_rows_seen += 1;
        if data_rows_seen <= 2 {
            // row 1 = the header ("| Source | License | How it's used |"),
            // row 2 = the "|---|---|---|" separator -- neither is data.
            continue;
        }
        let first_cell = line.trim_start_matches('|').split('|').next().unwrap_or("").trim().to_string();
        if !first_cell.is_empty() {
            rows.push(first_cell);
        }
    }
    if rows.is_empty() {
        bail!("LICENSES.md's '## Per-source table' section parsed to zero data rows -- a parsing bug, or the table's own structure changed");
    }
    Ok(rows)
}

/// The fail-loud drift check batch-s-brief.md requirement 3 asks for:
/// every LICENSES.md per-source-table row must match EXACTLY ONE
/// `sources.toml` entry (by `licenses_row_key`, a literal substring of
/// the row's own Source-column text) and vice versa -- a LICENSES.md row
/// with no page entry, a page entry with no LICENSES.md row, or an
/// ambiguous (non-unique) key all fail loud, naming the offending id/row.
pub fn validate_against_licenses(doc: &SourcesDocument, licenses_md: &str) -> Result<()> {
    let rows = extract_per_source_table_rows(licenses_md)?;

    let mut matched_row_for_source: Vec<Option<usize>> = vec![None; doc.sources.len()];
    let mut unmatched_rows: Vec<String> = Vec::new();

    for (ri, row) in rows.iter().enumerate() {
        let matches: Vec<usize> =
            doc.sources.iter().enumerate().filter(|(_, s)| row.contains(s.licenses_row_key.as_str())).map(|(si, _)| si).collect();
        match matches.as_slice() {
            [] => unmatched_rows.push(row.clone()),
            [only] => {
                if let Some(prev_ri) = matched_row_for_source[*only] {
                    bail!(
                        "sources.toml entry '{}' matches TWO LICENSES.md per-source-table rows ('{}' and '{}') -- its licenses_row_key ('{}') is not unique enough",
                        doc.sources[*only].id,
                        rows[prev_ri],
                        row,
                        doc.sources[*only].licenses_row_key
                    );
                }
                matched_row_for_source[*only] = Some(ri);
            }
            many => bail!(
                "LICENSES.md per-source-table row '{}' matches MULTIPLE sources.toml entries ({:?}) -- ambiguous licenses_row_key",
                row,
                many.iter().map(|i| doc.sources[*i].id.as_str()).collect::<Vec<_>>()
            ),
        }
    }

    if !unmatched_rows.is_empty() {
        bail!(
            "LICENSES.md's per-source table has {} row(s) with no matching data/curated/sources.toml entry -- a source was added to LICENSES.md but not to the Sources page's data (batch-s-brief.md requirement 3, fail loud):\n{}",
            unmatched_rows.len(),
            unmatched_rows.iter().map(|r| format!("  - {r}")).collect::<Vec<_>>().join("\n")
        );
    }

    let missing: Vec<&str> =
        doc.sources.iter().zip(&matched_row_for_source).filter(|(_, m)| m.is_none()).map(|(s, _)| s.id.as_str()).collect();
    if !missing.is_empty() {
        bail!(
            "data/curated/sources.toml has {} entry(ies) with no matching LICENSES.md per-source-table row -- citation integrity requires every Sources-page entry be backed by a LICENSES.md row: {:?}",
            missing.len(),
            missing
        );
    }

    Ok(())
}
