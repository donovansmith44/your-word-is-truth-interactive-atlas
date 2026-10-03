use atlas_graph_types::canon::Value;
use atlas_graph_types::sections::extra_line_body;
use rusqlite::Connection;

#[derive(Clone, Debug, PartialEq)]
pub enum Col {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
}

/// A table's name, its columns in DDL order, and its primary key in key order.
pub struct TableSpec {
    pub name: &'static str,
    pub columns: &'static [&'static str],
    pub pk: &'static [&'static str],
}


#[derive(Debug)]
pub struct SqliteError(pub String);

impl std::fmt::Display for SqliteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SqliteError {}
impl From<rusqlite::Error> for SqliteError {
    fn from(e: rusqlite::Error) -> Self {
        SqliteError(format!("sqlite: {e}"))
    }
}
impl From<atlas_graph_types::canon::CanonError> for SqliteError {
    fn from(e: atlas_graph_types::canon::CanonError) -> Self {
        SqliteError(format!("canon: {e}"))
    }
}

pub static PROVENANCE_TITLE: TableSpec = TableSpec { name: "provenance_title", columns: &["id", "title"], pk: &["id"] };

fn col_value(c: &Col) -> Result<Value, SqliteError> {
    Ok(match c {
        Col::Null => Value::Null,
        Col::Int(i) => Value::Int(*i),
        Col::Real(f) => Value::float(*f)?,
        Col::Text(s) => Value::Str(s.clone()),
    })
}

pub fn row_body(spec: &TableSpec, row: &[Col]) -> Result<Vec<u8>, SqliteError> {
    if row.len() != spec.columns.len() {
        return Err(SqliteError(format!("{}: row has {} cols, spec has {}", spec.name, row.len(), spec.columns.len())));
    }
    let mut cols = Vec::with_capacity(row.len());
    for (name, c) in spec.columns.iter().zip(row) {
        cols.push((*name, col_value(c)?));
    }
    Ok(extra_line_body(cols))
}


pub fn read_table(conn: &Connection, spec: &TableSpec) -> Result<Vec<Vec<Col>>, SqliteError> {
    let sql = format!("SELECT {} FROM {} ORDER BY {}", spec.columns.join(", "), spec.name, spec.pk.join(", "));
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let mut cols = Vec::with_capacity(spec.columns.len());
        for i in 0..spec.columns.len() {
            cols.push(match r.get_ref(i)? {
                rusqlite::types::ValueRef::Null => Col::Null,
                rusqlite::types::ValueRef::Integer(i) => Col::Int(i),
                rusqlite::types::ValueRef::Real(f) => Col::Real(f),
                rusqlite::types::ValueRef::Text(t) => {
                    Col::Text(String::from_utf8(t.to_vec()).map_err(|e| SqliteError(format!("{}: {e}", spec.name)))?)
                }
                rusqlite::types::ValueRef::Blob(_) => return Err(SqliteError(format!("{}: BLOB in an extra table", spec.name))),
            });
        }
        out.push(cols);
    }
    Ok(out)
}
