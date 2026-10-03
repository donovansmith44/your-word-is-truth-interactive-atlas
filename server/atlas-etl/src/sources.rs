use anyhow::{anyhow, bail, Context, Result};
use atlas_core::sources::{ProvenanceEntry, ProvenanceTitles, SourceCategory, SourceEntry, SourcesDocument};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
struct SourcesFile {
    category: Vec<SourceCategory>,
    source: Vec<SourceEntry>,
    /// `#[serde(default)]` mirrors the compiled shape's own default exactly, so this parser still reads a
    /// sources file written before the join table existed.
    #[serde(default)]
    provenance: Vec<ProvenanceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedSources(SourcesDocument);

pub fn admit_sources(input: &str) -> Result<AdmittedSources> {
    let doc = parse_sources(input)?;
    validate_structure(&doc)?;
    Ok(AdmittedSources(doc))
}

impl AdmittedSources {
    pub fn document(&self) -> &SourcesDocument {
        &self.0
    }

    pub fn into_document(self) -> SourcesDocument {
        self.0
    }

    pub fn provenance_titles(&self) -> ProvenanceTitles {
        self.0
            .provenances
            .iter()
            .filter_map(|p| self.0.sources.iter().find(|s| s.id == p.source).map(|source| (p.id.clone(), source.title.clone())))
            .collect()
    }
}

/// Purely structural: the TOML shape and field types only. The cross-reference checks and the LICENSES.md
/// reconciliation are separate functions, so a caller runs exactly the checks it needs.
fn parse_sources(input: &str) -> Result<SourcesDocument> {
    let f: SourcesFile = toml::from_str(input)
        .context("sources.toml: invalid TOML or does not match the [[category]]/[[source]]/[[provenance]] schema")?;
    Ok(SourcesDocument { categories: f.category, sources: f.source, provenances: f.provenance })
}

/// No duplicate ids, every source's `category` names a declared category, and no source carries an empty
/// `licenses_row_key` -- an empty key would trivially and silently match nothing in the reconciliation.
fn validate_structure(doc: &SourcesDocument) -> Result<()> {
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

    // The same checks over the provenance join table: a duplicate id would make resolution ambiguous and an
    // unknown `source` would resolve to nothing at the UI. `confidence` is a closed vocabulary, refused by
    // the curated file's own parse.
    let mut prov_ids: HashSet<&str> = HashSet::new();
    for p in &doc.provenances {
        if !prov_ids.insert(p.id.as_str()) {
            errors.push(format!("duplicate provenance id '{}'", p.id));
        }
        if !source_ids.contains(p.source.as_str()) {
            errors.push(format!("provenance '{}' names undeclared source '{}'", p.id, p.source));
        }
        if p.locator.as_deref().is_some_and(|l| l.trim().is_empty()) {
            errors.push(format!("provenance '{}' has an empty locator -- omit the key instead", p.id));
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    bail!("sources.toml structural validation failed with {} error(s):\n{}", errors.len(), errors.join("\n"));
}

/// The Source-column text of every data row of LICENSES.md's per-source table, taken between that heading
/// and the next `## ` heading, skipping the header row and the separator.
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

/// Every LICENSES.md row must match EXACTLY ONE sources entry, by `licenses_row_key` as a literal
/// substring of that row's Source text, and vice versa: a row with no entry, an entry with no row, or an
/// ambiguous key all fail loud, naming the offender.
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
            "LICENSES.md's per-source table has {} row(s) with no matching data/curated/sources.toml entry -- a source was added to LICENSES.md but not to the Sources page's data (batch-s-brief.md requirement 3, fail loud); a bundled client asset (a typeface, an icon) is not an ingested source and is listed under '## Bundled client assets', which this law does not read:\n{}",
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
