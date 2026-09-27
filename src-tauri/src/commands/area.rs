use crate::commands::student::replace_area_activities;
use crate::db::with_transaction;
use crate::state::{DbState, constraint_err};
use crate::types::{ActivityItem, AreaItem};
use rusqlite::Connection;
use std::collections::HashMap;
use tauri::State;

pub fn get_areas_impl(conn: &Connection) -> Result<Vec<AreaItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.name, a.byte_limit,
                    act.id AS act_id, act.name AS act_name
             FROM Area a
             LEFT JOIN AreaActivity aa ON a.id = aa.area_id
             LEFT JOIN Activity act ON aa.activity_id = act.id
             ORDER BY a.name ASC",
        )
        .map_err(|e| e.to_string())?;

    let mut areas: Vec<AreaItem> = Vec::new();
    let mut index_map: HashMap<i64, usize> = HashMap::new();

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (area_id, area_name, byte_limit, act_id, act_name) =
            row.map_err(|e| e.to_string())?;

        let idx = if let Some(&i) = index_map.get(&area_id) {
            i
        } else {
            let i = areas.len();
            areas.push(AreaItem {
                id: area_id,
                name: area_name,
                byte_limit,
                activities: vec![],
            });
            index_map.insert(area_id, i);
            i
        };

        if let (Some(id), Some(name)) = (act_id, act_name) {
            areas[idx].activities.push(ActivityItem { id, name });
        }
    }

    Ok(areas)
}

/// byte_limit이 스키마의 CHECK 제약(> 0)을 만족하는지 검사한다.
pub fn validate_byte_limit(byte_limit: i64) -> Result<(), String> {
    if byte_limit < 1 {
        return Err(format!("글자 수 제한은 1 이상이어야 합니다: {byte_limit}"));
    }
    Ok(())
}

pub fn create_area_impl(conn: &Connection, name: &str, byte_limit: i64) -> Result<i64, String> {
    validate_byte_limit(byte_limit)?;
    conn.execute(
        "INSERT INTO Area (name, byte_limit) VALUES (?1, ?2)",
        rusqlite::params![name, byte_limit],
    )
    .map_err(|e| constraint_err(&e, &format!("이미 같은 이름의 영역이 있습니다: {name}")))?;

    Ok(conn.last_insert_rowid())
}

pub fn update_area_impl(conn: &Connection, id: i64, name: &str, byte_limit: i64) -> Result<(), String> {
    validate_byte_limit(byte_limit)?;
    conn.execute(
        "UPDATE Area SET name = ?1, byte_limit = ?2 WHERE id = ?3",
        rusqlite::params![name, byte_limit, id],
    )
    .map_err(|e| constraint_err(&e, &format!("이미 같은 이름의 영역이 있습니다: {name}")))?;

    Ok(())
}

/// 영역 저장(추가·수정)과 활동 연결을 **한 트랜잭션**으로 처리한다. `id`가 없으면 추가다.
///
/// 따로 부르면 추가는 됐는데 연결에서 실패할 때 활동 없는 영역이 남고, 같은 모달에서
/// 다시 저장하면 이름 UNIQUE에 걸려 되돌릴 방법이 없었다. 저장된 영역 id를 돌려준다.
pub fn save_area_impl(
    conn: &Connection,
    id: Option<i64>,
    name: &str,
    byte_limit: i64,
    activity_ids: &[i64],
) -> Result<i64, String> {
    with_transaction(conn, || {
        let area_id = match id {
            Some(id) => {
                update_area_impl(conn, id, name, byte_limit)?;
                id
            }
            None => create_area_impl(conn, name, byte_limit)?,
        };
        replace_area_activities(conn, area_id, activity_ids)?;
        Ok(area_id)
    })
}

pub fn delete_area_impl(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM Area WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ── Tauri 커맨드 (얇은 래퍼) ─────────────────────────────────

#[tauri::command]
pub fn get_areas(state: State<DbState>) -> Result<Vec<AreaItem>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    get_areas_impl(conn)
}

#[tauri::command]
pub fn create_area(name: String, byte_limit: i64, state: State<DbState>) -> Result<i64, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    create_area_impl(conn, &name, byte_limit)
}

#[tauri::command]
pub fn update_area(id: i64, name: String, byte_limit: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    update_area_impl(conn, id, &name, byte_limit)
}

#[tauri::command]
pub fn delete_area(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    delete_area_impl(conn, id)
}

#[tauri::command]
pub fn save_area(
    id: Option<i64>,
    name: String,
    byte_limit: i64,
    activity_ids: Vec<i64>,
    state: State<DbState>,
) -> Result<i64, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    save_area_impl(conn, id, &name, byte_limit, &activity_ids)
}
