mod sections {
    pub use atlas_graph_types::sections::*;
    #[path = "/home/donovan/w/A-F39-review/docs/superpowers/reports/evidence/2026-10-03-f39-rereview/exact-row-line-body.rs"]
    mod exact;
    pub use exact::row_line_body;
}
mod sqlite {
    pub use atlas_graph::sqlite::{extras, hash_bytes, hash_from_bytes, partition, rows, SqliteError};
    #[path = "/home/donovan/w/A-F39-review/server/atlas-graph/src/sqlite/logical.rs"]
    pub mod logical;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use atlas_graph_types::sections::Section;
    use sqlite::logical::{logical_dump_of_db, logical_hash};
    let args:Vec<String>=std::env::args().skip(1).collect();
    let section=Section::MANIFEST_ORDER.iter().find(|s|s.name()==args[0]).copied().ok_or("unknown section")?;
    let conn=rusqlite::Connection::open_with_flags(&args[1],rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    println!("{}",logical_hash(&logical_dump_of_db(&conn,section)?));
    Ok(())
}
