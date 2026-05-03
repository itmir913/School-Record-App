use crate::commands::crypto::resolve_data_key;
use crate::crypto::{maybe_decrypt, maybe_encrypt};
use crate::state::{unique_err, CryptoStateHandle, DbState};
use crate::types::{BulkUpsertResult, StudentInput, StudentItem};
use rusqlite::Connection;
use tauri::State;

pub fn get_students_impl(
    conn: &Connection,
    key: Option<[u8; 32]>,
) -> Result<Vec<StudentItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, grade, class_num, number, name
             FROM Student
             ORDER BY grade, class_num, number",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut students = Vec::with_capacity(rows.len());
    for (id, grade, class_num, number, name) in rows {
        students.push(StudentItem {
            id,
            grade,
            class_num,
            number,
            name: maybe_decrypt(name, key)?,
        });
    }
    Ok(students)
}

pub fn create_student_impl(
    conn: &Connection,
    grade: i64,
    class_num: i64,
    number: i64,
    name: &str,
    key: Option<[u8; 32]>,
) -> Result<i64, String> {
    let stored_name = maybe_encrypt(name, key)?;
    conn.execute(
        "INSERT INTO Student (grade, class_num, number, name) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![grade, class_num, number, stored_name],
    )
    .map_err(|e| {
        unique_err(
            &e,
            &format!("이미 같은 학번의 학생이 있습니다: {grade}학년 {class_num}반 {number}번"),
        )
    })?;
    Ok(conn.last_insert_rowid())
}

pub fn update_student_impl(
    conn: &Connection,
    id: i64,
    grade: i64,
    class_num: i64,
    number: i64,
    name: &str,
    key: Option<[u8; 32]>,
) -> Result<(), String> {
    let stored_name = maybe_encrypt(name, key)?;
    conn.execute(
        "UPDATE Student SET grade = ?1, class_num = ?2, number = ?3, name = ?4 WHERE id = ?5",
        rusqlite::params![grade, class_num, number, stored_name, id],
    )
    .map_err(|e| {
        unique_err(
            &e,
            &format!("이미 같은 학번의 학생이 있습니다: {grade}학년 {class_num}반 {number}번"),
        )
    })?;
    Ok(())
}

pub fn delete_student_impl(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM Student WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn bulk_upsert_students_impl(
    conn: &Connection,
    students: &[StudentInput],
    key: Option<[u8; 32]>,
) -> Result<BulkUpsertResult, String> {
    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<BulkUpsertResult, String> {
        let mut inserted: i64 = 0;
        let mut updated: i64 = 0;
        for s in students.iter() {
            let stored_name = maybe_encrypt(&s.name, key)?;
            conn.execute(
                "INSERT OR IGNORE INTO Student (grade, class_num, number, name)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![s.grade, s.class_num, s.number, stored_name],
            )
            .map_err(|e| e.to_string())?;

            if conn.changes() > 0 {
                inserted += 1;
            } else {
                conn.execute(
                    "UPDATE Student SET name = ?1 WHERE grade=?2 AND class_num=?3 AND number=?4",
                    rusqlite::params![stored_name, s.grade, s.class_num, s.number],
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

pub fn get_area_students_impl(conn: &Connection, area_id: i64) -> Result<Vec<i64>, String> {
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

pub fn set_area_students_impl(
    conn: &Connection,
    area_id: i64,
    student_ids: &[i64],
) -> Result<(), String> {
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

pub fn set_area_activities_impl(
    conn: &Connection,
    area_id: i64,
    activity_ids: &[i64],
) -> Result<(), String> {
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

// ── Tauri 커맨드 (얇은 래퍼) ─────────────────────────────────

#[tauri::command]
pub fn get_students(
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<Vec<StudentItem>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    get_students_impl(conn, key)
}

#[tauri::command]
pub fn create_student(
    grade: i64,
    class_num: i64,
    number: i64,
    name: String,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    create_student_impl(conn, grade, class_num, number, &name, key)
}

#[tauri::command]
pub fn update_student(
    id: i64,
    grade: i64,
    class_num: i64,
    number: i64,
    name: String,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    update_student_impl(conn, id, grade, class_num, number, &name, key)
}

#[tauri::command]
pub fn delete_student(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    delete_student_impl(conn, id)
}

#[tauri::command]
pub fn set_area_activities(
    area_id: i64,
    activity_ids: Vec<i64>,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    set_area_activities_impl(conn, area_id, &activity_ids)
}

#[tauri::command]
pub fn bulk_upsert_students(
    students: Vec<StudentInput>,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<BulkUpsertResult, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    bulk_upsert_students_impl(conn, &students, key)
}

#[tauri::command]
pub fn get_area_students(area_id: i64, state: State<DbState>) -> Result<Vec<i64>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    get_area_students_impl(conn, area_id)
}

#[tauri::command]
pub fn set_area_students(
    area_id: i64,
    student_ids: Vec<i64>,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    set_area_students_impl(conn, area_id, &student_ids)
}
