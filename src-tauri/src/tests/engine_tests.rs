use crate::engine::{
    apply_rules, apply_rules_cached, detect_conflicts, fetch_rules_from_db,
    get_records_for_scope, validate_rules,
};
use crate::state::{ReplaceCache, MAX_CACHE_ENTRIES};
use crate::types::ReplaceRule;
use std::collections::HashMap;

use super::{insert_activity, insert_area, insert_record, insert_student, setup_test_db};

fn make_rule(id: i64, old: &str, new: &str, enabled: bool, priority: i64) -> ReplaceRule {
    ReplaceRule {
        id,
        old_text: old.to_string(),
        new_text: new.to_string(),
        is_regex: false,
        enabled,
        priority,
        created_at: String::new(),
        updated_at: String::new(),
        conflicts: vec![],
    }
}

fn make_regex_rule(id: i64, old: &str, new: &str, enabled: bool, priority: i64) -> ReplaceRule {
    ReplaceRule {
        id,
        old_text: old.to_string(),
        new_text: new.to_string(),
        is_regex: true,
        enabled,
        priority,
        created_at: String::new(),
        updated_at: String::new(),
        conflicts: vec![],
    }
}

// ── apply_rules ───────────────────────────────────────────────

#[test]
fn test_apply_rules_basic_literal() {
    let rules = vec![make_rule(1, "hello", "world", true, 0)];
    assert_eq!(apply_rules("hello", &rules), "world");
}

#[test]
fn test_apply_rules_sequential_chaining() {
    let rules = vec![
        make_rule(1, "A", "B", true, 0),
        make_rule(2, "B", "C", true, 1),
    ];
    assert_eq!(apply_rules("A", &rules), "C");
}

#[test]
fn test_apply_rules_multiple_occurrences() {
    let rules = vec![make_rule(1, "A", "B", true, 0)];
    assert_eq!(apply_rules("AAA", &rules), "BBB");
}

#[test]
fn test_apply_rules_disabled_rule_skipped() {
    let rules = vec![
        make_rule(1, "hello", "world", false, 0),
        make_rule(2, "foo", "bar", true, 1),
    ];
    assert_eq!(apply_rules("hello foo", &rules), "hello bar");
}

#[test]
fn test_apply_rules_empty_rules() {
    let rules: Vec<ReplaceRule> = vec![];
    assert_eq!(apply_rules("unchanged", &rules), "unchanged");
}

#[test]
fn test_apply_rules_no_match() {
    let rules = vec![make_rule(1, "X", "Y", true, 0)];
    assert_eq!(apply_rules("hello", &rules), "hello");
}

#[test]
fn test_apply_rules_regex_basic() {
    let rules = vec![make_regex_rule(1, r"\d+", "N", true, 0)];
    assert_eq!(apply_rules("abc123", &rules), "abcN");
}

#[test]
fn test_apply_rules_invalid_regex_skipped() {
    let rules = vec![make_regex_rule(1, r"[invalid(", "x", true, 0)];
    // 잘못된 정규식: 패닉 없이 원본 반환
    assert_eq!(apply_rules("hello", &rules), "hello");
}

