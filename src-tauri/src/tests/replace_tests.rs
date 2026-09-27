use crate::commands::replace::{
    apply_default_replace_rules_impl, apply_replace_impl, create_replace_rule_db,
    delete_replace_rule_impl, get_replace_rules_impl, preview_replace_impl, update_replace_rule_db,
    validate_replace_rule, validate_replace_rule_for_save,
};
use crate::engine::{apply_rules, fetch_rules_from_db, get_records_for_scope};
use crate::state::ReplaceCache;
use std::collections::HashMap;
use super::{insert_activity, insert_record, insert_student, setup_test_db};

/// validate_replace_rule을 우회해 DB에 직접 넣는다.
/// 구버전에서 저장되었거나 외부에서 들어온 규칙을 흉내 낸다.
fn insert_raw_regex_rule(conn: &rusqlite::Connection, pattern: &str) {
    conn.execute(
        "INSERT INTO ReplaceRule (old_text, new_text, is_regex, priority) VALUES (?1, 'x', 1, 0)",
        rusqlite::params![pattern],
    )
    .unwrap();
}

fn empty_cache() -> ReplaceCache {
    ReplaceCache {
        ruleset_version: 0,
        entries: HashMap::new(),
    }
}

/// unwrap_err는 Ok 타입에 Debug를 요구한다. 반환 타입에 Debug를 붙이지 않기 위해 직접 꺼낸다.
fn expect_err<T>(result: Result<T, String>) -> String {
    match result {
        Ok(_) => panic!("오류가 나야 하는데 성공했다"),
        Err(e) => e,
    }
}

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
fn test_apply_default_rules_inserts_when_empty() {
    let conn = setup_test_db();
    let rules = vec![
        serde_json::json!({"oldText": "hello", "newText": "world", "priority": 0, "isRegex": false}),
        serde_json::json!({"oldText": "foo", "newText": "bar", "priority": 1, "isRegex": false}),
    ];

    apply_default_replace_rules_impl(&conn, &rules).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ReplaceRule", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_disabled_rule_excluded_from_conflicts() {
    let conn = setup_test_db();
    let rule_a = create_replace_rule_db(&conn, "AA", "BB", false, 0).unwrap();
    let rule_b = create_replace_rule_db(&conn, "BB", "CC", false, 1).unwrap();

    // 초기 상태: cascade 충돌 있음 (AA→BB 이후 BB→CC 연쇄)
    let rules = get_replace_rules_impl(&conn).unwrap();
    let ra = rules.iter().find(|r| r.id == rule_a.id).unwrap();
    assert!(
        ra.conflicts.contains(&rule_b.id),
        "B가 enabled일 때 A의 conflicts에 포함되어야 함: {:?}",
        ra.conflicts
    );

    // rule B 비활성화
    update_replace_rule_db(&conn, rule_b.id, "BB", "CC", false, false, 1).unwrap();

    let rules = get_replace_rules_impl(&conn).unwrap();
    let ra = rules.iter().find(|r| r.id == rule_a.id).unwrap();
    assert!(
        ra.conflicts.is_empty(),
        "B가 disabled이면 A의 conflicts에서 제외되어야 함: {:?}",
        ra.conflicts
    );
}

#[test]
fn test_regex_special_chars_roundtrip() {
    let conn = setup_test_db();
    let pattern = r#"\"[^\"]*\""#;
    let rule = create_replace_rule_db(&conn, pattern, "QUOTED", true, 0).unwrap();

    let rules = get_replace_rules_impl(&conn).unwrap();
    let r = rules.iter().find(|r| r.id == rule.id).unwrap();
    assert_eq!(r.old_text, pattern, "백슬래시·따옴표 포함 패턴이 DB 왕복 후 손상 없어야 함");
    assert!(r.is_regex);
}

