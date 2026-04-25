use crate::state::DbState;
use crate::types::SnapshotItem;
use rusqlite::Connection;
use tauri::State;

pub fn create_snapshot_impl(conn: &Connection, memo: Option<String>) -> Result<SnapshotItem, String> {
    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

    let result: Result<SnapshotItem, String> = (|| {
        conn.execute(
            "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
             SELECT r.id, r.content, r.updated_at, NULL
             FROM ActivityRecord r
             WHERE NOT EXISTS (
                 SELECT 1 FROM ActivityRecordHistory h
                 WHERE h.activity_record_id = r.id
                   AND h.changed_at = r.updated_at
             )",
            [],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO Snapshot (memo) VALUES (?1)",
            rusqlite::params![memo],
        )
        .map_err(|e| e.to_string())?;

        let snapshot_id = conn.last_insert_rowid();
        let created_at: String = conn
            .query_row(
                "SELECT created_at FROM Snapshot WHERE id = ?1",
                rusqlite::params![snapshot_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        Ok(SnapshotItem { id: snapshot_id, memo, created_at })
    })();

    match result {
        Ok(item) => {
            conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            Ok(item)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

pub fn get_snapshots_impl(conn: &Connection) -> Result<Vec<SnapshotItem>, String> {
    let mut stmt = conn
        .prepare("SELECT id, memo, created_at FROM Snapshot ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map([], |row| {
            Ok(SnapshotItem {
                id: row.get(0)?,
                memo: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(items)
}

pub fn restore_snapshot_impl(conn: &Connection, snapshot_id: i64) -> Result<i64, String> {
    let snapshot_at: String = conn
        .query_row(
            "SELECT created_at FROM Snapshot WHERE id = ?1",
            rusqlite::params![snapshot_id],
            |row| row.get(0),
        )
        .map_err(|_| format!("스냅샷을 찾을 수 없습니다. id={snapshot_id}"))?;

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

    let result: Result<i64, String> = (|| {
        let rows = conn
            .execute(
                "UPDATE ActivityRecord SET
                   content = COALESCE(
                     (SELECT h.content
                      FROM ActivityRecordHistory h
                      WHERE h.activity_record_id = ActivityRecord.id
                        AND h.changed_at <= ?1
                      ORDER BY h.changed_at DESC LIMIT 1),
                     ''
                   ),
                   updated_at = datetime('now')",
                rusqlite::params![snapshot_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(rows as i64)
    })();

    match result {
        Ok(count) => {
            conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            Ok(count)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

#[tauri::command]
pub fn create_snapshot(
    memo: Option<String>,
    state: State<DbState>,
) -> Result<SnapshotItem, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    create_snapshot_impl(conn, memo)
}

#[tauri::command]
pub fn get_snapshots(state: State<DbState>) -> Result<Vec<SnapshotItem>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    get_snapshots_impl(conn)
}

#[tauri::command]
pub fn restore_snapshot(
    snapshot_id: i64,
    state: State<DbState>,
) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    restore_snapshot_impl(conn, snapshot_id)
}