#[test]
fn test_apply_rules_korean_quotes() {
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
fn test_apply_rules_follows_slice_order_not_priority() {
    // apply_rules는 priority를 보지 않고 받은 순서대로 적용한다.
    // priority 정렬은 fetch_rules_from_db의 몫이다
    // (test_fetch_rules_ordered_by_priority_then_old_text).
    // 일부러 priority가 큰 규칙을 앞에 둔다 — 앞 규칙이 "foo"를 먼저 바꾸므로
    // 뒤 규칙은 적용될 대상이 없다.
    let rules = vec![
        make_rule(2, "foo", "baz", true, 1),
        make_rule(1, "foo", "bar", true, 0),
    ];
    assert_eq!(apply_rules("foo", &rules), "baz");
}

// ── apply_rules_cached ───────────────────────────────────────

#[test]
fn test_cached_stores_result() {
    let mut cache = ReplaceCache { ruleset_version: 0, entries: HashMap::new() };
    let rules = vec![make_rule(1, "A", "B", true, 0)];
    let _ = apply_rules_cached("A", &rules, &mut cache);
    assert!(cache.entries.contains_key("A"));
}

#[test]
fn test_cached_returns_cached_on_hit() {
    let mut cache = ReplaceCache { ruleset_version: 0, entries: HashMap::new() };
    let rules = vec![make_rule(1, "A", "B", true, 0)];
    let r1 = apply_rules_cached("A", &rules, &mut cache);
    // 버전은 그대로, 규칙만 바꿔도 캐시 값 반환
    let rules2 = vec![make_rule(1, "A", "C", true, 0)];
    let r2 = apply_rules_cached("A", &rules2, &mut cache);
    assert_eq!(r1, "B");
    assert_eq!(r2, "B");
}

#[test]
fn test_cached_invalidates_on_version_change() {
    let mut cache = ReplaceCache { ruleset_version: 0, entries: HashMap::new() };
    let rules_v0 = vec![make_rule(1, "A", "B", true, 0)];
    assert_eq!(apply_rules_cached("A", &rules_v0, &mut cache), "B");

    cache.ruleset_version += 1;
    let rules_v1 = vec![make_rule(1, "A", "C", true, 0)];
    assert_eq!(apply_rules_cached("A", &rules_v1, &mut cache), "C");
}

#[test]
fn test_cached_empty_string() {
    let rules = vec![make_rule(1, "A", "B", true, 0)];
    let result = apply_rules_cached("", &rules, &mut ReplaceCache {
        ruleset_version: 0,
        entries: HashMap::new(),
    });
    assert_eq!(result, "");
}

// ── detect_conflicts ─────────────────────────────────────────

#[test]
fn test_conflict_cycle_ab_ba() {
    let rules = vec![
        make_rule(1, "A", "B", true, 0),
        make_rule(2, "B", "A", true, 1),
    ];
    let conflicts = detect_conflicts(&rules);
    assert!(conflicts.contains_key(&1));
    assert!(conflicts.contains_key(&2));
}

#[test]
fn test_conflict_cascade() {
    // Rule1.new_text "XY" contains Rule2.old_text "Y" → cascade
    let rules = vec![
        make_rule(1, "A", "XY", true, 0),
        make_rule(2, "Y", "Z", true, 1),
    ];
    let conflicts = detect_conflicts(&rules);
    assert!(conflicts.contains_key(&1), "Rule1이 Rule2와 cascade 충돌해야 함");
}

#[test]
fn test_no_conflict_unrelated_rules() {
    let rules = vec![
        make_rule(1, "A", "B", true, 0),
        make_rule(2, "C", "D", true, 1),
    ];
    let conflicts = detect_conflicts(&rules);
    assert!(conflicts.is_empty());
}

#[test]
fn test_no_conflict_for_regex_rules() {
    // is_regex=true 규칙은 충돌 검사에서 제외
    let rules = vec![
        make_regex_rule(1, "A", "B", true, 0),
        make_rule(2, "B", "A", true, 1),
    ];
    let conflicts = detect_conflicts(&rules);
    assert!(conflicts.is_empty());
}


// ── fetch_rules_from_db ───────────────────────────────────────

#[test]
fn test_fetch_rules_empty_db() {
    let conn = setup_test_db();
    let rules = fetch_rules_from_db(&conn).unwrap();
    assert!(rules.is_empty());
}

#[test]
fn test_fetch_rules_ordered_by_priority_then_old_text() {
    let conn = setup_test_db();
    conn.execute_batch(
        "INSERT INTO ReplaceRule (old_text, new_text, priority) VALUES ('B', 'x', 0);
         INSERT INTO ReplaceRule (old_text, new_text, priority) VALUES ('A', 'x', 1);
         INSERT INTO ReplaceRule (old_text, new_text, priority) VALUES ('C', 'x', 0);",
    )
    .unwrap();
    let rules = fetch_rules_from_db(&conn).unwrap();
    assert_eq!(rules.len(), 3);
    // priority=0 먼저: B, C (old_text ASC) → 그 다음 priority=1: A
    assert_eq!(rules[0].old_text, "B");
    assert_eq!(rules[1].old_text, "C");
    assert_eq!(rules[2].old_text, "A");
}

// ── get_records_for_scope ─────────────────────────────────────

#[test]
fn test_scope_all_returns_nonempty_records() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "영역1", 500);
    let act_id = insert_activity(&conn, "활동1");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    insert_record(&conn, act_id, stu_id, "내용있음");

    let records = get_records_for_scope(&conn, "all", &[], None).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "내용있음");
}

#[test]
fn test_scope_all_excludes_empty_content() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "활동1");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    insert_record(&conn, act_id, stu_id, ""); // 빈 content

    let records = get_records_for_scope(&conn, "all", &[], None).unwrap();
    assert!(records.is_empty());
}