#[test]
fn test_preview_replace_with_regex() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "활동1");
    let stu_id = insert_student(&conn, 1, 1, 1, "학생1");
    insert_record(&conn, act_id, stu_id, "줄바꿈\n\n테스트");

    create_replace_rule_db(&conn, r"\n+", " ", true, 0).unwrap();

    let rules = fetch_rules_from_db(&conn).unwrap();
    let records = get_records_for_scope(&conn, "all", &[], None).unwrap();
    assert_eq!(records.len(), 1);
    let result = apply_rules(&records[0].content, &rules);
    assert!(!result.contains('\n'), "정규식 규칙 적용 후 개행 없어야 함: {:?}", result);
    assert_eq!(result, "줄바꿈 테스트");
}

#[test]
fn test_apply_rules_invisible_chars_regex_class() {
    use crate::types::ReplaceRule;
    let rule = ReplaceRule {
        id: 1,
        old_text: "[\u{200B}\u{200C}\u{200D}\u{FEFF}]".to_string(),
        new_text: "".to_string(),
        is_regex: true,
        enabled: true,
        priority: 0,
        created_at: String::new(),
        updated_at: String::new(),
        conflicts: vec![],
    };
    let input = format!("가{}나{}다{}라{}", '\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}');
    let result = apply_rules(&input, &[rule]);
    assert_eq!(result, "가나다라");
}

#[test]
fn test_apply_rules_repeated_punctuation_regex() {
    use crate::types::ReplaceRule;
    fn regex_rule(pattern: &str, replacement: &str) -> ReplaceRule {
        ReplaceRule {
            id: 1,
            old_text: pattern.to_string(),
            new_text: replacement.to_string(),
            is_regex: true,
            enabled: true,
            priority: 0,
            created_at: String::new(),
            updated_at: String::new(),
            conflicts: vec![],
        }
    }

    assert_eq!(apply_rules("정말....대단해", &[regex_rule(r"\.{2,}", ".")]), "정말.대단해");
    assert_eq!(apply_rules("정말!!!대단해", &[regex_rule(r"!{2,}", "!")]), "정말!대단해");
    assert_eq!(apply_rules("진짜???뭐야", &[regex_rule(r"\?{2,}", "?")]), "진짜?뭐야");
    assert_eq!(apply_rules("하나,,둘", &[regex_rule(r",{2,}", ",")]), "하나,둘");
}

// ── CHECK 제약 검증 ────────────────────────────────────────────

#[test]
fn test_create_replace_rule_negative_priority_insert_or_ignore_behavior() {
    // INSERT OR IGNORE: CHECK 위반 → 삽입 무시 → changes()==0 → "이미 동일한 규칙" 에러 반환
    let conn = setup_test_db();
    let err = create_replace_rule_db(&conn, "a", "b", false, -1).unwrap_err();
    assert!(
        err.contains("이미 동일한 규칙"),
        "INSERT OR IGNORE로 CHECK 위반이 무시되어 changes()==0 에러가 반환되어야 함: {err}"
    );
}

#[test]
fn test_update_replace_rule_negative_priority_violates_check() {
    // UPDATE: OR IGNORE 없음 → CHECK 위반 직접 전파
    let conn = setup_test_db();
    let rule = create_replace_rule_db(&conn, "a", "b", false, 0).unwrap();
    let err = update_replace_rule_db(&conn, rule.id, "a", "b", false, true, -1).unwrap_err();
    assert!(err.contains("CHECK constraint failed"), "priority=-1 UPDATE CHECK 위반이어야 함: {err}");
}

