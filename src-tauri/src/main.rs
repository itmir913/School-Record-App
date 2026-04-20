// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;

use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

struct DbState(Mutex<Option<Connection>>);

// ── 직렬화 구조체 ────────────────────────────────────────────

/// AreaItem.activities 에서 사용하는 단순 항목
#[derive(Serialize, Clone)]
struct ActivityItem {
    id: i64,
    name: String,
}

#[derive(Serialize, Clone)]
struct AreaItem {
    id: i64,
    name: String,
    byte_limit: i64,
    activities: Vec<ActivityItem>,
}

/// ActivitySection 카드에서 사용하는 소속 영역 참조
#[derive(Serialize, Clone)]
struct AreaRef {
    id: i64,
    name: String,
}

/// get_activities 가 반환하는 풍부한 Activity 항목
#[derive(Serialize, Clone)]
struct ActivityDetail {
    id: i64,
    name: String,
    areas: Vec<AreaRef>,
    record_count: i64,
}

// ── 헬퍼 매크로: 커넥션 잠금 ─────────────────────────────────

macro_rules! conn {
    ($state:expr) => {{
        let guard = $state.0.lock().unwrap();
        guard
            .as_ref()
            .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())
            .map(|c| c as *const Connection)
    }};
}

// ── 프로젝트 커맨드 ──────────────────────────────────────────

#[tauri::command]
fn new_project(path: String, state: State<DbState>) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.exists() {
        return Err(format!("이미 파일이 존재합니다: {path}"));
    }
    let conn = db::create_new(p).map_err(|e| e.to_string())?;
    *state.0.lock().unwrap() = Some(conn);
    Ok(())
}

#[tauri::command]
fn open_project(path: String, state: State<DbState>) -> Result<(), String> {
    let conn = db::open_existing(std::path::Path::new(&path))
        .map_err(|e| e.to_string())?;
    *state.0.lock().unwrap() = Some(conn);
    Ok(())
}

// ── Area 커맨드 ──────────────────────────────────────────────

#[tauri::command]
fn get_areas(state: State<DbState>) -> Result<Vec<AreaItem>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.name, a.byte_limit,
                    act.id AS act_id, act.name AS act_name
             FROM Area a
             LEFT JOIN AreaActivity aa ON a.id = aa.area_id
             LEFT JOIN Activity act ON aa.activity_id = act.id
             ORDER BY a.id, aa.default_order",
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

#[tauri::command]
fn create_area(name: String, byte_limit: i64, state: State<DbState>) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute(
        "INSERT INTO Area (name, byte_limit) VALUES (?1, ?2)",
        rusqlite::params![name, byte_limit],
    )
        .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn update_area(id: i64, name: String, byte_limit: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute(
        "UPDATE Area SET name = ?1, byte_limit = ?2 WHERE id = ?3",
        rusqlite::params![name, byte_limit, id],
    )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn delete_area(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute("DELETE FROM Area WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ── Activity 커맨드 ──────────────────────────────────────────

/// 모든 Activity를 소속 Area 목록과 함께 반환
#[tauri::command]
fn get_activities(state: State<DbState>) -> Result<Vec<ActivityDetail>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT act.id, act.name, a.id AS area_id, a.name AS area_name,
                    (SELECT COUNT(*) FROM ActivityRecord ar WHERE ar.activity_id = act.id) AS record_count
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

#[tauri::command]
fn create_activity(name: String, state: State<DbState>) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute(
        "INSERT INTO Activity (name) VALUES (?1)",
        rusqlite::params![name],
    )
        .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn update_activity(id: i64, name: String, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute(
        "UPDATE Activity SET name = ?1 WHERE id = ?2",
        rusqlite::params![name, id],
    )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn delete_activity(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute(
        "DELETE FROM Activity WHERE id = ?1",
        rusqlite::params![id],
    )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn set_activity_areas(
    activity_id: i64,
    area_ids: Vec<i64>,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    // 이 활동의 기존 영역 매핑 전체 제거
    conn.execute(
        "DELETE FROM AreaActivity WHERE activity_id = ?1",
        rusqlite::params![activity_id],
    )
        .map_err(|e| e.to_string())?;

    // 각 영역에 활동 추가 (해당 영역의 마지막 순서로)
    for area_id in area_ids.iter() {
        conn.execute(
            "INSERT INTO AreaActivity (area_id, activity_id, default_order)
             VALUES (?1, ?2,
               COALESCE((SELECT MAX(default_order) FROM AreaActivity WHERE area_id = ?1), 0) + 1)",
            rusqlite::params![area_id, activity_id],
        )
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn set_area_activities(
    area_id: i64,
    activity_ids: Vec<i64>,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute(
        "DELETE FROM AreaActivity WHERE area_id = ?1",
        rusqlite::params![area_id],
    )
        .map_err(|e| e.to_string())?;

    for (order, act_id) in activity_ids.iter().enumerate() {
        conn.execute(
            "INSERT INTO AreaActivity (area_id, activity_id, default_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![area_id, act_id, (order + 1) as i64],
        )
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ── 앱 진입점 ────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DbState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            new_project,
            open_project,
            get_areas,
            create_area,
            update_area,
            delete_area,
            get_activities,
            set_area_activities,
            create_activity,
            update_activity,
            delete_activity,
            set_activity_areas,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