#[test]
fn test_scope_areas_filters_by_area() {
    let conn = setup_test_db();
    let area1 = insert_area(&conn, "영역1", 500);
    let area2 = insert_area(&conn, "영역2", 500);
    let act1 = insert_activity(&conn, "활동1");
    let act2 = insert_activity(&conn, "활동2");
    let stu = insert_student(&conn, 1, 1, 1, "홍길동");

    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area1, act1],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area2, act2],
    )
    .unwrap();
    // 학생이 두 영역 모두에 등록돼 있어야 활동 기준 필터만 검증된다
    for area in [area1, area2] {
        conn.execute(
            "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
            rusqlite::params![area, stu],
        )
        .unwrap();
    }

    insert_record(&conn, act1, stu, "영역1 기록");
    insert_record(&conn, act2, stu, "영역2 기록");

    let records = get_records_for_scope(&conn, "areas", &[area1], None).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "영역1 기록");
}

/// 활동이 영역에 속해도 그 영역에 등록되지 않은 학생의 기록은 범위 밖이다.
/// 점검(get_all_records_for_inspect)·빠른 교체·기록 그리드와 같은 기준이어야
/// 사용자가 고른 영역보다 넓게 치환되지 않는다.
#[test]
fn test_scope_areas_excludes_students_not_in_area() {
    let conn = setup_test_db();
    let area = insert_area(&conn, "영역", 500);
    let act = insert_activity(&conn, "활동");
    let registered = insert_student(&conn, 1, 1, 1, "등록학생");
    let unregistered = insert_student(&conn, 1, 1, 2, "미등록학생");

    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area, act],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area, registered],
    )
    .unwrap();

    insert_record(&conn, act, registered, "등록 기록");
    insert_record(&conn, act, unregistered, "미등록 기록");

    let records = get_records_for_scope(&conn, "areas", &[area], None).unwrap();
    assert_eq!(records.len(), 1, "영역에 등록되지 않은 학생의 기록은 제외되어야 함");
    assert_eq!(records[0].student_id, registered);
}

#[test]
fn test_scope_areas_requires_membership_in_the_same_area() {
    // 활동이 두 영역에 걸쳐 있어도, 학생은 **조회하는 그 영역**에 등록돼 있어야 한다.
    let conn = setup_test_db();
    let area_a = insert_area(&conn, "영역A", 500);
    let area_b = insert_area(&conn, "영역B", 500);
    let act = insert_activity(&conn, "공통 활동");
    let stu = insert_student(&conn, 1, 1, 1, "A에만 등록");
    conn.execute_batch(&format!(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES ({area_a}, {act}), ({area_b}, {act});
         INSERT INTO AreaStudent (area_id, student_id) VALUES ({area_a}, {stu});"
    ))
    .unwrap();
    insert_record(&conn, act, stu, "기록");

    let records = get_records_for_scope(&conn, "areas", &[area_b], None).unwrap();
    assert!(records.is_empty(), "영역B에 등록되지 않은 학생의 기록이 들어왔다");
}

#[test]
fn test_scope_areas_dedupes_record_shared_by_two_areas() {
    // 같은 기록이 두 영역을 통해 잡혀도 한 번만 나와야 한다.
    // 중복되면 미리보기에 두 번 뜨고 치환 대상 수가 부풀려진다.
    let conn = setup_test_db();
    let area_a = insert_area(&conn, "영역A", 500);
    let area_b = insert_area(&conn, "영역B", 500);
    let act = insert_activity(&conn, "공통 활동");
    let stu = insert_student(&conn, 1, 1, 1, "양쪽 등록");
    conn.execute_batch(&format!(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES ({area_a}, {act}), ({area_b}, {act});
         INSERT INTO AreaStudent (area_id, student_id) VALUES ({area_a}, {stu}), ({area_b}, {stu});"
    ))
    .unwrap();
    insert_record(&conn, act, stu, "기록");

    let records = get_records_for_scope(&conn, "areas", &[area_a, area_b], None).unwrap();
    assert_eq!(records.len(), 1);
}

#[test]
fn test_scope_areas_empty_ids_returns_empty() {
    let conn = setup_test_db();
    let records = get_records_for_scope(&conn, "areas", &[], None).unwrap();
    assert!(records.is_empty());
}

#[test]
fn test_scope_unknown_returns_error() {
    let conn = setup_test_db();
    let result = get_records_for_scope(&conn, "invalid_scope", &[], None);
    assert!(result.is_err());
}