#[test]
fn test_apply_default_rules_merges_when_nonempty() {
    let conn = setup_test_db();
    create_replace_rule_db(&conn, "existing", "rule", false, 0).unwrap();

    let default_rules = vec![
        serde_json::json!({"oldText": "hello", "newText": "world", "priority": 0, "isRegex": false}),
    ];
    apply_default_replace_rules_impl(&conn, &default_rules).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ReplaceRule", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "기존 규칙은 유지하고 누락된 기본 규칙만 추가되어야 함");
    let old_text: String = conn
        .query_row("SELECT old_text FROM ReplaceRule WHERE priority = 0 AND new_text = 'rule'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(old_text, "existing", "기존 커스텀 규칙이 그대로 남아있어야 함");
}

#[test]
fn test_apply_default_rules_does_not_duplicate_existing_default() {
    let conn = setup_test_db();
    create_replace_rule_db(&conn, "hello", "world", false, 0).unwrap();

    let default_rules = vec![
        serde_json::json!({"oldText": "hello", "newText": "world", "priority": 0, "isRegex": false}),
    ];
    apply_default_replace_rules_impl(&conn, &default_rules).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ReplaceRule", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "old_text+new_text가 동일한 기본 규칙은 중복 삽입되지 않아야 함");
}

// ── 잘못된 정규식은 조용히 넘어가지 않는다 ────────────────────

#[test]
fn test_preview_replace_rejects_invalid_regex_rule() {
    let conn = setup_test_db();
    insert_raw_regex_rule(&conn, "[invalid(");

    let err = expect_err(preview_replace_impl(&conn, "all", &[], None, &mut empty_cache()));
    assert!(err.contains("정규식"), "에러 메시지: {err}");
}

#[test]
fn test_apply_replace_rejects_invalid_regex_rule() {
    let conn = setup_test_db();
    insert_raw_regex_rule(&conn, "[invalid(");

    let err = expect_err(apply_replace_impl(&conn, "all", &[], None, &mut empty_cache()));
    assert!(err.contains("정규식"), "에러 메시지: {err}");
}

#[test]
fn test_apply_default_rules_rejects_invalid_regex() {
    let conn = setup_test_db();
    let rules = vec![
        serde_json::json!({"oldText": "hello", "newText": "world", "priority": 0, "isRegex": false}),
        serde_json::json!({"oldText": "[invalid(", "newText": "x", "priority": 1, "isRegex": true}),
    ];

    let err = apply_default_replace_rules_impl(&conn, &rules).unwrap_err();
    assert!(err.contains("정규식"), "에러 메시지: {err}");

    // 롤백되어 앞선 규칙도 남으면 안 된다.
    assert!(fetch_rules_from_db(&conn).unwrap().is_empty());
}

#[test]
fn test_apply_default_rules_missing_field_leaves_no_open_transaction() {
    // 필드가 빠진 규칙을 만나면 오류로 빠져나가는데, 예전에는 ROLLBACK 없이
    // 빠져나가 트랜잭션이 열린 채 남았다. 공유 Connection이라 그 뒤 모든 쓰기가
    // 그 트랜잭션에 묶이고 앱 종료 시 통째로 사라진다.
    let conn = setup_test_db();
    let rules = vec![
        serde_json::json!({"oldText": "hello", "newText": "world", "priority": 0}),
        serde_json::json!({"oldText": "priority가 없는 규칙", "newText": "x"}),
    ];

    let err = expect_err(apply_default_replace_rules_impl(&conn, &rules));
    assert!(err.contains("priority"), "에러 메시지: {err}");

    // 트랜잭션이 남아 있으면 BEGIN이 실패한다.
    match conn.execute_batch("BEGIN") {
        Ok(_) => {
            let _ = conn.execute_batch("ROLLBACK");
        }
        Err(e) => panic!("트랜잭션이 열린 채 남았다: {e}"),
    }

    // 앞선 규칙도 롤백되어야 한다.
    assert!(fetch_rules_from_db(&conn).unwrap().is_empty());
}

// ── 꺼진 규칙은 정규식 검증을 걸지 않는다 ─────────────────────

#[test]
fn test_disabled_invalid_regex_rule_can_be_saved() {
    // 구버전에서 저장된 잘못된 정규식 규칙을 끄려면 저장이 되어야 한다.
    // 막으면 사용자에게 남는 선택지가 삭제뿐이 된다.
    assert!(validate_replace_rule_for_save("[invalid(", "x", true, false).is_ok());
}

#[test]
fn test_enabled_invalid_regex_rule_still_rejected() {
    let err = expect_err(validate_replace_rule_for_save("[invalid(", "x", true, true));
    assert!(err.contains("정규식"), "에러 메시지: {err}");
}

#[test]
fn test_disabled_rule_still_checked_for_empty_and_identical_text() {
    // 정규식 검사만 건너뛴다. 나머지 기본 검증은 꺼진 규칙에도 그대로 적용된다.
    assert!(validate_replace_rule_for_save("", "x", false, false).is_err());
    assert!(validate_replace_rule_for_save("같음", "같음", false, false).is_err());
}

// ── preview / apply ───────────────────────────────────────────

#[test]
fn test_preview_replace_excludes_unchanged_records() {
    let conn = setup_test_db();
    let act = insert_activity(&conn, "활동");
    let s1 = insert_student(&conn, 1, 1, 1, "학생1");
    let s2 = insert_student(&conn, 1, 1, 2, "학생2");
    insert_record(&conn, act, s1, "사과를 먹었다");
    insert_record(&conn, act, s2, "귤을 먹었다");
    create_replace_rule_db(&conn, "사과", "배", false, 0).unwrap();

    let items = preview_replace_impl(&conn, "all", &[], None, &mut empty_cache()).unwrap();

    assert_eq!(items.len(), 1, "바뀌지 않는 기록은 미리보기에 나오면 안 된다");
    assert_eq!(items[0].student_id, s1);
    assert_eq!(items[0].result, "배를 먹었다");
}

#[test]
fn test_apply_replace_plain_file_keeps_pre_replace_history() {
    // 암호화 경로는 crypto_cmd_tests가 본다. 평문 파일에서도 치환 전 내용이 남아야 한다.
    let conn = setup_test_db();
    let act = insert_activity(&conn, "활동");
    let stu = insert_student(&conn, 1, 1, 1, "학생");
    insert_record(&conn, act, stu, "사과를 먹었다");
    create_replace_rule_db(&conn, "사과", "배", false, 0).unwrap();

    apply_replace_impl(&conn, "all", &[], None, &mut empty_cache()).unwrap();

    let (content, note): (String, Option<String>) = conn
        .query_row(
            "SELECT content, note FROM ActivityRecordHistory ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(content, "사과를 먹었다");
    assert_eq!(note.as_deref(), Some("치환 적용 전"));
}

#[test]
fn test_apply_replace_is_atomic_on_mid_failure() {
    // 두 번째 기록을 쓰다 실패하면 첫 번째 기록의 치환과 히스토리도 함께 되돌아가야 한다.
    // 그렇지 않으면 일부만 치환된 채 "치환 적용 전" 히스토리만 남는다.
    let conn = setup_test_db();
    let act = insert_activity(&conn, "활동");
    let s1 = insert_student(&conn, 1, 1, 1, "학생1");
    let s2 = insert_student(&conn, 1, 1, 2, "학생2");
    insert_record(&conn, act, s1, "사과");
    insert_record(&conn, act, s2, "사과");
    create_replace_rule_db(&conn, "사과", "배", false, 0).unwrap();
    conn.execute_batch(&format!(
        "CREATE TRIGGER fail_second BEFORE UPDATE ON ActivityRecord
         WHEN NEW.student_id = {s2} BEGIN SELECT RAISE(ABORT, 'forced'); END;"
    ))
    .unwrap();

    assert!(apply_replace_impl(&conn, "all", &[], None, &mut empty_cache()).is_err());

    let first: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE student_id = ?1",
            [s1],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(first, "사과", "첫 번째 기록의 치환이 롤백되어야 한다");
    let history: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |r| r.get(0))
        .unwrap();
    assert_eq!(history, 0, "롤백된 치환의 히스토리가 남으면 안 된다");
}
