// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use tauri::State;

struct DbState(Mutex<Option<Connection>>);

// ── 치환 엔진 상태 ────────────────────────────────────────────

struct ReplaceCache {
    ruleset_version: u64,
    /// hash(content) → (result, version)
    entries: HashMap<u64, (String, u64)>,
}

type ReplaceCacheState = Mutex<ReplaceCache>;

fn unique_err(e: &rusqlite::Error, conflict_msg: &str) -> String {
    if e.to_string().contains("UNIQUE constraint failed") {
        conflict_msg.to_string()
    } else {
        e.to_string()
    }
}

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

#[derive(Serialize, Clone)]
struct StudentItem {
    id: i64,
    grade: i64,
    class_num: i64,
    number: i64,
    name: String,
}

// ── 치환 규칙 구조체 ─────────────────────────────────────────

#[derive(Serialize, Clone)]
struct ReplaceRule {
    id: i64,
    old_text: String,
    new_text: String,
    enabled: bool,
    priority: i64,
    created_at: String,
    updated_at: String,
    /// 충돌하는 규칙 id 목록 (get_replace_rules에서 계산, DB에 저장 안 됨)
    conflicts: Vec<i64>,
}

#[derive(Serialize)]
struct ReplacePreviewItem {
    activity_id: i64,
    student_id: i64,
    activity_name: String,
    student_name: String,
    original: String,
    result: String,
}

#[derive(Serialize)]
struct ReplaceApplyResult {
    changed_count: i64,
    total_count: i64,
}

/// get_activities 가 반환하는 풍부한 Activity 항목
#[derive(Serialize, Clone)]
struct ActivityDetail {
    id: i64,
    name: String,
    areas: Vec<AreaRef>,
    record_count: i64,
}

// ── 유의어 점검 구조체 ────────────────────────────────────────

#[derive(Serialize)]
struct SynonymWordItem {
    id: i64,
    group_id: i64,
    word: String,
}

#[derive(Serialize)]
struct SynonymGroupFull {
    id: i64,
    name: String,
    created_at: String,
    items: Vec<SynonymWordItem>,
}

#[derive(Deserialize)]
struct SeedGroupInput {
    name: String,
    words: Vec<String>,
}

#[derive(Serialize)]
struct InspectRecord {
    id: i64,
    activity_name: String,
    student_name: String,
    area_name: String,
    grade: i64,
    class_num: i64,
    number: i64,
    content: String,
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
    let src = std::path::Path::new(&path);

    if let Some(parent) = src.parent() {
        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("backup");
        let ts = chrono::Local::now().format("%y%m%d-%H%M").to_string();
        let bak_name = format!("{stem}.{ts}.db.backup");
        let _ = std::fs::copy(src, parent.join(bak_name));
    }

