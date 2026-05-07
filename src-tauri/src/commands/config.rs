use crate::state::DbState;
use rusqlite::Connection;
use tauri::State;

/// 반환값: None = 버전 동일(모달 표시 안 함), Some(old) = 이전 버전(모달 표시)
/// old가 빈 문자열이면 이전 레코드가 없었음(하위 호환) → 전체 노트 표시
pub fn check_and_update_app_version_impl(conn: &Connection, current_version: &str) -> Result<Option<String>, String> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT config_value FROM APP_CONFIGS WHERE config_key = 'app_version'",
            [],
            |row| row.get(0),
        )
        .ok();

    if stored.as_deref() == Some(current_version) {
        return Ok(None);
    }

    conn.execute(
        "INSERT OR REPLACE INTO APP_CONFIGS (config_key, config_value) VALUES ('app_version', ?1)",
        [current_version],
    )
    .map_err(|e| e.to_string())?;

    Ok(Some(stored.unwrap_or_default()))
}

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
pub async fn check_and_update_app_version(
    db: State<'_, DbState>,
    current_version: String,
) -> Result<Option<String>, String> {
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("DB not open")?;
    check_and_update_app_version_impl(conn, &current_version)
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
