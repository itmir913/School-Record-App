use crate::engine::{
    apply_rules, apply_rules_cached, detect_conflicts, fetch_rules_from_db,
    get_records_for_scope, hash_content,
};
use crate::state::ReplaceCache;
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
fn test_apply_rules_priority_order() {
    // priority=0 규칙이 먼저 적용되어 "foo"→"bar"로 변환되면
    // priority=1 규칙 "foo"→"baz"는 이미 "bar"가 되었으므로 적용 안 됨
    let rules = vec![
        make_rule(1, "foo", "bar", true, 0),
        make_rule(2, "foo", "baz", true, 1),
    ];
    assert_eq!(apply_rules("foo", &rules), "bar");
}

// ── apply_rules_cached ───────────────────────────────────────

#[test]
fn test_cached_stores_result() {
    let mut cache = ReplaceCache { ruleset_version: 0, entries: HashMap::new() };
    let rules = vec![make_rule(1, "A", "B", true, 0)];
    let _ = apply_rules_cached("A", &rules, &mut cache);
    let key = hash_content("A");
    assert!(cache.entries.contains_key(&key));
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

// ── hash_content ─────────────────────────────────────────────

#[test]
fn test_hash_content_deterministic() {
    let h1 = hash_content("hello");
    let h2 = hash_content("hello");
    assert_eq!(h1, h2);
}

#[test]
fn test_hash_content_different_inputs() {
    let h1 = hash_content("hello");
    let h2 = hash_content("world");
    assert_ne!(h1, h2);
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

    insert_record(&conn, act1, stu, "영역1 기록");
    insert_record(&conn, act2, stu, "영역2 기록");

    let records = get_records_for_scope(&conn, "areas", &[area1], None).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "영역1 기록");
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
