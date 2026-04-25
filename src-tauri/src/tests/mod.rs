use rusqlite::Connection;

pub mod engine_tests;
pub mod db_tests;
pub mod area_tests;
pub mod activity_tests;
pub mod student_tests;
pub mod record_tests;
pub mod snapshot_tests;
pub mod replace_tests;

pub fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(include_str!("../schema.sql")).unwrap();
    conn
}

pub fn insert_student(conn: &Connection, grade: i64, class_num: i64, number: i64, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO Student (grade, class_num, number, name) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![grade, class_num, number, name],
    )
    .unwrap();
    conn.last_insert_rowid()
}

pub fn insert_area(conn: &Connection, name: &str, byte_limit: i64) -> i64 {
    conn.execute(
        "INSERT INTO Area (name, byte_limit) VALUES (?1, ?2)",
        rusqlite::params![name, byte_limit],
    )
    .unwrap();
    conn.last_insert_rowid()
}

pub fn insert_activity(conn: &Connection, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO Activity (name) VALUES (?1)",
        rusqlite::params![name],
    )
    .unwrap();
    conn.last_insert_rowid()
}

pub fn insert_record(conn: &Connection, activity_id: i64, student_id: i64, content: &str) {
    conn.execute(
        "INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, ?3)",
        rusqlite::params![activity_id, student_id, content],
    )
    .unwrap();
}
