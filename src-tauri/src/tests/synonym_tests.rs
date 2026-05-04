use crate::commands::synonym::{
    add_synonym_word_impl, create_synonym_group_impl, delete_synonym_group_impl,
    delete_synonym_word_impl, get_all_records_for_inspect_impl, get_synonym_groups_impl,
    seed_default_synonyms_impl,
};
use crate::types::SeedGroupInput;
use super::setup_test_db;

// ── 그룹 생성/삭제 ──────────────────────────────────────────────

#[test]
fn test_create_synonym_group_returns_id() {
    let conn = setup_test_db();
    let id = create_synonym_group_impl(&conn, "감정 표현").unwrap();
    assert!(id > 0);
}

#[test]
fn test_create_synonym_group_duplicate_name_error() {
    let conn = setup_test_db();
    create_synonym_group_impl(&conn, "감정 표현").unwrap();
    let err = create_synonym_group_impl(&conn, "감정 표현").unwrap_err();
    assert!(err.contains("이미 존재하는 그룹명"), "에러 메시지: {err}");
}

#[test]
fn test_delete_synonym_group_cascades_words() {
    let conn = setup_test_db();
    let gid = create_synonym_group_impl(&conn, "그룹A").unwrap();
    add_synonym_word_impl(&conn, gid, "단어1").unwrap();
    add_synonym_word_impl(&conn, gid, "단어2").unwrap();

    delete_synonym_group_impl(&conn, gid).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM SynonymItem WHERE group_id = ?1", [gid], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0, "그룹 삭제 시 SynonymItem도 CASCADE 삭제되어야 함");
}

// ── 단어 추가/삭제 ──────────────────────────────────────────────

#[test]
fn test_add_synonym_word_returns_id() {
    let conn = setup_test_db();
    let gid = create_synonym_group_impl(&conn, "그룹").unwrap();
    let wid = add_synonym_word_impl(&conn, gid, "단어").unwrap();
    assert!(wid > 0);
}

#[test]
fn test_add_synonym_word_duplicate_ignored() {
    let conn = setup_test_db();
    let gid = create_synonym_group_impl(&conn, "그룹").unwrap();
    add_synonym_word_impl(&conn, gid, "중복어").unwrap();
    // INSERT OR IGNORE — 에러 없이 Ok 반환
    let result = add_synonym_word_impl(&conn, gid, "중복어");
    assert!(result.is_ok(), "중복 단어 삽입은 에러 없이 무시되어야 함");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM SynonymItem WHERE group_id = ?1", [gid], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "중복 단어는 한 번만 저장");
}

