use crate::commands::crypto::resolve_data_key;
use crate::commands::student::validate_student_identity;
use crate::db::with_transaction;
use crate::crypto::{maybe_decrypt, maybe_encrypt};
use crate::state::{CryptoStateHandle, DbState};
use crate::types::{
    ActivityItem, ActivityRecordItem, AreaGridData, BulkImportResult, HistoryEntry,
    ImportRecordInput, PreviewImportItem, RecordCell, StudentItem,
};
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;
use tauri::State;

pub fn get_area_grid_impl(
    conn: &Connection,
    area_id: i64,
    key: Option<[u8; 32]>,
) -> Result<AreaGridData, String> {
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

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.grade, s.class_num, s.number, s.name
             FROM Student s
             JOIN AreaStudent as_ ON s.id = as_.student_id
             WHERE as_.area_id = ?1
             ORDER BY s.grade, s.class_num, s.number",
        )
        .map_err(|e| e.to_string())?;

    let raw_students = stmt
        .query_map(rusqlite::params![area_id], |row| {
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

    let mut students = Vec::with_capacity(raw_students.len());
    for (id, grade, class_num, number, name) in raw_students {
        students.push(StudentItem {
            id,
            grade,
            class_num,
            number,
            name: maybe_decrypt(name, key)?,
        });
    }

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
        let params: Vec<i64> = activity_ids
            .iter()
            .chain(student_ids.iter())
            .copied()
            .collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let raw_rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut rows = Vec::with_capacity(raw_rows.len());
        for (activity_id, student_id, content) in raw_rows {
            rows.push(RecordCell {
                activity_id,
                student_id,
                content: maybe_decrypt(content, key)?,
            });
        }
        rows
    };

    Ok(AreaGridData {
        activities,
        students,
        records,
    })
}

#[tauri::command]
pub fn get_area_grid(
    area_id: i64,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<AreaGridData, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    get_area_grid_impl(conn, area_id, key)
}

