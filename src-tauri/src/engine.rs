use regex::Regex;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use crate::crypto::maybe_decrypt;
use crate::state::{ReplaceCache, MAX_CACHE_ENTRIES};
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

/// 아직 존재하지 않는 백업 파일 경로를 만든다.
///
/// 예전에는 파일명이 분 단위(`%y%m%d-%H%M`)였고 `fs::copy`가 기존 파일을 덮어썼다.
/// 잘못된 가져오기나 복원을 한 뒤 1분 안에 다시 열면, 사고 직전의 백업이 사고 이후
/// 상태로 조용히 교체됐다 — 앱의 유일한 안전망이 사라지는 것이다.
///
/// 초 단위로 낮추고, 그래도 겹치면 일련번호를 붙여 **절대 덮어쓰지 않는다.**
pub fn unique_backup_path(src: &Path, suffix: &str) -> Result<PathBuf, String> {
    let parent = src
        .parent()
        .ok_or("DB 파일의 상위 디렉토리를 찾을 수 없습니다.")?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("backup");
    let ts = chrono::Local::now().format("%y%m%d-%H%M%S").to_string();

    let first = parent.join(format!("{stem}.{ts}{suffix}.db.backup"));
    if !first.exists() {
        return Ok(first);
    }
    // 같은 초에 두 번 이상 여는 경우. 무한정 시도하지 않고 상한을 둔다.
    for n in 2..=99 {
        let candidate = parent.join(format!("{stem}.{ts}-{n}{suffix}.db.backup"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("백업 파일 이름을 만들지 못했습니다. 같은 이름의 백업이 너무 많습니다.".to_string())
}

pub fn validate_parent_dir_path(path: &str, missing_parent_message: &str) -> Result<(), String> {
    let p = validate_absolute_path_without_parent_dir(path)?;
    p.parent()
        .ok_or_else(|| "유효하지 않은 경로입니다.".to_string())?
        .canonicalize()
        .map_err(|_| missing_parent_message.to_string())?;
    Ok(())
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

/// 규칙 목록의 정규식이 모두 컴파일되는지 검사한다.
///
/// `apply_rules`는 컴파일에 실패한 규칙을 조용히 건너뛴다. 그대로 두면 사용자는
/// 치환이 적용된 줄 알게 되므로, 실제로 적용하기 전에 여기서 걸러 알린다.
pub fn validate_rules(rules: &[ReplaceRule]) -> Result<(), String> {
    for rule in rules.iter().filter(|r| r.enabled && r.is_regex) {
        Regex::new(&rule.old_text).map_err(|e| {
            format!(
                "정규식 규칙이 올바르지 않습니다 (패턴 '{}'): {e}",
                rule.old_text
            )
        })?;
    }
    Ok(())
}

pub fn apply_rules_cached(content: &str, rules: &[ReplaceRule], cache: &mut ReplaceCache) -> String {
    if content.is_empty() {
        return String::new();
    }
    let version = cache.ruleset_version;
    if let Some((result, v)) = cache.entries.get(content) {
        if *v == version {
            return result.clone();
        }
    }
    let result = apply_rules(content, rules);
    // 상한을 넘으면 통째로 비운다. 캐시는 복호화된 평문을 그대로 담고 있어서
    // 그냥 두면 작업량에 비례해 평문이 메모리에 계속 쌓인다.
    if cache.entries.len() >= MAX_CACHE_ENTRIES {
        cache.entries.clear();
    }
    cache.entries.insert(content.to_string(), (result.clone(), version));
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
            // 영역에 등록된 학생의 기록만 대상이다. 점검·빠른 교체·기록 그리드와 같은
            // 기준이어야 한다 — 활동만으로 거르면 같은 활동을 쓰는 다른 영역 학생이나
            // 영역에서 뺀 학생의 기록까지 치환된다.
            let sql = format!(
                "SELECT ar.activity_id, ar.student_id, ar.content
                 FROM ActivityRecord ar
                 JOIN AreaActivity aa ON aa.activity_id = ar.activity_id
                 JOIN AreaStudent ast ON ast.student_id = ar.student_id AND ast.area_id = aa.area_id
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
