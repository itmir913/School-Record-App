use rusqlite::{Connection, Result};
use std::path::Path;

pub fn open(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    initialize(&conn)?;
    Ok(conn)
}

fn initialize(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("schema.sql"))?;
    Ok(())
}