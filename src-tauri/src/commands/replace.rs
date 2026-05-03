use crate::commands::crypto::resolve_data_key;
use crate::crypto::{maybe_decrypt, maybe_encrypt};
use crate::engine::{
    apply_rules_cached, detect_conflicts, fetch_rules_from_db, get_records_for_scope,
};
use crate::state::{CryptoStateHandle, DbState, ReplaceCacheState};
use crate::types::{ReplaceApplyResult, ReplacePreviewItem, ReplaceRule};
use regex::Regex;
use rusqlite::Connection;
use std::collections::HashMap;
use tauri::State;

pub fn validate_replace_rule(old_text: &str, new_text: &str, is_regex: bool) -> Result<(), String> {
    if old_text.trim().is_empty() {
        return Err("찾을 텍스트를 입력해주세요.".to_string());
    }
    if old_text == new_text {
        return Err("찾을 텍스트와 바꿀 텍스트가 동일합니다.".to_string());
    }
    if is_regex {
        Regex::new(old_text).map_err(|e| format!("정규식 오류: {}", e))?;
    }
    Ok(())
}

pub fn get_replace_rules_impl(conn: &Connection) -> Result<Vec<ReplaceRule>, String> {
    let mut rules = fetch_rules_from_db(conn)?;
    let conflicts = detect_conflicts(&rules);
    for rule in rules.iter_mut() {
        if let Some(ids) = conflicts.get(&rule.id) {
            rule.conflicts = ids.clone();
        }
    }
    Ok(rules)
}

pub fn create_replace_rule_db(
    conn: &Connection,
    old_text: &str,
    new_text: &str,
    is_regex: bool,
    priority: i64,
) -> Result<ReplaceRule, String> {
    let is_regex_int: i64 = if is_regex { 1 } else { 0 };
    conn.execute(
        "INSERT OR IGNORE INTO ReplaceRule (old_text, new_text, is_regex, priority) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![old_text, new_text, is_regex_int, priority],
    )
    .map_err(|e| e.to_string())?;

    if conn.changes() == 0 {
        return Err("이미 동일한 규칙이 존재합니다.".to_string());
    }

    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, old_text, new_text, is_regex, enabled, priority, created_at, updated_at
         FROM ReplaceRule WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(ReplaceRule {
                id: row.get(0)?,
                old_text: row.get(1)?,
                new_text: row.get(2)?,
                is_regex: row.get::<_, i64>(3)? != 0,
                enabled: row.get::<_, i64>(4)? != 0,
                priority: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                conflicts: vec![],
            })
        },
    )
    .map_err(|e| e.to_string())
}

pub fn update_replace_rule_db(
    conn: &Connection,
    id: i64,
    old_text: &str,
    new_text: &str,
    is_regex: bool,
    enabled: bool,
    priority: i64,
) -> Result<ReplaceRule, String> {
    let enabled_int: i64 = if enabled { 1 } else { 0 };
    let is_regex_int: i64 = if is_regex { 1 } else { 0 };
    conn.execute(
        "UPDATE ReplaceRule
         SET old_text=?1, new_text=?2, is_regex=?3, enabled=?4, priority=?5,
             updated_at=datetime('now')
         WHERE id=?6",
        rusqlite::params![old_text, new_text, is_regex_int, enabled_int, priority, id],
    )
    .map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT id, old_text, new_text, is_regex, enabled, priority, created_at, updated_at
         FROM ReplaceRule WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(ReplaceRule {
                id: row.get(0)?,
                old_text: row.get(1)?,
                new_text: row.get(2)?,
                is_regex: row.get::<_, i64>(3)? != 0,
                enabled: row.get::<_, i64>(4)? != 0,
                priority: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                conflicts: vec![],
            })
        },
    )
    .map_err(|e| e.to_string())
}

pub fn delete_replace_rule_impl(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute(
        "DELETE FROM ReplaceRule WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn seed_default_replace_rules_impl(
    conn: &Connection,
    rules: &[serde_json::Value],
) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ReplaceRule", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    if count > 0 {
        return Ok(());
    }

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    for rule in rules {
        let old_text = rule["oldText"].as_str().ok_or("oldText 누락")?;
        let new_text = rule["newText"].as_str().ok_or("newText 누락")?;
        let priority = rule["priority"].as_i64().ok_or("priority 누락")?;
        let is_regex: i64 = if rule["isRegex"].as_bool().unwrap_or(false) {
            1
        } else {
            0
        };
        conn.execute(
            "INSERT OR IGNORE INTO ReplaceRule (old_text, new_text, is_regex, priority) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![old_text, new_text, is_regex, priority],
        )
        .map_err(|e| {
            let _ = conn.execute_batch("ROLLBACK");
            e.to_string()
        })?;
    }
    conn.execute_batch("COMMIT").map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_replace_rules(state: State<DbState>) -> Result<Vec<ReplaceRule>, String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    get_replace_rules_impl(conn)
}

