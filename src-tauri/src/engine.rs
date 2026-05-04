use regex::Regex;
use rusqlite::Connection;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path};

use crate::crypto::maybe_decrypt;
use crate::state::ReplaceCache;
use crate::types::{RecordCell, ReplaceRule};

fn validate_absolute_path_without_parent_dir(path: &str) -> Result<&Path, String> {
    let p = Path::new(path);
    if !p.is_absolute() {
        return Err("절대 경로만 허용됩니다.".to_string());
    }
    for component in p.components() {
        if component == Component::ParentDir {
            return Err("경로에 '..'이 포함될 수 없습니다.".to_string());
        }
    }
    Ok(p)
}

pub fn validate_existing_path(path: &str, not_found_message: &str) -> Result<(), String> {
    let p = validate_absolute_path_without_parent_dir(path)?;
    p.canonicalize()
        .map_err(|_| not_found_message.to_string())?;
    Ok(())
}

pub fn validate_parent_dir_path(path: &str, missing_parent_message: &str) -> Result<(), String> {
    let p = validate_absolute_path_without_parent_dir(path)?;
    p.parent()
        .ok_or_else(|| "유효하지 않은 경로입니다.".to_string())?
        .canonicalize()
        .map_err(|_| missing_parent_message.to_string())?;
    Ok(())
}

pub fn hash_content(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

pub fn apply_rules(content: &str, rules: &[ReplaceRule]) -> String {
    let mut result = content.to_string();
    for rule in rules.iter().filter(|r| r.enabled) {
        if rule.is_regex {
            if let Ok(re) = Regex::new(&rule.old_text) {
                result = re.replace_all(&result, rule.new_text.as_str()).to_string();
            }
        } else {
            result = result.replace(&rule.old_text, &rule.new_text);
        }
    }
    result
}

pub fn apply_rules_cached(content: &str, rules: &[ReplaceRule], cache: &mut ReplaceCache) -> String {
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

pub fn detect_conflicts(rules: &[ReplaceRule]) -> HashMap<i64, Vec<i64>> {
    let mut conflicts: HashMap<i64, Vec<i64>> = HashMap::new();
    let n = rules.len();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let ri = &rules[i];
            let rj = &rules[j];
            if !ri.enabled || !rj.enabled {
                continue;
            }
            if ri.is_regex || rj.is_regex {
                continue;
            }
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

pub fn fetch_rules_from_db(conn: &Connection) -> Result<Vec<ReplaceRule>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, old_text, new_text, is_regex, enabled, priority, created_at, updated_at
             FROM ReplaceRule ORDER BY priority ASC, old_text ASC, new_text ASC",
        )
        .map_err(|e| e.to_string())?;

    let rules = stmt
        .query_map([], |row| {
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
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rules)
}

pub fn get_records_for_scope(
    conn: &Connection,
    scope_type: &str,
    area_ids: &[i64],
    key: Option<[u8; 32]>,
) -> Result<Vec<RecordCell>, String> {
    match scope_type {
        "all" => {
            let mut stmt = conn
                .prepare(
                    "SELECT activity_id, student_id, content
                     FROM ActivityRecord WHERE content != ''",
                )
                .map_err(|e| e.to_string())?;
            let raw = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            let mut result = Vec::with_capacity(raw.len());
            for (activity_id, student_id, content) in raw {
                result.push(RecordCell {
                    activity_id,
                    student_id,
                    content: maybe_decrypt(content, key)?,
                });
            }
            Ok(result)
        }
        "areas" => {
            if area_ids.is_empty() {
                return Ok(vec![]);
            }
            let placeholders = area_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!(
                "SELECT ar.activity_id, ar.student_id, ar.content
                 FROM ActivityRecord ar
                 JOIN AreaActivity aa ON aa.activity_id = ar.activity_id
                 WHERE aa.area_id IN ({placeholders}) AND ar.content != ''
                 GROUP BY ar.activity_id, ar.student_id"
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let raw = stmt
                .query_map(rusqlite::params_from_iter(area_ids.iter()), |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            let mut result = Vec::with_capacity(raw.len());
            for (activity_id, student_id, content) in raw {
                result.push(RecordCell {
                    activity_id,
                    student_id,
                    content: maybe_decrypt(content, key)?,
                });
            }
            Ok(result)
        }
        _ => Err(format!("알 수 없는 scope_type: {scope_type}")),
    }
}
