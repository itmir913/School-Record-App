use crate::commands::crypto::resolve_data_key;
use crate::crypto::maybe_decrypt;
use crate::state::{unique_err, CryptoStateHandle, DbState};
use crate::types::{InspectRecord, SeedGroupInput, SynonymGroupFull, SynonymWordItem};
use rusqlite::Connection;
use std::collections::HashMap;
use tauri::State;

pub fn get_synonym_groups_impl(conn: &Connection) -> Result<Vec<SynonymGroupFull>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT sg.id, sg.name, sg.created_at, si.id, si.word
             FROM SynonymGroup sg
             LEFT JOIN SynonymItem si ON sg.id = si.group_id
             ORDER BY sg.id, si.word",
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
            groups.push(SynonymGroupFull {
                id: gid,
                name: gname,
                created_at,
                items: vec![],
            });
            index_map.insert(gid, i);
            i
        };
        if let (Some(id), Some(w)) = (item_id, word) {
            groups[idx].items.push(SynonymWordItem {
                id,
                group_id: gid,
                word: w,
            });
        }
    }

    Ok(groups)
}

pub fn create_synonym_group_impl(conn: &Connection, name: &str) -> Result<i64, String> {
    conn.execute("INSERT INTO SynonymGroup (name) VALUES (?1)", [name])
        .map_err(|e| unique_err(&e, "이미 존재하는 그룹명입니다."))?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_synonym_group_impl(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM SynonymGroup WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn add_synonym_word_impl(conn: &Connection, group_id: i64, word: &str) -> Result<i64, String> {
    conn.execute(
        "INSERT OR IGNORE INTO SynonymItem (group_id, word) VALUES (?1, ?2)",
        rusqlite::params![group_id, word],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_synonym_word_impl(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM SynonymItem WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn seed_default_synonyms_impl(
    conn: &Connection,
    groups: &[SeedGroupInput],
) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM SynonymGroup", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    if count > 0 {
        return Ok(());
    }

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    for group in groups {
        conn.execute("INSERT INTO SynonymGroup (name) VALUES (?1)", [&group.name])
            .map_err(|e| {
                let _ = conn.execute_batch("ROLLBACK");
                e.to_string()
            })?;
        let gid = conn.last_insert_rowid();
        for word in &group.words {
            conn.execute(
                "INSERT OR IGNORE INTO SynonymItem (group_id, word) VALUES (?1, ?2)",
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

pub fn get_all_records_for_inspect_impl(
    conn: &Connection,
    scope_type: &str,
    area_ids: Vec<i64>,
    key: Option<[u8; 32]>,
) -> Result<Vec<InspectRecord>, String> {
    type RawRow = (i64, String, String, String, i64, i64, i64, String);
    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<RawRow> {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
        ))
    };

    let raw: Vec<RawRow> = match scope_type {
        "all" => {
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
                .query_map([], map_row)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        }
        "areas" => {
            if area_ids.is_empty() {
                return Ok(vec![]);
            }
            let placeholders = area_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!(
                "SELECT ar.id, act.name, s.name, a.name AS area_name,
                        s.grade, s.class_num, s.number, ar.content
                 FROM AreaActivity aa
                 JOIN Area a ON a.id = aa.area_id
                 JOIN Activity act ON act.id = aa.activity_id
                 JOIN ActivityRecord ar ON ar.activity_id = aa.activity_id
                 JOIN Student s ON s.id = ar.student_id
                 JOIN AreaStudent ast ON ast.student_id = ar.student_id AND ast.area_id = aa.area_id
                 WHERE ar.content != '' AND aa.area_id IN ({placeholders})
                 GROUP BY ar.id
                 ORDER BY a.id, act.id, s.grade, s.class_num, s.number"
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params_from_iter(area_ids.iter()), map_row)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        }
        _ => return Err(format!("알 수 없는 scope_type: {scope_type}")),
    };

    let mut result = Vec::with_capacity(raw.len());
    for (id, activity_name, student_name, area_name, grade, class_num, number, content) in raw {
        result.push(InspectRecord {
            id,
            activity_name,
            student_name: maybe_decrypt(student_name, key)?,
            area_name,
            grade,
            class_num,
            number,
            content: maybe_decrypt(content, key)?,
        });
    }
    Ok(result)
}

// ── Tauri 명령어 래퍼 ──────────────────────────────────────────

#[tauri::command]
pub fn get_synonym_groups(state: State<DbState>) -> Result<Vec<SynonymGroupFull>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    get_synonym_groups_impl(conn)
}

#[tauri::command]
pub fn create_synonym_group(name: String, state: State<DbState>) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    create_synonym_group_impl(conn, &name)
}

#[tauri::command]
pub fn delete_synonym_group(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    delete_synonym_group_impl(conn, id)
}

#[tauri::command]
pub fn add_synonym_word(group_id: i64, word: String, state: State<DbState>) -> Result<i64, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    add_synonym_word_impl(conn, group_id, &word)
}

#[tauri::command]
pub fn delete_synonym_word(id: i64, state: State<DbState>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    delete_synonym_word_impl(conn, id)
}

#[tauri::command]
pub fn seed_default_synonyms(
    groups: Vec<SeedGroupInput>,
    state: State<DbState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    seed_default_synonyms_impl(conn, &groups)
}

#[tauri::command]
pub fn get_all_records_for_inspect(
    scope_type: String,
    area_ids: Vec<i64>,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<Vec<InspectRecord>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    get_all_records_for_inspect_impl(conn, &scope_type, area_ids, key)
}
