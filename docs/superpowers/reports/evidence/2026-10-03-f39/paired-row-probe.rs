mod sections { pub use atlas_graph_types::sections::*; }
mod sqlite {
    pub use atlas_graph::sqlite::{extras, hash_bytes, hash_from_bytes, partition, rows, SqliteError};
    #[path = "/home/donovan/w/A-F39-review/server/atlas-graph/src/sqlite/logical.rs"]
    pub mod logical;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use atlas_graph_types::sections::Section;
    use sqlite::logical::{logical_dump_of_db, logical_hash};
    let mut passed = 0;
    for reassignment in 1..=32_i64 {
        let conn = rusqlite::Connection::open_in_memory()?;
        atlas_graph::sqlite::ddl::create_tables(&conn, Section::Core)?;
        conn.pragma_update(None,"journal_mode","MEMORY")?;
        conn.execute("INSERT INTO located_at (id, ord, event_id, place_id, provenance, justification_id) VALUES (0,0,'e','p','source-a',NULL)",[])?;
        let rel=atlas_graph_types::section_index::directed_rel_code(atlas_graph_types::edge::RelationId::LocatedAt);
        conn.execute("INSERT INTO edge_index (subject,rel,dir,ord,object,edge_id,meta_kind,meta_narrative,meta_votes,meta_parentage,row_family,row_id) VALUES ('n:Event:e',?1,0,0,'n:Place:p',zeroblob(16),0,NULL,NULL,NULL,?2,0)",rusqlite::params![rel,i64::from(atlas_graph_types::canon::RowFamily::LocatedAt.ordinal())])?;
        conn.execute("INSERT INTO label (position,label) VALUES ('n:Event:e','Event E')",[])?;
        conn.execute("INSERT INTO edge_count (subject,rel,dir,count) VALUES ('n:Event:e',?1,0,1)",[rel])?;
        let baseline = logical_dump_of_db(&conn,Section::Core)?;
        for sql in ["UPDATE label SET label=?1", "UPDATE edge_index SET object=?1", "UPDATE edge_count SET count=?1"] {
            conn.execute_batch("BEGIN")?;
            conn.execute(sql,[reassignment+1])?;
            let derived_change=logical_dump_of_db(&conn,Section::Core)?;
            assert_ne!(logical_hash(&derived_change),logical_hash(&baseline));
            conn.execute_batch("ROLLBACK")?;
        }
        let before: (i64, String) = conn.query_row("SELECT located_at.id,located_at.provenance FROM edge_index JOIN located_at ON located_at.id=edge_index.row_id",[],|row|Ok((row.get(0)?,row.get(1)?)))?;
        assert_eq!(before,(0,"source-a".to_string()));
        conn.execute("UPDATE located_at SET id=?1, ord=?1 WHERE id=0",[reassignment])?;
        let after = logical_dump_of_db(&conn,Section::Core)?;
        assert_eq!(after,baseline);
        assert_eq!(logical_hash(&after),logical_hash(&baseline));
        let after_provenance: Vec<(i64,String)> = conn.prepare("SELECT located_at.id,located_at.provenance FROM edge_index JOIN located_at ON located_at.id=edge_index.row_id")?.query_map([],|row|Ok((row.get(0)?,row.get(1)?)))?.collect::<Result<_,_>>()?;
        assert_eq!(after_provenance,Vec::<(i64,String)>::new());
        passed+=1;
    }
    println!("32 generated coordinated id/ord reassignments: production logical_dump_of_db succeeds byte-identically; logical hash unchanged; edge-index provenance join becomes empty.");
    assert_eq!(passed,32);
    println!("96 generated single-row changes across label, edge_index and edge_count all change the production logical hash.");
    if let Some(path) = std::env::args().nth(1) {
        let conn=rusqlite::Connection::open(path)?;
        let baseline=logical_dump_of_db(&conn,Section::Core)?;
        let id:i64=conn.query_row("SELECT MAX(id) FROM located_at",[],|r|r.get(0))?;
        let family=i64::from(atlas_graph_types::canon::RowFamily::LocatedAt.ordinal());
        let before:Vec<(String,i64,String)>=conn.prepare("SELECT hex(edge_index.edge_id),located_at.id,located_at.provenance FROM edge_index JOIN located_at ON located_at.id=edge_index.row_id WHERE edge_index.row_family=?1 AND located_at.id=?2 ORDER BY edge_index.subject,edge_index.rel,edge_index.dir,edge_index.ord")?.query_map(rusqlite::params![family,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect::<Result<_,_>>()?;
        assert!(!before.is_empty());
        conn.execute("UPDATE located_at SET id=?1,ord=?1 WHERE id=?2",rusqlite::params![id+32,id])?;
        let after=logical_dump_of_db(&conn,Section::Core)?;
        assert_eq!(baseline,after);
        let after_provenance:Vec<(String,i64,String)>=conn.prepare("SELECT hex(edge_index.edge_id),located_at.id,located_at.provenance FROM edge_index JOIN located_at ON located_at.id=edge_index.row_id WHERE edge_index.row_family=?1 AND located_at.id=?2 ORDER BY edge_index.subject,edge_index.rel,edge_index.dir,edge_index.ord")?.query_map(rusqlite::params![family,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect::<Result<_,_>>()?;
        assert_eq!(after_provenance,Vec::<(String,i64,String)>::new());
        println!("Actual copied 8b23906 Core: LocatedAt id/ord {id} -> {}; logical hash {}; before whole provenance links {before:?}; after []",id+32,logical_hash(&after));
    }
    Ok(())
}