#[tauri::command]
pub fn create_replace_rule(
    old_text: String,
    new_text: String,
    is_regex: bool,
    priority: i64,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
) -> Result<ReplaceRule, String> {
    validate_replace_rule(&old_text, &new_text, is_regex)?;

    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let rule = create_replace_rule_db(conn, &old_text, &new_text, is_regex, priority)?;
    drop(guard);
    cache.lock().unwrap().ruleset_version += 1;
    Ok(rule)
}

#[tauri::command]
pub fn update_replace_rule(
    id: i64,
    old_text: String,
    new_text: String,
    is_regex: bool,
    enabled: bool,
    priority: i64,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
) -> Result<ReplaceRule, String> {
    validate_replace_rule(&old_text, &new_text, is_regex)?;

    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    let rule = update_replace_rule_db(conn, id, &old_text, &new_text, is_regex, enabled, priority)?;
    drop(guard);
    cache.lock().unwrap().ruleset_version += 1;
    Ok(rule)
}

#[tauri::command]
pub fn delete_replace_rule(
    id: i64,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    delete_replace_rule_impl(conn, id)?;
    drop(guard);
    cache.lock().unwrap().ruleset_version += 1;
    Ok(())
}

#[tauri::command]
pub fn seed_default_replace_rules(
    rules: Vec<serde_json::Value>,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    seed_default_replace_rules_impl(conn, &rules)?;
    drop(guard);
    cache.lock().unwrap().ruleset_version += 1;
    Ok(())
}

#[tauri::command]
pub fn preview_replace(
    scope_type: String,
    area_ids: Vec<i64>,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
    crypto: State<CryptoStateHandle>,
) -> Result<Vec<ReplacePreviewItem>, String> {
    let (rules, records, act_names, stu_names) = {
        let guard = state.0.lock().unwrap();
        let conn = guard
            .as_ref()
            .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
        let key = resolve_data_key(conn, &crypto)?;

        let rules = fetch_rules_from_db(conn)?;
        let records = get_records_for_scope(conn, &scope_type, &area_ids, key)?;

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
            if !stu_names.contains_key(&rec.student_id) {
                let raw_name: String = conn
                    .query_row(
                        "SELECT name FROM Student WHERE id=?1",
                        rusqlite::params![rec.student_id],
                        |r| r.get(0),
                    )
                    .unwrap_or_else(|_| format!("학생#{}", rec.student_id));
                let name = maybe_decrypt(raw_name, key)?;
                stu_names.insert(rec.student_id, name);
            }
        }
        (rules, records, act_names, stu_names)
    };

    let mut cache_guard = cache.lock().unwrap();
    let mut items = Vec::new();

    for rec in &records {
        let result = apply_rules_cached(&rec.content, &rules, &mut cache_guard);
        if result == rec.content {
            continue;
        }
        let activity_name = act_names.get(&rec.activity_id).cloned().unwrap_or_default();
        let student_name = stu_names.get(&rec.student_id).cloned().unwrap_or_default();

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
pub fn apply_replace(
    scope_type: String,
    area_ids: Vec<i64>,
    state: State<DbState>,
    cache: State<ReplaceCacheState>,
    crypto: State<CryptoStateHandle>,
) -> Result<ReplaceApplyResult, String> {
    let (key, rules, records) = {
        let guard = state.0.lock().unwrap();
        let conn = guard
            .as_ref()
            .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
        let key = resolve_data_key(conn, &crypto)?;
        let rules = fetch_rules_from_db(conn)?;
        let records = get_records_for_scope(conn, &scope_type, &area_ids, key)?;
        (key, rules, records)
    };

    let total_count = records.len() as i64;

    // changes contains (activity_id, student_id, plain_new_content)
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
        return Ok(ReplaceApplyResult {
            changed_count: 0,
            total_count,
        });
    }

    let guard = state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result: Result<(), String> = (|| {
        for (activity_id, student_id, plain_content) in &changes {
            let stored = maybe_encrypt(plain_content, key)?;
            conn.execute(
                "INSERT INTO ActivityRecord (activity_id, student_id, content, updated_at)
                 VALUES (?1, ?2, ?3, datetime('now'))
                 ON CONFLICT(activity_id, student_id) DO UPDATE SET
                   content = excluded.content,
                   updated_at = excluded.updated_at",
                rusqlite::params![activity_id, student_id, stored],
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
            Ok(ReplaceApplyResult {
                changed_count,
                total_count,
            })
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}