pub fn upsert_record_impl(
    conn: &Connection,
    activity_id: i64,
    student_id: i64,
    content: &str,
    key: Option<[u8; 32]>,
) -> Result<(), String> {
    let stored = maybe_encrypt(content, key)?;
    conn.execute(
        "INSERT INTO ActivityRecord (activity_id, student_id, content, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(activity_id, student_id) DO UPDATE SET
           content = excluded.content,
           updated_at = excluded.updated_at",
        rusqlite::params![activity_id, student_id, stored],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn upsert_record(
    activity_id: i64,
    student_id: i64,
    content: String,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    upsert_record_impl(conn, activity_id, student_id, &content, key)
}

pub fn get_record_history_impl(
    conn: &Connection,
    activity_id: i64,
    student_id: i64,
    limit: i64,
    offset: i64,
    key: Option<[u8; 32]>,
) -> Result<Vec<HistoryEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT h.id, h.content, h.changed_at, h.note
             FROM ActivityRecordHistory h
             JOIN ActivityRecord r ON r.id = h.activity_record_id
             WHERE r.activity_id = ?1 AND r.student_id = ?2
             ORDER BY h.id DESC
             LIMIT ?3 OFFSET ?4",
        )
        .map_err(|e| e.to_string())?;

    let raw = stmt
        .query_map(
            rusqlite::params![activity_id, student_id, limit, offset],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut entries = Vec::with_capacity(raw.len());
    for (id, content, changed_at, note) in raw {
        entries.push(HistoryEntry {
            id,
            content: maybe_decrypt(content, key)?,
            changed_at,
            // note도 v2부터 암호화 대상이다. content만 풀고 note를 그대로 두면
            // 화면에 암호문이 그대로 찍힌다.
            note: note.map(|n| maybe_decrypt(n, key)).transpose()?,
        });
    }
    Ok(entries)
}

#[tauri::command]
pub fn get_record_history(
    activity_id: i64,
    student_id: i64,
    limit: i64,
    offset: i64,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<Vec<HistoryEntry>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    get_record_history_impl(conn, activity_id, student_id, limit, offset, key)
}

/// 암호화 파일에서 기록의 현재 내용을 히스토리에 남기되, 가장 최근 히스토리와
/// **평문이 같으면** 남기지 않는다. 삽입한 행 수(0 또는 1)를 돌려준다.
///
/// 평문 파일은 SQL의 `h.content = r.content`로 충분하지만, 암호문은 매번 다른
/// nonce로 만들어져 같은 평문이라도 저장된 값이 다르다(같은 내용 재저장, 재가져오기,
/// 암호화 전환·비밀번호 변경의 일괄 재암호화). 그래서 복호화해서 비교한다.
/// `stored_note`는 이미 암호화된 값이다.
pub(crate) fn insert_history_if_changed_encrypted(
    conn: &Connection,
    record_id: i64,
    current_raw: &str,
    stored_note: Option<&str>,
    key: [u8; 32],
) -> Result<usize, String> {
    let latest: Option<String> = conn
        .query_row(
            "SELECT content FROM ActivityRecordHistory
             WHERE activity_record_id = ?1
             ORDER BY id DESC LIMIT 1",
            rusqlite::params![record_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some(latest) = latest {
        let same = latest == current_raw
            || maybe_decrypt(latest, Some(key))?
                == maybe_decrypt(current_raw.to_string(), Some(key))?;
        if same {
            return Ok(0);
        }
    }

    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         SELECT id, content, updated_at, ?2 FROM ActivityRecord WHERE id = ?1",
        rusqlite::params![record_id, stored_note],
    )
    .map_err(|e| e.to_string())
}

/// `insert_history_if_changed_encrypted`를 (활동, 학생) 셀 단위로 부른다.
/// 기록이 아직 없으면 남길 것도 없으므로 0이다.
fn insert_cell_history_if_changed_encrypted(
    conn: &Connection,
    activity_id: i64,
    student_id: i64,
    stored_note: Option<&str>,
    key: [u8; 32],
) -> Result<usize, String> {
    let record: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, content FROM ActivityRecord WHERE activity_id = ?1 AND student_id = ?2",
            rusqlite::params![activity_id, student_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    match record {
        None => Ok(0),
        Some((record_id, content)) => {
            insert_history_if_changed_encrypted(conn, record_id, &content, stored_note, key)
        }
    }
}

/// 히스토리에 현재 내용을 남긴다. `note`는 평문으로 받아 저장 직전에 암호화한다.
///
/// 호출부가 넘기는 note는 사용자가 직접 타이핑한 메모이거나 앱이 붙이는 고정 문자열
/// ("가져오기 전" 등)이다. 둘을 구분하지 않고 전부 암호화한다 — 한 컬럼에 평문과
/// 암호문이 섞이면 decrypt_all_data가 실패해 암호화 해제가 막힌다(crypto.rs 참고).
pub fn save_snapshot_internal(
    conn: &Connection,
    activity_id: i64,
    student_id: i64,
    note: Option<&str>,
    key: Option<[u8; 32]>,
) -> Result<(), String> {
    let stored_note = note.map(|n| maybe_encrypt(n, key)).transpose()?;

    let inserted = match key {
        None => conn
            .execute(
                "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
                 SELECT r.id, r.content, r.updated_at, ?3
                 FROM ActivityRecord r
                 WHERE r.activity_id = ?1 AND r.student_id = ?2
                   AND NOT EXISTS (
                       SELECT 1 FROM ActivityRecordHistory h
                       WHERE h.id = (SELECT MAX(h2.id) FROM ActivityRecordHistory h2
                                     WHERE h2.activity_record_id = r.id)
                         AND h.content = r.content
                   )",
                rusqlite::params![activity_id, student_id, stored_note],
            )
            .map_err(|e| e.to_string())?,
        Some(k) => {
            insert_cell_history_if_changed_encrypted(conn, activity_id, student_id, stored_note.as_deref(), k)?
        }
    };

    // **의도된 동작이다(CLAUDE.md).** 내용이 그대로면 같은 내용의 행을 또 쌓지 않고,
    // 가장 최근 행의 note만 이번 작업 이름으로 갱신한다. note는 "이 상태가 마지막으로
    // 어떤 작업 직전에 보존됐는가"를 가리키므로 최신 작업 이름이 맞다. 그 행의 note가
    // 사용자가 직접 쓴 메모였다면 덮인다 — 감사에서 "메모 손실"로 반복 보고되는
    // 지점이지만 바꾸지 말 것.
    if inserted == 0 {
        conn.execute(
            "UPDATE ActivityRecordHistory SET note = COALESCE(?3, note)
             WHERE id = (
                 SELECT MAX(h.id) FROM ActivityRecordHistory h
                 JOIN ActivityRecord r ON r.id = h.activity_record_id
                 WHERE r.activity_id = ?1 AND r.student_id = ?2
             )",
            rusqlite::params![activity_id, student_id, stored_note],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_history_snapshot(
    activity_id: i64,
    student_id: i64,
    note: Option<String>,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    save_snapshot_internal(conn, activity_id, student_id, note.as_deref(), key)
}

pub fn bulk_import_records_impl(
    conn: &Connection,
    records: &[ImportRecordInput],
    key: Option<[u8; 32]>,
) -> Result<BulkImportResult, String> {
    let mut students_created: i64 = 0;
    let mut students_updated: i64 = 0;
    let mut records_saved: i64 = 0;
    let mut student_cache: HashMap<(i64, i64, i64), i64> = HashMap::new();

    for r in records.iter() {
        validate_student_identity(r.grade, r.class_num, r.number)?;
        let cache_key = (r.grade, r.class_num, r.number);

        if !student_cache.contains_key(&cache_key) {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM Student WHERE grade=?1 AND class_num=?2 AND number=?3",
                    rusqlite::params![r.grade, r.class_num, r.number],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?
                > 0;

            if exists {
                let mut name_updated = false;
                if let Some(ref n) = r.name {
                    if !n.is_empty() {
                        let existing_name_raw: String = conn
                            .query_row(
                                "SELECT name FROM Student WHERE grade=?1 AND class_num=?2 AND number=?3",
                                rusqlite::params![r.grade, r.class_num, r.number],
                                |row| row.get(0),
                            )
                            .map_err(|e| e.to_string())?;
                        let existing_name = maybe_decrypt(existing_name_raw, key)?;
                        if existing_name.trim().is_empty() {
                            let stored_name = maybe_encrypt(n, key)?;
                            conn.execute(
                                "UPDATE Student SET name = ?1 WHERE grade=?2 AND class_num=?3 AND number=?4",
                                rusqlite::params![stored_name, r.grade, r.class_num, r.number],
                            )
                            .map_err(|e| e.to_string())?;
                            name_updated = true;
                        }
                    }
                }
                if name_updated {
                    students_updated += 1;
                }
            } else {
                let name = r.name.as_deref().unwrap_or("이름 없음");
                let stored_name = maybe_encrypt(name, key)?;
                conn.execute(
                    "INSERT INTO Student (grade, class_num, number, name) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![r.grade, r.class_num, r.number, stored_name],
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

            student_cache.insert(cache_key, student_id);
        }

        let &student_id = student_cache
            .get(&cache_key)
            .ok_or_else(|| "캐시 오류".to_string())?;
        let stored_content = maybe_encrypt(&r.content, key)?;

        // 덮어쓰기 **전에** 기존 내용을 히스토리에 남긴다.
        // apply_replace·bulk_quick_replace는 이미 그렇게 하는데 가져오기만 빠져 있었다.
        // 셀 편집은 히스토리를 만들지 않는 설계(CLAUDE.md)이므로, 손으로 고친 뒤
        // 재가져오기를 하면 그 내용이 복구 수단 없이 사라졌다.
        // 해당 기록이 아직 없으면 남길 것도 없으므로 no-op이 된다.
        save_snapshot_internal(conn, r.activity_id, student_id, Some("가져오기 전"), key)?;

        conn.execute(
            "INSERT INTO ActivityRecord (activity_id, student_id, content, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(activity_id, student_id) DO UPDATE SET
               content = excluded.content,
               updated_at = excluded.updated_at",
            rusqlite::params![r.activity_id, student_id, stored_content],
        )
        .map_err(|e| e.to_string())?;

        if !r.content.is_empty() {
            // note를 SQL 리터럴로 박아두면 암호화를 거치지 않는다. 바인딩해서 넘긴다.
            let import_note = maybe_encrypt("import", key)?;
            match key {
                None => {
                    conn.execute(
                        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
                         SELECT r.id, r.content, r.updated_at, ?3
                         FROM ActivityRecord r
                         WHERE r.activity_id = ?1 AND r.student_id = ?2
                           AND NOT EXISTS (
                               SELECT 1 FROM ActivityRecordHistory h
                               WHERE h.id = (SELECT MAX(h2.id) FROM ActivityRecordHistory h2
                                             WHERE h2.activity_record_id = r.id)
                                 AND h.content = r.content
                           )",
                        rusqlite::params![r.activity_id, student_id, import_note],
                    )
                    .map_err(|e| e.to_string())?;
                }
                Some(k) => {
                    insert_cell_history_if_changed_encrypted(
                        conn,
                        r.activity_id,
                        student_id,
                        Some(import_note.as_str()),
                        k,
                    )?;
                }
            }
        }
        records_saved += 1;
    }

    Ok(BulkImportResult {
        students_created,
        students_updated,
        records_saved,
    })
}

#[tauri::command]
pub fn bulk_import_records(
    records: Vec<ImportRecordInput>,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<BulkImportResult, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;

    with_transaction(conn, || bulk_import_records_impl(conn, &records, key))
}

pub fn preview_import_records_impl(
    conn: &Connection,
    records: &[ImportRecordInput],
    key: Option<[u8; 32]>,
) -> Result<Vec<PreviewImportItem>, String> {
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

        // 미리보기 단계에서 걸러야 사용자가 적용 직전이 아니라 지금 알 수 있다.
        validate_student_identity(r.grade, r.class_num, r.number)?;
        let cache_key = (r.grade, r.class_num, r.number);
        let student_info = if let Some(cached) = student_cache.get(&cache_key) {
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
            let info = info
                .map(|(id, enc_name)| maybe_decrypt(enc_name, key).map(|name| (id, name)))
                .transpose()?;
            student_cache.insert(cache_key, info.clone());
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
                let plain_content = match content {
                    Some(c) => maybe_decrypt(c, key)?,
                    None => String::new(),
                };
                (name, plain_content)
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

#[tauri::command]
pub fn preview_import_records(
    records: Vec<ImportRecordInput>,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<Vec<PreviewImportItem>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    preview_import_records_impl(conn, &records, key)
}

pub fn get_activity_records_impl(
    conn: &Connection,
    activity_id: i64,
    key: Option<[u8; 32]>,
) -> Result<Vec<ActivityRecordItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.grade, s.class_num, s.number, s.name, ar.content
             FROM ActivityRecord ar
             JOIN Student s ON s.id = ar.student_id
             WHERE ar.activity_id = ?1
               AND ar.content != ''
             ORDER BY s.grade, s.class_num, s.number",
        )
        .map_err(|e| e.to_string())?;

    let raw = stmt
        .query_map(rusqlite::params![activity_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut items = Vec::with_capacity(raw.len());
    for (student_id, grade, class_num, number, name, content) in raw {
        items.push(ActivityRecordItem {
            student_id,
            grade,
            class_num,
            number,
            student_name: maybe_decrypt(name, key)?,
            content: maybe_decrypt(content, key)?,
        });
    }
    Ok(items)
}

#[tauri::command]
pub fn get_activity_records(
    activity_id: i64,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<Vec<ActivityRecordItem>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    get_activity_records_impl(conn, activity_id, key)
}

pub fn bulk_quick_replace_impl(
    conn: &Connection,
    area_id: i64,
    search_text: &str,
    replace_with: &str,
    key: Option<[u8; 32]>,
) -> Result<i64, String> {
    let mut stmt = conn
        .prepare(
            "SELECT r.activity_id, r.student_id, r.content
             FROM ActivityRecord r
             JOIN AreaActivity  aa  ON aa.activity_id  = r.activity_id
             JOIN AreaStudent   as_ ON as_.student_id  = r.student_id
                                   AND as_.area_id     = aa.area_id
             WHERE aa.area_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(i64, i64, String)> = stmt
        .query_map(rusqlite::params![area_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut changed = 0i64;
    for (activity_id, student_id, raw_content) in rows {
        let content = maybe_decrypt(raw_content, key)?;
        if content.contains(search_text) {
            save_snapshot_internal(conn, activity_id, student_id, Some("빠른 텍스트 교체"), key)?;
            let new_content = content.replace(search_text, replace_with);
            upsert_record_impl(conn, activity_id, student_id, &new_content, key)?;
            changed += 1;
        }
    }
    Ok(changed)
}

#[tauri::command]
pub fn bulk_quick_replace(
    area_id: i64,
    search_text: String,
    replace_with: String,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<i64, String> {
    if search_text.trim().is_empty() {
        return Err("찾을 텍스트가 비어 있습니다.".to_string());
    }
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;

    with_transaction(conn, || {
        bulk_quick_replace_impl(conn, area_id, &search_text, &replace_with, key)
    })
}