    let conn = db::open_existing(src).map_err(|e| e.to_string())?;
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
             ORDER BY a.id",
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
        .map_err(|e| unique_err(&e, &format!("이미 같은 이름의 영역이 있습니다: {name}")))?;

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
        .map_err(|e| unique_err(&e, &format!("이미 같은 이름의 영역이 있습니다: {name}")))?;

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
        .map_err(|e| unique_err(&e, &format!("이미 같은 이름의 활동이 있습니다: {name}")))?;

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
        .map_err(|e| unique_err(&e, &format!("이미 같은 이름의 활동이 있습니다: {name}")))?;

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

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        // 이 활동의 기존 영역 매핑 전체 제거
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
    })();
    match result {
        Ok(_) => conn.execute_batch("COMMIT").map_err(|e| e.to_string()),
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

// ── Student 커맨드 ────────────────────────────────────────────

#[tauri::command]
fn get_students(state: State<DbState>) -> Result<Vec<StudentItem>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, grade, class_num, number, name
             FROM Student
             ORDER BY grade, class_num, number",
        )
        .map_err(|e| e.to_string())?;

    let students = stmt
        .query_map([], |row| {
            Ok(StudentItem {
                id: row.get(0)?,
                grade: row.get(1)?,
                class_num: row.get(2)?,
                number: row.get(3)?,
                name: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(students)
}

#[tauri::command]
fn create_student(
    grade: i64,
    class_num: i64,
    number: i64,
    name: String,
    state: State<DbState>,
) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute(
        "INSERT INTO Student (grade, class_num, number, name) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![grade, class_num, number, name],
    )
        .map_err(|e| unique_err(&e, &format!("이미 같은 학번의 학생이 있습니다: {grade}학년 {class_num}반 {number}번")))?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn update_student(
    id: i64,
    grade: i64,
    class_num: i64,
    number: i64,
    name: String,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute(
        "UPDATE Student SET grade = ?1, class_num = ?2, number = ?3, name = ?4 WHERE id = ?5",
        rusqlite::params![grade, class_num, number, name, id],
    )
        .map_err(|e| unique_err(&e, &format!("이미 같은 학번의 학생이 있습니다: {grade}학년 {class_num}반 {number}번")))?;

    Ok(())
}

#[tauri::command]
fn delete_student(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute("DELETE FROM Student WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;

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

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        conn.execute(
            "DELETE FROM AreaActivity WHERE area_id = ?1",
            rusqlite::params![area_id],
        )
            .map_err(|e| e.to_string())?;

        for act_id in activity_ids.iter() {
            conn.execute(
                "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
                rusqlite::params![area_id, act_id],
            )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    match result {
        Ok(_) => conn.execute_batch("COMMIT").map_err(|e| e.to_string()),
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

#[derive(Deserialize)]
struct StudentInput {
    grade: i64,
    class_num: i64,
    number: i64,
    name: String,
}

#[derive(Serialize)]
struct BulkUpsertResult {
    inserted: i64,
    updated: i64,
}

#[tauri::command]
fn bulk_upsert_students(
    students: Vec<StudentInput>,
    state: State<DbState>,
) -> Result<BulkUpsertResult, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<BulkUpsertResult, String> {
        let mut inserted: i64 = 0;
        let mut updated: i64 = 0;
        for s in students.iter() {
            conn.execute(
                "INSERT OR IGNORE INTO Student (grade, class_num, number, name)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![s.grade, s.class_num, s.number, s.name],
            )
                .map_err(|e| e.to_string())?;

            if conn.changes() > 0 {
                inserted += 1;
            } else {
                conn.execute(
                    "UPDATE Student SET name = ?1 WHERE grade=?2 AND class_num=?3 AND number=?4",
                    rusqlite::params![s.name, s.grade, s.class_num, s.number],
                )
                    .map_err(|e| e.to_string())?;
                updated += 1;
            }
        }
        Ok(BulkUpsertResult { inserted, updated })
    })();
    match result {
        Ok(r) => {
            conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            Ok(r)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

// ── AreaStudent 커맨드 ────────────────────────────────────────

#[tauri::command]
fn get_area_students(area_id: i64, state: State<DbState>) -> Result<Vec<i64>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let mut stmt = conn
        .prepare("SELECT student_id FROM AreaStudent WHERE area_id = ?1")
        .map_err(|e| e.to_string())?;

    let ids = stmt
        .query_map(rusqlite::params![area_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ids)
}

#[tauri::command]
fn set_area_students(
    area_id: i64,
    student_ids: Vec<i64>,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        conn.execute(
            "DELETE FROM AreaStudent WHERE area_id = ?1",
            rusqlite::params![area_id],
        )
            .map_err(|e| e.to_string())?;

        for student_id in student_ids.iter() {
            conn.execute(
                "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
                rusqlite::params![area_id, student_id],
            )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    match result {
        Ok(_) => conn.execute_batch("COMMIT").map_err(|e| e.to_string()),
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

// ── 기록 그리드 커맨드 ────────────────────────────────────────

#[derive(Serialize, Clone)]
struct RecordCell {
    activity_id: i64,
    student_id: i64,
    content: String,
}

#[derive(Serialize, Clone)]
struct AreaGridData {
    activities: Vec<ActivityItem>,
    students: Vec<StudentItem>,
    records: Vec<RecordCell>,
}

#[tauri::command]
fn get_area_grid(area_id: i64, state: State<DbState>) -> Result<AreaGridData, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    // 영역에 속한 활동 목록
    let mut stmt = conn
        .prepare(
            "SELECT act.id, act.name
             FROM Activity act
             JOIN AreaActivity aa ON act.id = aa.activity_id
             WHERE aa.area_id = ?1
             ORDER BY act.name ASC",
        )
        .map_err(|e| e.to_string())?;

    let activities = stmt
        .query_map(rusqlite::params![area_id], |row| {
            Ok(ActivityItem {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 영역에 배정된 학생 목록
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.grade, s.class_num, s.number, s.name
             FROM Student s
             JOIN AreaStudent as_ ON s.id = as_.student_id
             WHERE as_.area_id = ?1
             ORDER BY s.grade, s.class_num, s.number",
        )
        .map_err(|e| e.to_string())?;

    let students = stmt
        .query_map(rusqlite::params![area_id], |row| {
            Ok(StudentItem {
                id: row.get(0)?,
                grade: row.get(1)?,
                class_num: row.get(2)?,
                number: row.get(3)?,
                name: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 해당 활동들의 기록 (이 영역에 배정된 학생으로만 필터링)
    let activity_ids: Vec<i64> = activities.iter().map(|a| a.id).collect();
    let student_ids: Vec<i64> = students.iter().map(|s| s.id).collect();
    let records = if activity_ids.is_empty() || student_ids.is_empty() {
        vec![]
    } else {
        let act_placeholders = activity_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let stu_placeholders = student_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", activity_ids.len() + i + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT activity_id, student_id, content
             FROM ActivityRecord
             WHERE activity_id IN ({})
               AND student_id IN ({})",
            act_placeholders, stu_placeholders
        );
        let params: Vec<i64> = activity_ids.iter().chain(student_ids.iter()).copied().collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok(RecordCell {
                    activity_id: row.get(0)?,
                    student_id: row.get(1)?,
                    content: row.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };

    Ok(AreaGridData { activities, students, records })
}

#[tauri::command]
fn upsert_record(
    activity_id: i64,
    student_id: i64,
    content: String,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute(
        "INSERT INTO ActivityRecord (activity_id, student_id, content, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(activity_id, student_id) DO UPDATE SET
           content = excluded.content,
           updated_at = excluded.updated_at",
        rusqlite::params![activity_id, student_id, content],
    )
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ── 히스토리 커맨드 ──────────────────────────────────────────

#[derive(Serialize)]
struct HistoryEntry {
    id: i64,
    content: String,
    changed_at: String,
    note: Option<String>,
}

/// 특정 셀의 히스토리 목록 조회 (최신순, 페이지네이션)
#[tauri::command]
fn get_record_history(
    activity_id: i64,
    student_id: i64,
    limit: i64,
    offset: i64,
    state: State<DbState>,
) -> Result<Vec<HistoryEntry>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT h.id, h.content, h.changed_at, h.note
             FROM ActivityRecordHistory h
             JOIN ActivityRecord r ON r.id = h.activity_record_id
             WHERE r.activity_id = ?1 AND r.student_id = ?2
             ORDER BY h.changed_at DESC
             LIMIT ?3 OFFSET ?4",
        )
        .map_err(|e| e.to_string())?;

    let entries = stmt
        .query_map(rusqlite::params![activity_id, student_id, limit, offset], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                changed_at: row.get(2)?,
                note: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(entries)
}

/// 현재 DB content를 히스토리에 스냅샷으로 기록.
/// 같은 버전(changed_at = updated_at)의 항목이 이미 존재하면 note만 갱신한다.
fn save_snapshot_internal(
    conn: &Connection,
    activity_id: i64,
    student_id: i64,
    note: Option<&str>,
) -> Result<(), String> {
    // 해당 버전 항목이 없을 때만 INSERT
    let inserted = conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         SELECT r.id, r.content, r.updated_at, ?3
         FROM ActivityRecord r
         WHERE r.activity_id = ?1 AND r.student_id = ?2
           AND NOT EXISTS (
               SELECT 1 FROM ActivityRecordHistory h
               WHERE h.activity_record_id = r.id
                 AND h.changed_at = r.updated_at
           )",
        rusqlite::params![activity_id, student_id, note],
    )
    .map_err(|e| e.to_string())?;

    // 이미 존재하면 note만 갱신
    if inserted == 0 {
        conn.execute(
            "UPDATE ActivityRecordHistory SET note = ?3
             WHERE activity_record_id = (
                 SELECT r.id FROM ActivityRecord r
                 WHERE r.activity_id = ?1 AND r.student_id = ?2
             )
             AND changed_at = (
                 SELECT r.updated_at FROM ActivityRecord r
                 WHERE r.activity_id = ?1 AND r.student_id = ?2
             )",
            rusqlite::params![activity_id, student_id, note],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn save_history_snapshot(
    activity_id: i64,
    student_id: i64,
    note: Option<String>,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    save_snapshot_internal(conn, activity_id, student_id, note.as_deref())
}

// ── 일괄 기록 가져오기 ────────────────────────────────────────

#[derive(Deserialize)]
struct ImportRecordInput {
    grade: i64,
    class_num: i64,
    number: i64,
    name: Option<String>,
    activity_id: i64,
    content: String,
}

#[derive(Serialize)]
struct BulkImportResult {
    students_created: i64,
    students_updated: i64,
    records_saved: i64,
}

#[tauri::command]
fn bulk_import_records(
    records: Vec<ImportRecordInput>,
    state: State<DbState>,
) -> Result<BulkImportResult, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

    let result = (|| -> Result<BulkImportResult, String> {
        let mut students_created: i64 = 0;
        let mut students_updated: i64 = 0;
        let mut records_saved: i64 = 0;
        let mut student_cache: HashMap<(i64, i64, i64), i64> = HashMap::new();

        for r in records.iter() {
            let key = (r.grade, r.class_num, r.number);

            if !student_cache.contains_key(&key) {
                let exists: bool = conn
                    .query_row(
                        "SELECT COUNT(*) FROM Student WHERE grade=?1 AND class_num=?2 AND number=?3",
                        rusqlite::params![r.grade, r.class_num, r.number],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|e| e.to_string())?
                    > 0;

                if exists {
                    if let Some(ref n) = r.name {
                        if !n.is_empty() {
                            let existing_name: String = conn
                                .query_row(
                                    "SELECT name FROM Student WHERE grade=?1 AND class_num=?2 AND number=?3",
                                    rusqlite::params![r.grade, r.class_num, r.number],
                                    |row| row.get(0),
                                )
                                .map_err(|e| e.to_string())?;
                            if existing_name.trim().is_empty() {
                                conn.execute(
                                    "UPDATE Student SET name = ?1 WHERE grade=?2 AND class_num=?3 AND number=?4",
                                    rusqlite::params![n, r.grade, r.class_num, r.number],
                                )
                                    .map_err(|e| e.to_string())?;
                            }
                        }
                    }
                    students_updated += 1;
                } else {
                    let name = r.name.as_deref().unwrap_or("이름 없음");
                    conn.execute(
                        "INSERT INTO Student (grade, class_num, number, name) VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![r.grade, r.class_num, r.number, name],
                    )
                        .map_err(|e| e.to_string())?;
                    students_created += 1;
                }

                let student_id: i64 = conn
                    .query_row(
                        "SELECT id FROM Student WHERE grade=?1 AND class_num=?2 AND number=?3",
                        rusqlite::params![r.grade, r.class_num, r.number],
                        |row| row.get(0),
                    )
                    .map_err(|e| e.to_string())?;

                student_cache.insert(key, student_id);
            }

            let &student_id = student_cache.get(&key).ok_or_else(|| "캐시 오류".to_string())?;

            conn.execute(
                "INSERT INTO ActivityRecord (activity_id, student_id, content, updated_at)
                 VALUES (?1, ?2, ?3, datetime('now'))
                 ON CONFLICT(activity_id, student_id) DO UPDATE SET
                   content = excluded.content,
                   updated_at = excluded.updated_at",
                rusqlite::params![r.activity_id, student_id, r.content],
            )
                .map_err(|e| e.to_string())?;

            if !r.content.is_empty() {
                conn.execute(
                    "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
                     SELECT r.id, r.content, r.updated_at, 'import'
                     FROM ActivityRecord r
                     WHERE r.activity_id = ?1 AND r.student_id = ?2
                       AND NOT EXISTS (
                           SELECT 1 FROM ActivityRecordHistory h
                           WHERE h.activity_record_id = r.id
                             AND h.changed_at = r.updated_at
                       )",
                    rusqlite::params![r.activity_id, student_id],
                )
                    .map_err(|e| e.to_string())?;
            }
            records_saved += 1;
        }

        Ok(BulkImportResult { students_created, students_updated, records_saved })
    })();

    match result {
        Ok(r) => {
            conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            Ok(r)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

// ── Import 미리보기 ───────────────────────────────────────────

#[derive(Serialize)]
struct PreviewImportItem {
    grade: i64,
    class_num: i64,
    number: i64,
    student_name: String,
    activity_id: i64,
    activity_name: String,
    new_content: String,
    existing_content: String,
}

#[tauri::command]
fn preview_import_records(
    records: Vec<ImportRecordInput>,
    state: State<DbState>,
) -> Result<Vec<PreviewImportItem>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let mut result = Vec::new();
    let mut student_cache: HashMap<(i64, i64, i64), Option<(i64, String)>> = HashMap::new();
    let mut activity_cache: HashMap<i64, String> = HashMap::new();

    for r in records.iter() {
        let activity_name = if let Some(name) = activity_cache.get(&r.activity_id) {
            name.clone()
        } else {
            let name: Option<String> = conn
                .query_row(
                    "SELECT name FROM Activity WHERE id = ?1",
                    rusqlite::params![r.activity_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            let name = name.unwrap_or_else(|| format!("활동 #{}", r.activity_id));
            activity_cache.insert(r.activity_id, name.clone());
            name
        };

        let key = (r.grade, r.class_num, r.number);
        let student_info = if let Some(cached) = student_cache.get(&key) {
            cached.clone()
        } else {
            let info: Option<(i64, String)> = conn
                .query_row(
                    "SELECT id, name FROM Student WHERE grade=?1 AND class_num=?2 AND number=?3",
                    rusqlite::params![r.grade, r.class_num, r.number],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            student_cache.insert(key, info.clone());
            info
        };

        let (student_name, existing_content) = match student_info {
            Some((student_id, name)) => {
                let content: Option<String> = conn
                    .query_row(
                        "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
                        rusqlite::params![r.activity_id, student_id],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                (name, content.unwrap_or_default())
            }
            None => {
                let name = r.name.as_deref().unwrap_or("이름 없음").to_string();
                (name, String::new())
            }
        };

        result.push(PreviewImportItem {
            grade: r.grade,
            class_num: r.class_num,
            number: r.number,
            student_name,
            activity_id: r.activity_id,
            activity_name,
            new_content: r.content.clone(),
            existing_content,
        });
    }

    Ok(result)
}

// ── 스냅샷 ───────────────────────────────────────────────────

#[derive(Serialize)]
struct SnapshotItem {
    id: i64,
    memo: Option<String>,
    created_at: String,
}

#[tauri::command]
fn create_snapshot(
    memo: Option<String>,
    state: State<DbState>,
) -> Result<SnapshotItem, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

    let result: Result<SnapshotItem, String> = (|| {
        // 변경된 레코드만 history 삽입 (기존 노트 보존을 위해 note = NULL)
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

#[tauri::command]
fn get_snapshots(state: State<DbState>) -> Result<Vec<SnapshotItem>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

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

#[tauri::command]
fn restore_snapshot(
    snapshot_id: i64,
    state: State<DbState>,
) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let snapshot_at: String = conn
        .query_row(
            "SELECT created_at FROM Snapshot WHERE id = ?1",
            rusqlite::params![snapshot_id],
            |row| row.get(0),
        )
        .map_err(|_| format!("스냅샷을 찾을 수 없습니다. id={snapshot_id}"))?;

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

    let result: Result<i64, String> = (|| {
        // history 있는 레코드 → 해당 시점 content 복원
        // history 없는 레코드(스냅샷 이후 생성) → '' 초기화
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

// ── 치환 엔진 ────────────────────────────────────────────────

fn hash_content(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

fn apply_rules(content: &str, rules: &[ReplaceRule]) -> String {
    let mut result = content.to_string();
    for rule in rules.iter().filter(|r| r.enabled) {
        result = result.replace(&rule.old_text, &rule.new_text);
    }
    result
}

fn apply_rules_cached(content: &str, rules: &[ReplaceRule], cache: &mut ReplaceCache) -> String {
    if content.is_empty() {
        return String::new();
    }
    let version = cache.ruleset_version;
    let key = hash_content(content);
    if let Some((result, v)) = cache.entries.get(&key) {
        if *v == version {
            return result.clone();
        }
    }
    let result = apply_rules(content, rules);
    cache.entries.insert(key, (result.clone(), version));
    result
}

fn detect_conflicts(rules: &[ReplaceRule]) -> HashMap<i64, Vec<i64>> {
    let mut conflicts: HashMap<i64, Vec<i64>> = HashMap::new();
    let n = rules.len();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let ri = &rules[i];
            let rj = &rules[j];
            let is_cycle = ri.old_text == rj.new_text && ri.new_text == rj.old_text;
            let is_cascade =
                !rj.old_text.is_empty() && ri.new_text.contains(rj.old_text.as_str());
            if is_cycle || is_cascade {
                conflicts.entry(ri.id).or_default().push(rj.id);
            }
        }
    }
    conflicts
}

fn fetch_rules_from_db(conn: &Connection) -> Result<Vec<ReplaceRule>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, old_text, new_text, enabled, priority, created_at, updated_at
             FROM ReplaceRule ORDER BY priority ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;

    let rules = stmt
        .query_map([], |row| {
            Ok(ReplaceRule {
                id: row.get(0)?,
                old_text: row.get(1)?,
                new_text: row.get(2)?,
                enabled: row.get::<_, i64>(3)? != 0,
                priority: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                conflicts: vec![],
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rules)
}

fn get_records_for_scope(
    conn: &Connection,
    scope_type: &str,
    area_id: Option<i64>,
) -> Result<Vec<RecordCell>, String> {
    match scope_type {
        "all" => {
            let mut stmt = conn
                .prepare(
                    "SELECT activity_id, student_id, content
                     FROM ActivityRecord WHERE content != ''",
                )
                .map_err(|e| e.to_string())?;
            let records = stmt
                .query_map([], |row| {
                    Ok(RecordCell {
                        activity_id: row.get(0)?,
                        student_id: row.get(1)?,
                        content: row.get(2)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(records)
        }
        "area" => {
            let aid =
                area_id.ok_or_else(|| "area 범위에는 area_id가 필요합니다.".to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT ar.activity_id, ar.student_id, ar.content
                     FROM ActivityRecord ar
                     JOIN AreaActivity aa ON aa.activity_id = ar.activity_id
                     WHERE aa.area_id = ?1 AND ar.content != ''",
                )
                .map_err(|e| e.to_string())?;
            let records = stmt
                .query_map(rusqlite::params![aid], |row| {
                    Ok(RecordCell {
                        activity_id: row.get(0)?,
                        student_id: row.get(1)?,
                        content: row.get(2)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(records)
        }
        _ => Err(format!("알 수 없는 scope_type: {scope_type}")),
    }
}

// ── 치환 규칙 커맨드 ─────────────────────────────────────────

#[tauri::command]
fn get_replace_rules(state: State<DbState>) -> Result<Vec<ReplaceRule>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let mut rules = fetch_rules_from_db(conn)?;
    let conflicts = detect_conflicts(&rules);
    for rule in rules.iter_mut() {
        if let Some(ids) = conflicts.get(&rule.id) {
            rule.conflicts = ids.clone();
        }
    }
    Ok(rules)
}

#[tauri::command]
fn create_replace_rule(
    old_text: String,
    new_text: String,
    priority: i64,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
) -> Result<ReplaceRule, String> {
    if old_text.trim().is_empty() {
        return Err("찾을 텍스트를 입력해주세요.".to_string());
    }
    if old_text == new_text {
        return Err("찾을 텍스트와 바꿀 텍스트가 동일합니다.".to_string());
    }

    let rule = {
        let guard = state.0.lock().unwrap();
        let conn = guard
            .as_ref()
            .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

        conn.execute(
            "INSERT INTO ReplaceRule (old_text, new_text, priority) VALUES (?1, ?2, ?3)",
            rusqlite::params![old_text, new_text, priority],
        )
        .map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, old_text, new_text, enabled, priority, created_at, updated_at
             FROM ReplaceRule WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(ReplaceRule {
                    id: row.get(0)?,
                    old_text: row.get(1)?,
                    new_text: row.get(2)?,
                    enabled: row.get::<_, i64>(3)? != 0,
                    priority: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    conflicts: vec![],
                })
            },
        )
        .map_err(|e| e.to_string())?
    };

    cache.lock().unwrap().ruleset_version += 1;
    Ok(rule)
}

#[tauri::command]
fn update_replace_rule(
    id: i64,
    old_text: String,
    new_text: String,
    enabled: bool,
    priority: i64,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
) -> Result<ReplaceRule, String> {
    if old_text.trim().is_empty() {
        return Err("찾을 텍스트를 입력해주세요.".to_string());
    }
    if old_text == new_text {
        return Err("찾을 텍스트와 바꿀 텍스트가 동일합니다.".to_string());
    }

    let rule = {
        let guard = state.0.lock().unwrap();
        let conn = guard
            .as_ref()
            .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

        let enabled_int: i64 = if enabled { 1 } else { 0 };
        conn.execute(
            "UPDATE ReplaceRule
             SET old_text=?1, new_text=?2, enabled=?3, priority=?4,
                 updated_at=datetime('now','localtime')
             WHERE id=?5",
            rusqlite::params![old_text, new_text, enabled_int, priority, id],
        )
        .map_err(|e| e.to_string())?;

        conn.query_row(
            "SELECT id, old_text, new_text, enabled, priority, created_at, updated_at
             FROM ReplaceRule WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(ReplaceRule {
                    id: row.get(0)?,
                    old_text: row.get(1)?,
                    new_text: row.get(2)?,
                    enabled: row.get::<_, i64>(3)? != 0,
                    priority: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    conflicts: vec![],
                })
            },
        )
        .map_err(|e| e.to_string())?
    };

    cache.lock().unwrap().ruleset_version += 1;
    Ok(rule)
}

#[tauri::command]
fn delete_replace_rule(
    id: i64,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute("DELETE FROM ReplaceRule WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;

    drop(guard);
    cache.lock().unwrap().ruleset_version += 1;
    Ok(())
}

#[tauri::command]
fn preview_replace(
    scope_type: String,
    area_id: Option<i64>,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
) -> Result<Vec<ReplacePreviewItem>, String> {
    // Phase 1: DB에서 규칙·기록·이름 수집 (DB 락 보유)
    let (rules, records, act_names, stu_names) = {
        let guard = state.0.lock().unwrap();
        let conn = guard
            .as_ref()
            .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

        let rules = fetch_rules_from_db(conn)?;
        let records = get_records_for_scope(conn, &scope_type, area_id)?;

        let mut act_names: HashMap<i64, String> = HashMap::new();
        let mut stu_names: HashMap<i64, String> = HashMap::new();
        for rec in &records {
            act_names.entry(rec.activity_id).or_insert_with(|| {
                conn.query_row(
                    "SELECT name FROM Activity WHERE id=?1",
                    rusqlite::params![rec.activity_id],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| format!("활동#{}", rec.activity_id))
            });
            stu_names.entry(rec.student_id).or_insert_with(|| {
                conn.query_row(
                    "SELECT name FROM Student WHERE id=?1",
                    rusqlite::params![rec.student_id],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| format!("학생#{}", rec.student_id))
            });
        }
        (rules, records, act_names, stu_names)
    }; // DB 락 해제

    // Phase 2: 캐시를 이용해 치환 결과 계산
    let mut cache_guard = cache.lock().unwrap();
    let mut items = Vec::new();

    for rec in &records {
        let result = apply_rules_cached(&rec.content, &rules, &mut cache_guard);
        if result == rec.content {
            continue;
        }
        let activity_name = act_names
            .get(&rec.activity_id)
            .cloned()
            .unwrap_or_default();
        let student_name = stu_names
            .get(&rec.student_id)
            .cloned()
            .unwrap_or_default();

        items.push(ReplacePreviewItem {
            activity_id: rec.activity_id,
            student_id: rec.student_id,
            activity_name,
            student_name,
            original: rec.content.clone(),
            result,
        });
    }

    Ok(items)
}

#[tauri::command]
fn apply_replace(
    scope_type: String,
    area_id: Option<i64>,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
) -> Result<ReplaceApplyResult, String> {
    // Phase 1: 규칙·기록 수집 (DB 락 해제 후 캐시 계산)
    let (rules, records) = {
        let guard = state.0.lock().unwrap();
        let conn = guard
            .as_ref()
            .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
        let rules = fetch_rules_from_db(conn)?;
        let records = get_records_for_scope(conn, &scope_type, area_id)?;
        (rules, records)
    };

    let total_count = records.len() as i64;

    // Phase 2: 변경 목록 계산 (캐시 락만 보유)
    let changes: Vec<(i64, i64, String)> = {
        let mut cache_guard = cache.lock().unwrap();
        records
            .iter()
            .filter_map(|rec| {
                let result = apply_rules_cached(&rec.content, &rules, &mut cache_guard);
                if result != rec.content {
                    Some((rec.activity_id, rec.student_id, result))
                } else {
                    None
                }
            })
            .collect()
    };

    let changed_count = changes.len() as i64;
    if changes.is_empty() {
        return Ok(ReplaceApplyResult { changed_count: 0, total_count });
    }

    // Phase 3: 트랜잭션으로 DB에 반영
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result: Result<(), String> = (|| {
        for (activity_id, student_id, new_content) in &changes {
            conn.execute(
                "INSERT INTO ActivityRecord (activity_id, student_id, content, updated_at)
                 VALUES (?1, ?2, ?3, datetime('now'))
                 ON CONFLICT(activity_id, student_id) DO UPDATE SET
                   content = excluded.content,
                   updated_at = excluded.updated_at",
                rusqlite::params![activity_id, student_id, new_content],
            )
            .map_err(|e| e.to_string())?;

            conn.execute(
                "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
                 SELECT r.id, r.content, r.updated_at, '치환 적용'
                 FROM ActivityRecord r
                 WHERE r.activity_id = ?1 AND r.student_id = ?2
                   AND NOT EXISTS (
                       SELECT 1 FROM ActivityRecordHistory h
                       WHERE h.activity_record_id = r.id
                         AND h.changed_at = r.updated_at
                   )",
                rusqlite::params![activity_id, student_id],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();

    match result {
        Ok(_) => {
            conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            Ok(ReplaceApplyResult { changed_count, total_count })
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

// ── 파일 유틸 ────────────────────────────────────────────────

#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_bytes_file(path: String, data: String) -> Result<(), String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())
}

// ── 유의어 점검 커맨드 ────────────────────────────────────────

#[tauri::command]
fn get_synonym_groups(state: State<DbState>) -> Result<Vec<SynonymGroupFull>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT sg.id, sg.name, sg.created_at, si.id, si.word
             FROM SynonymGroup sg
             LEFT JOIN SynonymItem si ON sg.id = si.group_id
             ORDER BY sg.id, si.id",
        )
        .map_err(|e| e.to_string())?;

    let mut groups: Vec<SynonymGroupFull> = Vec::new();
    let mut index_map: HashMap<i64, usize> = HashMap::new();

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (gid, gname, created_at, item_id, word) = row.map_err(|e| e.to_string())?;
        let idx = if let Some(&i) = index_map.get(&gid) {
            i
        } else {
            let i = groups.len();
            groups.push(SynonymGroupFull { id: gid, name: gname, created_at, items: vec![] });
            index_map.insert(gid, i);
            i
        };
        if let (Some(id), Some(w)) = (item_id, word) {
            groups[idx].items.push(SynonymWordItem { id, group_id: gid, word: w });
        }
    }

    Ok(groups)
}

#[tauri::command]
fn create_synonym_group(name: String, state: State<DbState>) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    conn.execute("INSERT INTO SynonymGroup (name) VALUES (?1)", [&name])
        .map_err(|e| unique_err(&e, "이미 존재하는 그룹명입니다."))?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn delete_synonym_group(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    conn.execute("DELETE FROM SynonymGroup WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn add_synonym_word(group_id: i64, word: String, state: State<DbState>) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    conn.execute(
        "INSERT INTO SynonymItem (group_id, word) VALUES (?1, ?2)",
        rusqlite::params![group_id, word],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn delete_synonym_word(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    conn.execute("DELETE FROM SynonymItem WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn seed_default_synonyms(groups: Vec<SeedGroupInput>, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM SynonymGroup", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    if count > 0 {
        return Ok(());
    }

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    for group in &groups {
        conn.execute("INSERT INTO SynonymGroup (name) VALUES (?1)", [&group.name])
            .map_err(|e| {
                let _ = conn.execute_batch("ROLLBACK");
                e.to_string()
            })?;
        let gid = conn.last_insert_rowid();
        for word in &group.words {
            conn.execute(
                "INSERT INTO SynonymItem (group_id, word) VALUES (?1, ?2)",
                rusqlite::params![gid, word],
            )
            .map_err(|e| {
                let _ = conn.execute_batch("ROLLBACK");
                e.to_string()
            })?;
        }
    }
    conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_all_records_for_inspect(state: State<DbState>) -> Result<Vec<InspectRecord>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT ar.id, act.name, s.name, COALESCE(a.name, '') AS area_name,
                    s.grade, s.class_num, s.number, ar.content
             FROM ActivityRecord ar
             JOIN Activity act ON ar.activity_id = act.id
             JOIN Student s ON ar.student_id = s.id
             LEFT JOIN AreaActivity aa ON act.id = aa.activity_id
             LEFT JOIN Area a ON aa.area_id = a.id
             WHERE ar.content != ''
             GROUP BY ar.id
             ORDER BY a.id, act.id, s.grade, s.class_num, s.number",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(InspectRecord {
                id:            row.get(0)?,
                activity_name: row.get(1)?,
                student_name:  row.get(2)?,
                area_name:     row.get(3)?,
                grade:         row.get(4)?,
                class_num:     row.get(5)?,
                number:        row.get(6)?,
                content:       row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

// ── 앱 진입점 ────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(DbState(Mutex::new(None)))
        .manage(Mutex::new(ReplaceCache {
            ruleset_version: 0,
            entries: HashMap::new(),
        }))
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
            get_students,
            create_student,
            update_student,
            delete_student,
            bulk_upsert_students,
            get_area_students,
            set_area_students,
            get_area_grid,
            upsert_record,
            get_record_history,
            save_history_snapshot,
            bulk_import_records,
            preview_import_records,
            write_text_file,
            write_bytes_file,
            create_snapshot,
            get_snapshots,
            restore_snapshot,
            get_replace_rules,
            create_replace_rule,
            update_replace_rule,
            delete_replace_rule,
            preview_replace,
            apply_replace,
            get_synonym_groups,
            create_synonym_group,
            delete_synonym_group,
            add_synonym_word,
            delete_synonym_word,
            seed_default_synonyms,
            get_all_records_for_inspect,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ── 단위 테스트 ──────────────────────────────────────────────

#[cfg(test)]
mod replace_tests {
    use super::*;

    fn make_rule(id: i64, old: &str, new: &str, enabled: bool, priority: i64) -> ReplaceRule {
        ReplaceRule {
            id,
            old_text: old.to_string(),
            new_text: new.to_string(),
            enabled,
            priority,
            created_at: String::new(),
            updated_at: String::new(),
            conflicts: vec![],
        }
    }

    #[test]
    fn test_sequential_apply() {
        // A→B (rule 0), B→C (rule 1): 최종 결과 C
        let rules = vec![
            make_rule(1, "A", "B", true, 0),
            make_rule(2, "B", "C", true, 1),
        ];
        assert_eq!(apply_rules("A", &rules), "C");
    }

    #[test]
    fn test_quote_replacement() {
        let rules = vec![
            make_rule(1, "\u{201C}", "'", true, 0),
            make_rule(2, "\u{201D}", "'", true, 1),
            make_rule(3, "\u{2018}", "'", true, 2),
            make_rule(4, "\u{2019}", "'", true, 3),
        ];
        let input = "\u{201C}안녕\u{201D} \u{2018}반갑\u{2019}";
        assert_eq!(apply_rules(input, &rules), "'안녕' '반갑'");
    }

    #[test]
    fn test_disabled_rule_skipped() {
        let rules = vec![
            make_rule(1, "hello", "world", false, 0),
            make_rule(2, "foo", "bar", true, 1),
        ];
        assert_eq!(apply_rules("hello foo", &rules), "hello bar");
    }

    #[test]
    fn test_cache_invalidation_on_version_change() {
        let mut cache = ReplaceCache { ruleset_version: 0, entries: HashMap::new() };
        let rules_v0 = vec![make_rule(1, "A", "B", true, 0)];
        assert_eq!(apply_rules_cached("A", &rules_v0, &mut cache), "B");

        cache.ruleset_version += 1;
        let rules_v1 = vec![make_rule(1, "A", "C", true, 0)];
        assert_eq!(apply_rules_cached("A", &rules_v1, &mut cache), "C");
    }

    #[test]
    fn test_conflict_cycle_detection() {
        let rules = vec![
            make_rule(1, "A", "B", true, 0),
            make_rule(2, "B", "A", true, 1),
        ];
        let conflicts = detect_conflicts(&rules);
        assert!(conflicts.contains_key(&1));
        assert!(conflicts.contains_key(&2));
    }

    #[test]
    fn test_empty_content_returns_empty() {
        let rules = vec![make_rule(1, "A", "B", true, 0)];
        assert_eq!(apply_rules_cached("", &rules, &mut ReplaceCache {
            ruleset_version: 0, entries: HashMap::new()
        }), "");
    }

    #[test]
    fn test_no_match_returns_original() {
        let rules = vec![make_rule(1, "X", "Y", true, 0)];
        assert_eq!(apply_rules("hello", &rules), "hello");
    }
}
