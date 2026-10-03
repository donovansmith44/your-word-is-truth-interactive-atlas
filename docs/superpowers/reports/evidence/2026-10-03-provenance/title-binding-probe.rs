#[path="exact-title-encoding.rs"]
mod exact;
use exact::{PROVENANCE_TITLE,read_table,row_body};

fn dump(conn:&rusqlite::Connection)->Result<Vec<u8>,exact::SqliteError> {
    let mut bytes=Vec::new();
    for row in read_table(conn,&PROVENANCE_TITLE)? {
        bytes.extend_from_slice(b"provenance_title\t");
        bytes.extend_from_slice(&row_body(&PROVENANCE_TITLE,&row)?);
        bytes.push(b'\n');
    }
    Ok(bytes)
}

fn hash(bytes:&[u8])->[u8;16] {
    atlas_graph_types::sha256::sha256_prefixed_128(atlas_graph_types::canon::DOMAIN_PREFIX,bytes)
}

fn main()->Result<(),Box<dyn std::error::Error>> {
    let conn=rusqlite::Connection::open(std::env::args().nth(1).ok_or("need disposable Core copy")?)?;
    let baseline=dump(&conn)?;
    let ids:Vec<String>=conn.prepare("SELECT id FROM provenance_title ORDER BY id")?.query_map([],|r|r.get(0))?.collect::<Result<_,_>>()?;
    assert_eq!(ids.len(),33);
    let mut checked=0;
    for id in &ids {
        for column in PROVENANCE_TITLE.columns {
            conn.execute_batch("BEGIN")?;
            conn.execute(&format!("UPDATE provenance_title SET {column}={column}||'__codex_changed' WHERE id=?1"),[id])?;
            let changed=dump(&conn)?;
            assert_ne!(changed,baseline);
            assert_ne!(hash(&changed),hash(&baseline));
            conn.execute_batch("ROLLBACK")?;
            assert_eq!(dump(&conn)?,baseline);
            checked+=1;
        }
    }
    println!("{} generated mutations cover id and title of all {} actual title rows; each changes complete production-encoded canonical table bytes and table-fragment hash; every rollback restores the whole table.",checked,ids.len());
    Ok(())
}
