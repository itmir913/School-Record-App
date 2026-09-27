use crate::db::with_transaction;
use crate::state::{DbState, constraint_err};
use crate::types::{ActivityDetail, AreaRef};
use rusqlite::Connection;
use std::collections::HashMap;
use tauri::State;

pub fn get_activities_impl(conn: &Connection) -> Result<Vec<ActivityDetail>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT act.id, act.name, a.id AS area_id, a.name AS area_name,
                    (SELECT COUNT(*) FROM ActivityRecord ar WHERE ar.activity_id = act.id AND ar.content != '') AS record_count
             FROM Activity act
             LEFT JOIN AreaActivity aa ON act.id = aa.activity_id
             LEFT JOIN Area a ON aa.area_id = a.id
             ORDER BY act.id, a.id",
        )
        .map_err(|e| e.to_string())?;

    let mut activities: Vec<ActivityDetail> = Vec::new();
    let mut index_map: HashMap<i64, usize> = HashMap::new();

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (act_id, act_name, area_id, area_name, record_count) = row.map_err(|e| e.to_string())?;

        let idx = if let Some(&i) = index_map.get(&act_id) {
            i
        } else {
            let i = activities.len();
            activities.push(ActivityDetail {
                id: act_id,
                name: act_name,
                areas: vec![],
                record_count,
            });
            index_map.insert(act_id, i);
            i
        };

        if let (Some(id), Some(name)) = (area_id, area_name) {
            activities[idx].areas.push(AreaRef { id, name });
        }
    }

    Ok(activities)
}

pub fn create_activity_impl(conn: &Connection, name: &str) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO Activity (name) VALUES (?1)",
        rusqlite::params![name],
    )
    .map_err(|e| constraint_err(&e, &format!("이미 같은 이름의 활동이 있습니다: {name}")))?;

    Ok(conn.last_insert_rowid())
}

pub fn update_activity_impl(conn: &Connection, id: i64, name: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE Activity SET name = ?1 WHERE id = ?2",
        rusqlite::params![name, id],
    )
    .map_err(|e| constraint_err(&e, &format!("이미 같은 이름의 활동이 있습니다: {name}")))?;

    Ok(())
}

pub fn delete_activity_impl(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute(
        "DELETE FROM Activity WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn create_activities_batch_impl(
    conn: &Connection,
    names: &[String],
) -> Result<HashMap<String, i64>, String> {
    with_transaction(conn, || {
        for name in names {
            conn.execute(
                "INSERT OR IGNORE INTO Activity (name) VALUES (?1)",
                rusqlite::params![name],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    })?;

    let mut map = HashMap::new();
    for name in names {
        let id: i64 = conn
            .query_row(
                "SELECT id FROM Activity WHERE name = ?1",
                rusqlite::params![name],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        map.insert(name.clone(), id);
    }
    Ok(map)
}

pub fn set_activity_areas_impl(
    conn: &Connection,
    activity_id: i64,
    area_ids: &[i64],
) -> Result<(), String> {
    with_transaction(conn, || replace_activity_areas(conn, activity_id, area_ids))
}

/// 활동의 영역 연결을 통째로 바꾼다. **트랜잭션을 열지 않는다** — 호출하는 쪽이
/// `with_transaction` 안에서 불러야 한다.
fn replace_activity_areas(
    conn: &Connection,
    activity_id: i64,
    area_ids: &[i64],
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM AreaActivity WHERE activity_id = ?1",
        rusqlite::params![activity_id],
    )
    .map_err(|e| e.to_string())?;

    for area_id in area_ids.iter() {
        conn.execute(
            "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
            rusqlite::params![area_id, activity_id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 활동 저장(추가·수정)과 영역 연결을 **한 트랜잭션**으로 처리한다. `id`가 없으면 추가다.
///
/// 따로 부르면 추가는 됐는데 연결에서 실패할 때 영역 없는 활동이 남고, 같은 모달에서
/// 다시 저장하면 이름 UNIQUE에 걸려 되돌릴 방법이 없었다. 저장된 활동 id를 돌려준다.
pub fn save_activity_impl(
    conn: &Connection,
    id: Option<i64>,
    name: &str,
    area_ids: &[i64],
) -> Result<i64, String> {
    with_transaction(conn, || {
        let activity_id = match id {
            Some(id) => {
                update_activity_impl(conn, id, name)?;
                id
            }
            None => create_activity_impl(conn, name)?,
        };
        replace_activity_areas(conn, activity_id, area_ids)?;
        Ok(activity_id)
    })
}

// ── Tauri 커맨드 (얇은 래퍼) ─────────────────────────────────

#[tauri::command]
pub fn get_activities(state: State<DbState>) -> Result<Vec<ActivityDetail>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    get_activities_impl(conn)
}

#[tauri::command]
pub fn create_activity(name: String, state: State<DbState>) -> Result<i64, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    create_activity_impl(conn, &name)
}

#[tauri::command]
pub fn update_activity(id: i64, name: String, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    update_activity_impl(conn, id, &name)
}

#[tauri::command]
pub fn delete_activity(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    delete_activity_impl(conn, id)
}

#[tauri::command]
pub fn create_activities_batch(
    names: Vec<String>,
    state: State<DbState>,
) -> Result<HashMap<String, i64>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    create_activities_batch_impl(conn, &names)
}

#[tauri::command]
pub fn set_activity_areas(
    activity_id: i64,
    area_ids: Vec<i64>,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    set_activity_areas_impl(conn, activity_id, &area_ids)
}

#[tauri::command]
pub fn save_activity(
    id: Option<i64>,
    name: String,
    area_ids: Vec<i64>,
    state: State<DbState>,
) -> Result<i64, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    save_activity_impl(conn, id, &name, &area_ids)
}