#[test]
fn test_delete_synonym_word() {
    let conn = setup_test_db();
    let gid = create_synonym_group_impl(&conn, "그룹").unwrap();
    let wid = add_synonym_word_impl(&conn, gid, "삭제할단어").unwrap();

    delete_synonym_word_impl(&conn, wid).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM SynonymItem WHERE id = ?1", [wid], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

// ── get_synonym_groups_impl ─────────────────────────────────────

#[test]
fn test_get_synonym_groups_empty_db() {
    let conn = setup_test_db();
    let groups = get_synonym_groups_impl(&conn).unwrap();
    assert!(groups.is_empty());
}

#[test]
fn test_get_synonym_groups_with_words_sorted_alphabetically() {
    let conn = setup_test_db();
    let gid = create_synonym_group_impl(&conn, "그룹").unwrap();
    add_synonym_word_impl(&conn, gid, "나").unwrap();
    add_synonym_word_impl(&conn, gid, "가").unwrap();
    add_synonym_word_impl(&conn, gid, "다").unwrap();

    let groups = get_synonym_groups_impl(&conn).unwrap();
    assert_eq!(groups.len(), 1);
    let words: Vec<&str> = groups[0].items.iter().map(|i| i.word.as_str()).collect();
    assert_eq!(words, vec!["가", "나", "다"], "단어는 알파벳(가나다) 순 정렬");
}

#[test]
fn test_get_synonym_groups_no_words_returns_empty_items() {
    let conn = setup_test_db();
    create_synonym_group_impl(&conn, "빈그룹").unwrap();

    let groups = get_synonym_groups_impl(&conn).unwrap();
    assert_eq!(groups.len(), 1);
    assert!(groups[0].items.is_empty(), "단어 없는 그룹은 items=[]");
}

// ── seed_default_synonyms_impl ──────────────────────────────────

#[test]
fn test_seed_default_synonyms_inserts_when_empty() {
    let conn = setup_test_db();
    let groups = vec![
        SeedGroupInput { name: "긍정".to_string(), words: vec!["좋다".to_string(), "훌륭하다".to_string()] },
        SeedGroupInput { name: "부정".to_string(), words: vec!["나쁘다".to_string()] },
    ];

    seed_default_synonyms_impl(&conn, &groups).unwrap();

    let gc: i64 = conn
        .query_row("SELECT COUNT(*) FROM SynonymGroup", [], |r| r.get(0))
        .unwrap();
    let wc: i64 = conn
        .query_row("SELECT COUNT(*) FROM SynonymItem", [], |r| r.get(0))
        .unwrap();
    assert_eq!(gc, 2);
    assert_eq!(wc, 3);
}

#[test]
fn test_seed_default_synonyms_skips_when_nonempty() {
    let conn = setup_test_db();
    create_synonym_group_impl(&conn, "기존그룹").unwrap();

    let groups = vec![
        SeedGroupInput { name: "신규그룹".to_string(), words: vec!["단어".to_string()] },
    ];
    seed_default_synonyms_impl(&conn, &groups).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM SynonymGroup", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "기존 그룹 있으면 seed 무시");
    let name: String = conn
        .query_row("SELECT name FROM SynonymGroup", [], |r| r.get(0))
        .unwrap();
    assert_eq!(name, "기존그룹");
}

// ── get_all_records_for_inspect_impl ───────────────────────────

fn setup_inspect_db() -> rusqlite::Connection {
    let conn = setup_test_db();
    // 영역, 활동, 학생, 기록 삽입
    conn.execute("INSERT INTO Area (name, byte_limit) VALUES ('영역A', 500)", []).unwrap();
    let area_id = conn.last_insert_rowid();
    conn.execute("INSERT INTO Activity (name) VALUES ('활동1')", []).unwrap();
    let act_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    ).unwrap();
    conn.execute(
        "INSERT INTO Student (grade, class_num, number, name) VALUES (1, 1, 1, '홍길동')",
        [],
    ).unwrap();
    let stu_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    ).unwrap();
    conn.execute(
        "INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, '테스트 기록')",
        rusqlite::params![act_id, stu_id],
    ).unwrap();
    conn
}

#[test]
fn test_get_all_records_for_inspect_all_scope() {
    let conn = setup_inspect_db();
    let records = get_all_records_for_inspect_impl(&conn, "all", vec![], None).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "테스트 기록");
    assert_eq!(records[0].student_name, "홍길동");
}

#[test]
fn test_get_all_records_for_inspect_areas_scope() {
    let conn = setup_inspect_db();
    let area_id: i64 = conn
        .query_row("SELECT id FROM Area WHERE name = '영역A'", [], |r| r.get(0))
        .unwrap();

    let records = get_all_records_for_inspect_impl(&conn, "areas", vec![area_id], None).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].area_name, "영역A");
}

#[test]
fn test_get_all_records_for_inspect_areas_empty_ids() {
    let conn = setup_inspect_db();
    let records = get_all_records_for_inspect_impl(&conn, "areas", vec![], None).unwrap();
    assert!(records.is_empty(), "area_ids=[] 이면 빈 목록 반환");
}

#[test]
fn test_get_all_records_for_inspect_unknown_scope_error() {
    let conn = setup_test_db();
    let err = get_all_records_for_inspect_impl(&conn, "unknown", vec![], None).unwrap_err();
    assert!(err.contains("알 수 없는 scope_type"), "에러 메시지: {err}");
}