#[test]
fn test_invalidate_clears_entries_and_bumps_version() {
    let mut cache = ReplaceCache { ruleset_version: 0, entries: HashMap::new() };
    let rules = vec![make_rule(1, "A", "B", true, 0)];
    let _ = apply_rules_cached("A", &rules, &mut cache);

    cache.invalidate();

    // 버전만 올리면 옛 항목은 조회되지 않을 뿐 평문이 메모리에 그대로 남는다.
    assert_eq!(cache.ruleset_version, 1);
    assert!(cache.entries.is_empty(), "invalidate는 평문을 실제로 버려야 한다");
}

#[test]
fn test_cached_clears_when_entry_limit_reached() {
    let mut cache = ReplaceCache { ruleset_version: 0, entries: HashMap::new() };
    let rules = vec![make_rule(1, "A", "B", true, 0)];
    for i in 0..MAX_CACHE_ENTRIES {
        let _ = apply_rules_cached(&format!("내용{i}"), &rules, &mut cache);
    }
    assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);

    // 상한에 도달한 뒤 한 건 더 넣으면 통째로 비우고 새 항목만 남는다.
    let _ = apply_rules_cached("마지막", &rules, &mut cache);
    assert_eq!(cache.entries.len(), 1);
    assert!(cache.entries.contains_key("마지막"));
}

// ── validate_rules ───────────────────────────────────────────

#[test]
fn test_validate_rules_accepts_valid_regex() {
    let rules = vec![make_regex_rule(1, r"\d+", "N", true, 0)];
    assert!(validate_rules(&rules).is_ok());
}

#[test]
fn test_validate_rules_rejects_invalid_regex() {
    let rules = vec![make_regex_rule(1, r"[invalid(", "x", true, 0)];
    let err = validate_rules(&rules).unwrap_err();
    assert!(
        err.contains("[invalid("),
        "어떤 패턴이 문제인지 알려줘야 한다: {err}"
    );
}

#[test]
fn test_validate_rules_ignores_disabled_and_literal_rules() {
    // 꺼진 규칙은 적용되지 않고, 리터럴 규칙은 정규식으로 해석되지 않는다.
    let rules = vec![
        make_regex_rule(1, r"[invalid(", "x", false, 0),
        make_rule(2, r"[invalid(", "x", true, 1),
    ];
    assert!(validate_rules(&rules).is_ok());
}

// ── unique_backup_path (감사 F2) ──────────────────────────────
//
// 예전에는 파일명이 분 단위였고 fs::copy가 덮어썼다. 잘못된 가져오기나 복원 뒤
// 1분 안에 다시 열면 사고 직전 백업이 사고 이후 상태로 교체됐다.

use crate::engine::unique_backup_path;

fn temp_dir_with(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ubp_{}_{}_{}", name, std::process::id(), nanos));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_unique_backup_path_never_returns_existing_file() {
    let dir = temp_dir_with("exists");
    let src = dir.join("학생부.db");
    std::fs::write(&src, b"db").unwrap();

    let first = unique_backup_path(&src, "").unwrap();
    std::fs::write(&first, b"backup1").unwrap();

    // 같은 초에 다시 요청해도 기존 백업을 가리켜서는 안 된다.
    let second = unique_backup_path(&src, "").unwrap();
    assert_ne!(first, second, "기존 백업을 덮어쓸 경로를 돌려주면 안 된다");
    assert!(!second.exists(), "돌려준 경로는 아직 없는 파일이어야 한다");

    // 첫 백업 내용이 그대로 남아 있어야 한다.
    assert_eq!(std::fs::read(&first).unwrap(), b"backup1");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_unique_backup_path_uses_second_granularity() {
    let dir = temp_dir_with("secs");
    let src = dir.join("학생부.db");
    std::fs::write(&src, b"db").unwrap();

    let p = unique_backup_path(&src, "").unwrap();
    let name = p.file_name().unwrap().to_string_lossy().to_string();
    // 학생부.YYMMDD-HHMMSS.db.backup — 타임스탬프가 12자리(초 포함)여야 한다.
    let ts = name.split('.').nth(1).unwrap();
    assert_eq!(ts.len(), 13, "YYMMDD-HHMMSS 형식이어야 한다: {name}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_unique_backup_path_keeps_suffix() {
    let dir = temp_dir_with("suffix");
    let src = dir.join("학생부.db");
    std::fs::write(&src, b"db").unwrap();

    let p = unique_backup_path(&src, "-pre-encrypt").unwrap();
    let name = p.file_name().unwrap().to_string_lossy().to_string();
    assert!(name.contains("-pre-encrypt"), "접미사가 유지되어야 한다: {name}");
    assert!(name.ends_with(".db.backup"));
    std::fs::remove_dir_all(&dir).ok();
}
