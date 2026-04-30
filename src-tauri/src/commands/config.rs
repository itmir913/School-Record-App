use crate::state::DbState;
use rusqlite::Connection;
use tauri::State;

pub fn get_config_impl(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    let mut stmt = conn
        .prepare("SELECT config_value FROM APP_CONFIGS WHERE config_key = ?1")
        .map_err(|e| e.to_string())?;

    Ok(stmt.query_row([key], |row| row.get::<_, String>(0)).ok())
}

pub fn set_config_impl(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO APP_CONFIGS (config_key, config_value) VALUES (?1, ?2)",
        [key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_config(
    db: State<'_, DbState>,
    key: String,
) -> Result<Option<String>, String> {
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("DB not open")?;
    get_config_impl(conn, &key)
}

#[tauri::command]
pub async fn set_config(
    db: State<'_, DbState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("DB not open")?;
    set_config_impl(conn, &key, &value)
}
