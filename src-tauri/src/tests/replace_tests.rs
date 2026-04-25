use crate::commands::replace::{
    create_replace_rule_db, delete_replace_rule_impl, get_replace_rules_impl,
    seed_default_replace_rules_impl, update_replace_rule_db, validate_replace_rule,
};
use super::setup_test_db;

// ── validate_replace_rule (순수 함수) ──────────────────────────

#[test]
fn test_validate_empty_old_text_error() {
    let err = validate_replace_rule("", "world", false).unwrap_err();
    assert!(err.contains("찾을 텍스트"), "에러 메시지: {err}");
}

#[test]
fn test_validate_whitespace_only_old_text_error() {
    let err = validate_replace_rule("   ", "world", false).unwrap_err();
    assert!(err.contains("찾을 텍스트"), "에러 메시지: {err}");
}

#[test]
fn test_validate_same_old_new_text_error() {
    let err = validate_replace_rule("abc", "abc", false).unwrap_err();
    assert!(err.contains("동일"), "에러 메시지: {err}");
}

#[test]
fn test_validate_invalid_regex_error() {
    let err = validate_replace_rule("[invalid", "world", true).unwrap_err();
    assert!(err.contains("정규식 오류"), "에러 메시지: {err}");
}

#[test]
fn test_validate_valid_regex_ok() {
    let result = validate_replace_rule(r"\d+", "N", true);
    assert!(result.is_ok());
}

#[test]
fn test_validate_literal_same_old_new_is_error() {
    let err = validate_replace_rule("hello", "hello", false).unwrap_err();
    assert!(err.contains("동일"), "에러 메시지: {err}");
}

// ── DB 연동 테스트 ─────────────────────────────────────────────

#[test]
fn test_create_rule_persists_to_db() {
    let conn = setup_test_db();
    let rule = create_replace_rule_db(&conn, "hello", "world", false, 0).unwrap();
    assert!(rule.id > 0);
    assert_eq!(rule.old_text, "hello");
    assert_eq!(rule.new_text, "world");
    assert!(!rule.is_regex);
    assert!(rule.enabled);
    assert_eq!(rule.priority, 0);
}

#[test]
fn test_create_rule_duplicate_returns_error() {
    let conn = setup_test_db();
    create_replace_rule_db(&conn, "hello", "world", false, 0).unwrap();
    let err = create_replace_rule_db(&conn, "hello", "world", false, 0).unwrap_err();
    assert!(err.contains("동일한 규칙"), "에러 메시지: {err}");
}

#[test]
fn test_get_replace_rules_includes_conflict_ids() {
    let conn = setup_test_db();
    // "AA" → "BB" 후 "BB" → "CC" 연쇄 충돌
    let rule1 = create_replace_rule_db(&conn, "AA", "BB", false, 0).unwrap();
    let rule2 = create_replace_rule_db(&conn, "BB", "CC", false, 1).unwrap();

    let rules = get_replace_rules_impl(&conn).unwrap();
    let r1 = rules.iter().find(|r| r.id == rule1.id).unwrap();
    assert!(
        r1.conflicts.contains(&rule2.id),
        "rule1.conflicts = {:?}, rule2.id = {}",
        r1.conflicts,
        rule2.id
    );
}

#[test]
fn test_get_replace_rules_ordered_by_priority_then_old_text() {
    let conn = setup_test_db();
    create_replace_rule_db(&conn, "beta", "X", false, 1).unwrap();
    create_replace_rule_db(&conn, "alpha", "Y", false, 1).unwrap();
    create_replace_rule_db(&conn, "zeta", "Z", false, 0).unwrap();

    let rules = get_replace_rules_impl(&conn).unwrap();
    assert_eq!(rules[0].old_text, "zeta", "priority=0이 먼저");
    assert_eq!(rules[1].old_text, "alpha", "priority=1, old_text 알파벳순 alpha");
    assert_eq!(rules[2].old_text, "beta", "priority=1, old_text 알파벳순 beta");
}

#[test]
fn test_update_rule_changes_all_fields() {
    let conn = setup_test_db();
    let rule = create_replace_rule_db(&conn, "old", "new", false, 0).unwrap();

    let updated = update_replace_rule_db(&conn, rule.id, "OLD2", "NEW2", true, false, 5).unwrap();

    assert_eq!(updated.old_text, "OLD2");
    assert_eq!(updated.new_text, "NEW2");
    assert!(updated.is_regex);
    assert!(!updated.enabled);
    assert_eq!(updated.priority, 5);
}

#[test]
fn test_update_rule_toggle_enabled() {
    let conn = setup_test_db();
    let rule = create_replace_rule_db(&conn, "abc", "xyz", false, 0).unwrap();
    assert!(rule.enabled);

    let updated = update_replace_rule_db(&conn, rule.id, "abc", "xyz", false, false, 0).unwrap();
    assert!(!updated.enabled);
}

#[test]
fn test_delete_rule_removes_from_db() {
    let conn = setup_test_db();
    let rule = create_replace_rule_db(&conn, "del", "gone", false, 0).unwrap();

    delete_replace_rule_impl(&conn, rule.id).unwrap();

    let rules = get_replace_rules_impl(&conn).unwrap();
    assert!(rules.is_empty());
}

#[test]
fn test_seed_default_rules_inserts_when_empty() {
    let conn = setup_test_db();
    let rules = vec![
        serde_json::json!({"oldText": "hello", "newText": "world", "priority": 0, "isRegex": false}),
        serde_json::json!({"oldText": "foo", "newText": "bar", "priority": 1, "isRegex": false}),
    ];

    seed_default_replace_rules_impl(&conn, &rules).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ReplaceRule", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_seed_default_rules_skips_when_nonempty() {
    let conn = setup_test_db();
    create_replace_rule_db(&conn, "existing", "rule", false, 0).unwrap();

    let seed_rules = vec![
        serde_json::json!({"oldText": "hello", "newText": "world", "priority": 0, "isRegex": false}),
    ];
    seed_default_replace_rules_impl(&conn, &seed_rules).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ReplaceRule", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "기존 규칙 있으면 seed 무시");
    let old_text: String = conn
        .query_row("SELECT old_text FROM ReplaceRule", [], |r| r.get(0))
        .unwrap();
    assert_eq!(old_text, "existing");
}