#[test]
fn test_inspect_multiple_area_ids() {
    let conn = setup_test_db();

    conn.execute("INSERT INTO Area (name, byte_limit) VALUES ('영역1', 500)", []).unwrap();
    let area1 = conn.last_insert_rowid();
    conn.execute("INSERT INTO Area (name, byte_limit) VALUES ('영역2', 500)", []).unwrap();
    let area2 = conn.last_insert_rowid();

    conn.execute("INSERT INTO Activity (name) VALUES ('활동1')", []).unwrap();
    let act1 = conn.last_insert_rowid();
    conn.execute("INSERT INTO Activity (name) VALUES ('활동2')", []).unwrap();
    let act2 = conn.last_insert_rowid();
    conn.execute("INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)", rusqlite::params![area1, act1]).unwrap();
    conn.execute("INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)", rusqlite::params![area2, act2]).unwrap();

    conn.execute("INSERT INTO Student (grade, class_num, number, name) VALUES (1, 1, 1, '학생1')", []).unwrap();
    let stu = conn.last_insert_rowid();
    conn.execute("INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)", rusqlite::params![area1, stu]).unwrap();
    conn.execute("INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)", rusqlite::params![area2, stu]).unwrap();

    conn.execute("INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, '기록1')", rusqlite::params![act1, stu]).unwrap();
    conn.execute("INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, '기록2')", rusqlite::params![act2, stu]).unwrap();

    let records = get_all_records_for_inspect_impl(&conn, "areas", vec![area1, area2], None).unwrap();
    assert_eq!(records.len(), 2, "두 area_id로 조회 시 2개 기록 반환: {:?}", records);
}

#[test]
fn test_inspect_area_student_filter() {
    let conn = setup_test_db();

    conn.execute("INSERT INTO Area (name, byte_limit) VALUES ('영역', 500)", []).unwrap();
    let area = conn.last_insert_rowid();
    conn.execute("INSERT INTO Activity (name) VALUES ('활동')", []).unwrap();
    let act = conn.last_insert_rowid();
    conn.execute("INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)", rusqlite::params![area, act]).unwrap();

    conn.execute("INSERT INTO Student (grade, class_num, number, name) VALUES (1, 1, 1, '등록학생')", []).unwrap();
    let stu1 = conn.last_insert_rowid();
    conn.execute("INSERT INTO Student (grade, class_num, number, name) VALUES (1, 1, 2, '미등록학생')", []).unwrap();
    let stu2 = conn.last_insert_rowid();
    // stu1만 AreaStudent에 등록
    conn.execute("INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)", rusqlite::params![area, stu1]).unwrap();

    conn.execute("INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, '기록1')", rusqlite::params![act, stu1]).unwrap();
    conn.execute("INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, '기록2')", rusqlite::params![act, stu2]).unwrap();

    let records = get_all_records_for_inspect_impl(&conn, "areas", vec![area], None).unwrap();
    assert_eq!(records.len(), 1, "AreaStudent 미등록 학생 기록은 제외되어야 함");
    assert_eq!(records[0].student_name, "등록학생");
}

#[test]
fn test_inspect_coalesce_null_area_name() {
    let conn = setup_test_db();

    // Area 없이 Activity + Student + Record 생성
    conn.execute("INSERT INTO Activity (name) VALUES ('독립활동')", []).unwrap();
    let act = conn.last_insert_rowid();
    conn.execute("INSERT INTO Student (grade, class_num, number, name) VALUES (2, 3, 5, '독립학생')", []).unwrap();
    let stu = conn.last_insert_rowid();
    conn.execute("INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, '독립기록')", rusqlite::params![act, stu]).unwrap();

    let records = get_all_records_for_inspect_impl(&conn, "all", vec![], None).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].area_name, "", "area 미연결 기록은 area_name='' 이어야 함");
    assert_eq!(records[0].student_name, "독립학생");
}

#[test]
fn test_get_all_records_for_inspect_excludes_empty_content() {
    let conn = setup_test_db();
    conn.execute("INSERT INTO Activity (name) VALUES ('활동')", []).unwrap();
    let act_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO Student (grade, class_num, number, name) VALUES (1, 1, 1, '학생')",
        [],
    ).unwrap();
    let stu_id = conn.last_insert_rowid();
    // content 빈 문자열
    conn.execute(
        "INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, '')",
        rusqlite::params![act_id, stu_id],
    ).unwrap();

    let records = get_all_records_for_inspect_impl(&conn, "all", vec![], None).unwrap();
    assert!(records.is_empty(), "content='' 기록은 제외되어야 함");
}
