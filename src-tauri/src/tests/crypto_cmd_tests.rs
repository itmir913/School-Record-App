use super::{insert_activity, insert_area, insert_student, setup_temp_db_path_state, setup_test_db};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rusqlite::Connection;
use crate::commands::config::set_config_impl;
use crate::commands::crypto::{
    change_encryption_password_impl, disable_encryption_impl, enable_encryption_impl,
    get_encryption_status_impl, resolve_data_key, unlock_encryption_impl,
};
use crate::commands::record::{
    bulk_import_records_impl, get_area_grid_impl, get_record_history_impl,
    preview_import_records_impl, save_snapshot_internal, upsert_record_impl,
};
use crate::commands::replace::{apply_replace_impl, create_replace_rule_db, preview_replace_impl};
use crate::commands::snapshot::{create_snapshot_impl, restore_snapshot_impl};
use crate::commands::synonym::get_all_records_for_inspect_impl;
use crate::state::ReplaceCache;
use crate::commands::student::{
    bulk_upsert_students_impl, create_student_impl, get_students_impl, update_student_impl,
};
use crate::crypto::derive_key;
use crate::engine::get_records_for_scope;
use crate::state::{clear_crypto_state, CryptoState, CryptoStateHandle};
use crate::types::{ImportRecordInput, StudentInput};

fn test_key() -> [u8; 32] {
    derive_key("password", &[42u8; 16])
}

fn crypto_state(key: Option<[u8; 32]>) -> CryptoStateHandle {
    std::sync::Mutex::new(CryptoState { key })
}

// ── 학생 이름 암호화 ──────────────────────────────────────────────

#[test]
fn test_create_student_with_key_stores_encrypted_name() {
    let conn = setup_test_db();
    let key = test_key();
    create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();

    // DB에는 암호화된 값이 저장되어야 한다
    let raw_name: String = conn
        .query_row("SELECT name FROM Student WHERE grade=1", [], |r| r.get(0))
        .unwrap();
    assert_ne!(raw_name, "홍길동", "DB에 평문이 저장되면 안 된다");
}

#[test]
fn test_get_students_with_key_decrypts_name() {
    let conn = setup_test_db();
    let key = test_key();
    create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();

    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert_eq!(students[0].name, "홍길동");
}

#[test]
fn test_get_students_without_key_returns_encrypted_value() {
    let conn = setup_test_db();
    let key = test_key();
    create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();

    // 키 없이 조회하면 암호화된 raw 값이 그대로 나온다
    let students = get_students_impl(&conn, None).unwrap();
    assert_ne!(students[0].name, "홍길동");
}

#[test]
fn test_update_student_with_key_stores_and_reads_correctly() {
    let conn = setup_test_db();
    let key = test_key();
    let id = create_student_impl(&conn, 1, 1, 1, "원래이름", Some(key)).unwrap();
    update_student_impl(&conn, id, 1, 1, 1, "변경이름", Some(key)).unwrap();

    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert_eq!(students[0].name, "변경이름");
}

#[test]
fn test_get_students_sorted_order_preserved_with_encryption() {
    let conn = setup_test_db();
    let key = test_key();
    create_student_impl(&conn, 2, 1, 1, "세번째", Some(key)).unwrap();
    create_student_impl(&conn, 1, 2, 1, "두번째", Some(key)).unwrap();
    create_student_impl(&conn, 1, 1, 1, "첫번째", Some(key)).unwrap();

    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert_eq!(students[0].name, "첫번째");
    assert_eq!(students[1].name, "두번째");
    assert_eq!(students[2].name, "세번째");
}

#[test]
fn test_bulk_upsert_students_with_key() {
    let conn = setup_test_db();
    let key = test_key();
    let inputs = vec![
        StudentInput {
            grade: 1,
            class_num: 1,
            number: 1,
            name: "가".to_string(),
        },
        StudentInput {
            grade: 1,
            class_num: 1,
            number: 2,
            name: "나".to_string(),
        },
    ];
    let result = bulk_upsert_students_impl(&conn, &inputs, Some(key)).unwrap();
    assert_eq!(result.inserted, 2);

    let students = get_students_impl(&conn, Some(key)).unwrap();
    let names: Vec<&str> = students.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"가"));
    assert!(names.contains(&"나"));
}

#[test]
fn test_bulk_upsert_students_update_overwrites_with_encryption() {
    let conn = setup_test_db();
    let key = test_key();
    // 1차 삽입
    bulk_upsert_students_impl(
        &conn,
        &[StudentInput {
            grade: 1,
            class_num: 1,
            number: 1,
            name: "원래".to_string(),
        }],
        Some(key),
    )
    .unwrap();
    // 2차 갱신
    bulk_upsert_students_impl(
        &conn,
        &[StudentInput {
            grade: 1,
            class_num: 1,
            number: 1,
            name: "변경".to_string(),
        }],
        Some(key),
    )
    .unwrap();

    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert_eq!(students[0].name, "변경");
}

// ── 기록 content 암호화 ───────────────────────────────────────────

#[test]
fn test_upsert_record_with_key_stores_encrypted_content() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(
        &conn,
        act_id,
        stu_id,
        "리더십이 뛰어난 학생입니다.",
        Some(key),
    )
    .unwrap();

    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(
        raw, "리더십이 뛰어난 학생입니다.",
        "DB에 평문이 저장되면 안 된다"
    );
}

#[test]
fn test_upsert_record_empty_content_not_encrypted() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "", Some(key)).unwrap();

    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(raw, "", "빈 문자열은 암호화하지 않아야 한다");
}

#[test]
fn test_get_area_grid_with_key_decrypts_content_and_name() {
    let conn = setup_test_db();
    let key = test_key();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "독서");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "김철수", Some(key)).unwrap();

    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "독후감 내용", Some(key)).unwrap();

    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.students[0].name, "김철수");
    assert_eq!(grid.records[0].content, "독후감 내용");
}

#[test]
fn test_get_record_history_with_key_decrypts_content() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "발표 내용", Some(key)).unwrap();

    // 히스토리에 암호화된 content를 직접 삽입
    let encrypted_content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         SELECT id, content, '2024-01-01 10:00:00', NULL FROM ActivityRecord
         WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    )
    .unwrap();

    let history = get_record_history_impl(&conn, act_id, stu_id, 10, 0, Some(key)).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(
        history[0].content, "발표 내용",
        "복호화된 히스토리 content여야 한다"
    );
    // DB에는 여전히 암호화 값이 저장되어 있어야 한다
    // — 평문과 달라야 하고, 올바른 키로 복호화하면 원문이 나와야 한다
    assert_ne!(encrypted_content, "발표 내용", "DB에 평문이 저장되면 안 된다");
    let decrypted = crate::crypto::decrypt(&encrypted_content, &key)
        .expect("유효한 암호문이어야 한다");
    assert_eq!(decrypted, "발표 내용", "올바른 키로 복호화하면 원문이어야 한다");
}

// ── engine: get_records_for_scope ─────────────────────────────────

#[test]
fn test_get_records_for_scope_all_with_key_decrypts() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "활동");
    let stu_id = insert_student(&conn, 1, 1, 1, "학생");
    upsert_record_impl(&conn, act_id, stu_id, "기록 내용", Some(key)).unwrap();

    let records = get_records_for_scope(&conn, "all", &[], Some(key)).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "기록 내용");
}

#[test]
fn test_get_records_for_scope_all_without_key_returns_encrypted() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "활동");
    let stu_id = insert_student(&conn, 1, 1, 1, "학생");
    upsert_record_impl(&conn, act_id, stu_id, "기록 내용", Some(key)).unwrap();

    let records = get_records_for_scope(&conn, "all", &[], None).unwrap();
    assert_eq!(records.len(), 1);
    assert_ne!(
        records[0].content, "기록 내용",
        "키 없이 조회하면 암호화된 값이 나와야 한다"
    );
}

#[test]
fn test_get_records_for_scope_areas_with_key_decrypts() {
    let conn = setup_test_db();
    let key = test_key();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "활동");
    let stu_id = insert_student(&conn, 1, 1, 1, "학생");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "영역별 내용", Some(key)).unwrap();

    let records = get_records_for_scope(&conn, "areas", &[area_id], Some(key)).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "영역별 내용");
}

// ── 암호화 없을 때 기존 동작 보존 ─────────────────────────────────

#[test]
fn test_create_get_student_without_encryption_unchanged() {
    let conn = setup_test_db();
    create_student_impl(&conn, 1, 1, 1, "홍길동", None).unwrap();
    let students = get_students_impl(&conn, None).unwrap();
    assert_eq!(students[0].name, "홍길동");
}

#[test]
fn test_upsert_get_record_without_encryption_unchanged() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "발표 내용", None).unwrap();

    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1",
            rusqlite::params![act_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(content, "발표 내용");
}

// ── 기존 평문 데이터 → 키 적용 시 에러 (일관성 검증) ───────────────

#[test]
fn test_decrypt_plaintext_with_key_returns_error() {
    let conn = setup_test_db();
    let key = test_key();
    // 평문으로 저장된 학생 이름을 key로 복호화하려 하면 에러여야 한다
    insert_student(&conn, 1, 1, 1, "평문이름");

    let result = get_students_impl(&conn, Some(key));
    // "잘못된 암호화 형식" 또는 "복호화 실패" 에러
    assert!(result.is_err(), "평문을 키로 복호화하면 에러여야 한다");
}

// ── enable_all_data / disable_all_data 흐름 통합 검증 ──────────────

#[test]
fn test_encrypt_then_decrypt_all_data_restores_plaintext() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);

    // 1. 평문으로 데이터 삽입
    let act_id = insert_activity(&conn, "활동");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "활동 기록", None).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, Some("before encryption")).unwrap();

    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let status = get_encryption_status_impl(&conn, &crypto).unwrap();
    assert!(status.enabled);
    assert!(status.unlocked);

    let raw_name: String = conn
        .query_row(
            "SELECT name FROM Student WHERE id=?1",
            rusqlite::params![stu_id],
            |r| r.get(0),
        )
        .unwrap();
    let raw_content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    let raw_history: String = conn
        .query_row(
            "SELECT content FROM ActivityRecordHistory LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw_name, "홍길동");
    assert_ne!(raw_content, "활동 기록");
    assert_ne!(raw_history, "활동 기록");

    // 2. 암호화 상태에서 _impl 으로 읽기
    let key = resolve_data_key(&conn, &crypto).unwrap();
    let students = get_students_impl(&conn, key).unwrap();
    assert_eq!(students[0].name, "홍길동");

    // 3. 복호화 후 None 키로 읽으면 평문이 나와야 한다
    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();
    std::fs::remove_dir_all(&tmp_dir).ok();
    let status = get_encryption_status_impl(&conn, &crypto).unwrap();
    assert!(!status.enabled);
    assert!(!status.unlocked);

    let students = get_students_impl(&conn, None).unwrap();
    assert_eq!(students[0].name, "홍길동");
    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(content, "활동 기록");
}

#[test]
fn test_resolve_data_key_requires_unlock_when_enabled() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    set_config_impl(&conn, "encryption_enabled", "true").unwrap();

    let err = resolve_data_key(&conn, &crypto).unwrap_err();
    assert!(err.contains("잠금"), "에러 메시지: {err}");
}

#[test]
fn test_change_password_creates_backup_file() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);

    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    change_encryption_password_impl(&conn, &crypto, &db_path, "password", "new-password").unwrap();

    let backup_exists = std::fs::read_dir(&tmp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .contains("-pre-reencrypt")
        });
    assert!(backup_exists, "비밀번호 변경 전 백업 파일이 생성되어야 한다");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_change_password_requires_new_password_afterward() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "old-password").unwrap();
    change_encryption_password_impl(&conn, &crypto, &db_path, "old-password", "new-password").unwrap();
    std::fs::remove_dir_all(&tmp_dir).ok();

    clear_crypto_state(&crypto).unwrap();
    assert!(unlock_encryption_impl(&conn, &crypto, "old-password").is_err());
    unlock_encryption_impl(&conn, &crypto, "new-password").unwrap();

    let key = resolve_data_key(&conn, &crypto).unwrap();
    let students = get_students_impl(&conn, key).unwrap();
    assert_eq!(students[0].id, stu_id);
    assert_eq!(students[0].name, "홍길동");
}

// ── bulk_import_records: 암호화 경로 ─────────────────────────────

fn make_import(
    grade: i64,
    class_num: i64,
    number: i64,
    name: Option<&str>,
    activity_id: i64,
    content: &str,
) -> ImportRecordInput {
    ImportRecordInput {
        grade,
        class_num,
        number,
        name: name.map(|s| s.to_string()),
        activity_id,
        content: content.to_string(),
    }
}

#[test]
fn test_bulk_import_records_with_key() {
    let conn = setup_test_db();
    let key = test_key();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "발표");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();

    bulk_import_records_impl(
        &conn,
        &[make_import(1, 1, 1, Some("홍길동"), act_id, "발표 내용")],
        Some(key),
    )
    .unwrap();

    // DB에 이름과 content가 암호화된 채로 저장되어야 한다
    let raw_name: String = conn
        .query_row("SELECT name FROM Student WHERE grade=1", [], |r| r.get(0))
        .unwrap();
    assert_ne!(raw_name, "홍길동", "이름이 평문으로 저장되면 안 된다");

    let raw_content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1",
            rusqlite::params![act_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw_content, "발표 내용", "content가 평문으로 저장되면 안 된다");

    // Some(key)로 읽으면 복호화된 원문이 나와야 한다
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, (SELECT id FROM Student WHERE grade=1))",
        rusqlite::params![area_id],
    )
    .unwrap();
    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.students[0].name, "홍길동");
    assert_eq!(grid.records[0].content, "발표 내용");
}

#[test]
fn test_bulk_import_records_with_key_existing_empty_name() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");

    // 이름이 빈 학생을 암호화 상태로 생성 (maybe_encrypt("", key) == "")
    create_student_impl(&conn, 1, 1, 1, "", Some(key)).unwrap();

    // bulk_import로 이름 갱신 시도
    bulk_import_records_impl(
        &conn,
        &[make_import(1, 1, 1, Some("새이름"), act_id, "내용")],
        Some(key),
    )
    .unwrap();

    // 이름이 갱신되고, 암호화된 채로 저장 후 복호화 시 새이름이어야 한다
    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert_eq!(students[0].name, "새이름", "빈 이름은 갱신되어야 한다");

    // DB에는 암호화된 값이 있어야 한다
    let raw_name: String = conn
        .query_row("SELECT name FROM Student WHERE grade=1", [], |r| r.get(0))
        .unwrap();
    assert_ne!(raw_name, "새이름");
}

// ── preview_import_records: 암호화 경로 ──────────────────────────

#[test]
fn test_preview_import_records_with_key() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "기존 내용", Some(key)).unwrap();

    let items = preview_import_records_impl(
        &conn,
        &[make_import(1, 1, 1, Some("홍길동"), act_id, "새 내용")],
        Some(key),
    )
    .unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].student_name, "홍길동", "이름이 복호화되어야 한다");
    assert_eq!(items[0].existing_content, "기존 내용", "기존 content가 복호화되어야 한다");
    assert_eq!(items[0].new_content, "새 내용");
}

// ── get_all_records_for_inspect: 암호화 경로 ─────────────────────

#[test]
fn test_get_all_records_for_inspect_with_key_all_scope() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "발표 평가 내용", Some(key)).unwrap();

    // Some(key)로 조회 → 복호화된 원문
    let records = get_all_records_for_inspect_impl(&conn, "all", vec![], Some(key)).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].student_name, "홍길동");
    assert_eq!(records[0].content, "발표 평가 내용");

    // None으로 조회 → 암호화된 raw 값
    let records_raw = get_all_records_for_inspect_impl(&conn, "all", vec![], None).unwrap();
    assert_eq!(records_raw.len(), 1);
    assert_ne!(records_raw[0].student_name, "홍길동", "키 없이 조회하면 암호화된 이름이어야 한다");
    assert_ne!(records_raw[0].content, "발표 평가 내용", "키 없이 조회하면 암호화된 content여야 한다");
}

#[test]
fn test_get_all_records_for_inspect_with_key_areas_scope() {
    let conn = setup_test_db();
    let key = test_key();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "독서");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "김철수", Some(key)).unwrap();
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "독후감 내용", Some(key)).unwrap();

    let records =
        get_all_records_for_inspect_impl(&conn, "areas", vec![area_id], Some(key)).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].student_name, "김철수");
    assert_eq!(records[0].content, "독후감 내용");
    assert_eq!(records[0].area_name, "국어");
}

// ── snapshot restore: 암호화 경로 ────────────────────────────────

#[test]
fn test_restore_snapshot_with_encryption() {
    let conn = setup_test_db();
    let key = test_key();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "독서");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();

    // v1 내용 저장 후 스냅샷
    upsert_record_impl(&conn, act_id, stu_id, "v1 내용", Some(key)).unwrap();
    let snapshot = create_snapshot_impl(&conn, Some("v1 스냅샷".to_string())).unwrap();

    // v2로 덮어쓰기
    upsert_record_impl(&conn, act_id, stu_id, "v2 내용", Some(key)).unwrap();
    let grid_v2 = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid_v2.records[0].content, "v2 내용");

    // 스냅샷 복원
    restore_snapshot_impl(&conn, snapshot.id).unwrap();

    // 복원 후 v1이 복호화되어 나와야 한다
    let grid_restored = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(
        grid_restored.records[0].content, "v1 내용",
        "스냅샷 복원 후 암호화된 v1 내용이 복호화되어야 한다"
    );
}

// ── preview_replace / apply_replace: 암호화 경로 ─────────────────

fn make_cache() -> ReplaceCache {
    ReplaceCache {
        ruleset_version: 0,
        entries: std::collections::HashMap::new(),
    }
}

#[test]
fn test_preview_replace_with_key_shows_decrypted_content() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "가나다 발표", Some(key)).unwrap();

    create_replace_rule_db(&conn, "가나다", "ABC", false, 1).unwrap();

    let mut cache = make_cache();
    let items = preview_replace_impl(&conn, "all", &[], Some(key), &mut cache).unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].original, "가나다 발표", "preview의 원본이 복호화된 평문이어야 한다");
    assert_eq!(items[0].result, "ABC 발표");
    assert_eq!(items[0].student_name, "홍길동", "student_name이 복호화되어야 한다");
}

#[test]
fn test_apply_replace_with_key_reencrypts_result() {
    let conn = setup_test_db();
    let key = test_key();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "발표");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "가나다 발표", Some(key)).unwrap();

    create_replace_rule_db(&conn, "가나다", "ABC", false, 1).unwrap();

    let mut cache = make_cache();
    let result = apply_replace_impl(&conn, "all", &[], Some(key), &mut cache).unwrap();
    assert_eq!(result.changed_count, 1);

    // DB에 재암호화된 값이 저장되어야 한다
    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw, "ABC 발표", "치환 결과가 평문으로 저장되면 안 된다");

    // Some(key)로 읽으면 치환된 평문이 복호화되어야 한다
    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.records[0].content, "ABC 발표", "치환 후 복호화하면 치환된 원문이어야 한다");

    // 치환 이력도 history에 암호화된 채로 저장되고 복호화 가능해야 한다
    let history = get_record_history_impl(&conn, act_id, stu_id, 10, 0, Some(key)).unwrap();
    assert!(!history.is_empty(), "치환 적용 시 history가 생성되어야 한다");
    assert_eq!(history[0].content, "ABC 발표", "history content도 복호화되어야 한다");
}

// ── disable_encryption + 빈 이름 학생 회귀 테스트 ─────────────────

#[test]
fn test_disable_encryption_with_blank_name_student() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    let act_id = insert_activity(&conn, "발표");

    // 암호화 활성화
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap();

    // 암호화 활성화 후 빈 이름 학생 생성 (버그 시나리오)
    let stu_id = create_student_impl(&conn, 1, 1, 1, "", Some(key.unwrap())).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "기록 내용", key).unwrap();

    // disable_encryption이 실패 없이 완료되어야 한다 (이것이 수정된 버그)
    let result = disable_encryption_impl(&conn, &crypto, &db_path);
    assert!(result.is_ok(), "빈 이름 학생이 있어도 disable_encryption이 성공해야 한다: {:?}", result);

    // 복호화 후 데이터 무결성 확인
    let students = get_students_impl(&conn, None).unwrap();
    assert_eq!(students[0].name, "", "복호화 후 빈 이름이 유지되어야 한다");

    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1",
            rusqlite::params![act_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(content, "기록 내용", "복호화 후 record content가 평문으로 복원되어야 한다");

    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 통합 시나리오 테스트 ──────────────────────────────────────────

#[test]
fn test_full_workflow_with_encryption() {
    // 전체 파이프라인: enable → import → replace → disable
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "발표");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();

    // 1. 암호화 활성화
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap().unwrap();

    // 2. 암호화 상태에서 학생/기록 임포트
    bulk_import_records_impl(
        &conn,
        &[make_import(1, 1, 1, Some("홍길동"), act_id, "가나다 발표 내용")],
        Some(key),
    )
    .unwrap();

    let stu_id: i64 = conn
        .query_row("SELECT id FROM Student WHERE grade=1", [], |r| r.get(0))
        .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();

    // 3. 치환 규칙 적용 (가나다 → ABC)
    create_replace_rule_db(&conn, "가나다", "ABC", false, 1).unwrap();
    let mut cache = make_cache();
    let replace_result = apply_replace_impl(&conn, "all", &[], Some(key), &mut cache).unwrap();
    assert_eq!(replace_result.changed_count, 1);

    // 4. 암호화 상태에서 치환된 결과 확인
    let grid_encrypted = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid_encrypted.records[0].content, "ABC 발표 내용");
    assert_eq!(grid_encrypted.students[0].name, "홍길동");

    // DB에는 암호화된 값이 저장되어야 한다
    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw, "ABC 발표 내용", "치환 결과가 평문으로 저장되면 안 된다");

    // 5. 암호화 비활성화
    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();

    // 6. None 키로 동일한 평문이 조회되어야 한다
    let grid_plain = get_area_grid_impl(&conn, area_id, None).unwrap();
    assert_eq!(grid_plain.records[0].content, "ABC 발표 내용");
    assert_eq!(grid_plain.students[0].name, "홍길동");

    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_encrypt_state_transitions() {
    // 상태 A: 암호화 비활성화 → resolve_data_key → Ok(None)
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    let key_a = resolve_data_key(&conn, &crypto).unwrap();
    assert!(key_a.is_none(), "암호화 비활성화 상태에서 key는 None이어야 한다");

    // 상태 B: 활성화 + 잠금 → resolve_data_key → Err("잠금")
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    clear_crypto_state(&crypto).unwrap(); // 잠금 상태로 전환
    let err = resolve_data_key(&conn, &crypto).unwrap_err();
    assert!(err.contains("잠금"), "잠금 상태 에러 메시지: {err}");

    // 상태 C: 활성화 + 해제 → resolve_data_key → Ok(Some(key))
    unlock_encryption_impl(&conn, &crypto, "password").unwrap();
    let key_c = resolve_data_key(&conn, &crypto).unwrap();
    assert!(key_c.is_some(), "해제 상태에서 key는 Some이어야 한다");
    assert_ne!(key_c.unwrap(), [0u8; 32], "key는 영벡터가 아니어야 한다");

    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_replace_then_history_roundtrip_with_encryption() {
    // apply_replace 후 get_record_history_impl로 history 평문 검증
    // (apply_replace는 치환된 새 content를 history에 저장)
    let conn = setup_test_db();
    let key = test_key();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "발표");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();

    upsert_record_impl(&conn, act_id, stu_id, "원본 내용 ABC", Some(key)).unwrap();

    create_replace_rule_db(&conn, "ABC", "XYZ", false, 1).unwrap();
    let mut cache = make_cache();
    apply_replace_impl(&conn, "all", &[], Some(key), &mut cache).unwrap();

    // 현재 content가 복호화되어야 한다
    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.records[0].content, "원본 내용 XYZ");

    // apply_replace는 치환 후 새 content를 history에 저장한다.
    // upsert_record_impl만으로는 history가 생성되지 않으므로 항목은 1개여야 한다.
    let history = get_record_history_impl(&conn, act_id, stu_id, 10, 0, Some(key)).unwrap();
    assert_eq!(history.len(), 1, "apply_replace 후 history는 정확히 1개여야 한다");
    assert_eq!(history[0].content, "원본 내용 XYZ", "history[0]이 복호화된 치환 결과여야 한다");
}

#[test]
fn test_bulk_import_then_inspect_with_encryption() {
    // 여러 학생 대량 임포트 후 inspect 결과가 모두 평문인지 검증
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");

    bulk_import_records_impl(
        &conn,
        &[
            make_import(1, 1, 1, Some("홍길동"), act_id, "우수한 발표 내용"),
            make_import(1, 1, 2, Some("이순신"), act_id, "성실한 태도 기록"),
            make_import(1, 1, 3, Some("강감찬"), act_id, "창의적인 발표"),
        ],
        Some(key),
    )
    .unwrap();

    // DB에는 암호화된 값이 저장되어야 한다
    // — 평문과 달라야 하고, 올바른 키로 복호화하면 원문이 나와야 한다
    let expected_names = ["홍길동", "이순신", "강감찬"];
    let raw_names: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM Student ORDER BY number")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    for (raw, expected) in raw_names.iter().zip(expected_names.iter()) {
        assert_ne!(raw, expected, "DB에 평문이 저장되면 안 된다: {raw}");
        let decrypted = crate::crypto::decrypt(raw, &key)
            .unwrap_or_else(|e| panic!("유효한 암호문이어야 한다 ({raw}): {e}"));
        assert_eq!(&decrypted, expected, "올바른 키로 복호화하면 원문이어야 한다");
    }

    // Some(key)로 inspect 조회 → 모든 필드가 평문이어야 한다
    let records =
        get_all_records_for_inspect_impl(&conn, "all", vec![], Some(key)).unwrap();
    assert_eq!(records.len(), 3, "3개 기록이 반환되어야 한다");

    let names: Vec<&str> = records.iter().map(|r| r.student_name.as_str()).collect();
    assert!(names.contains(&"홍길동"), "홍길동이 복호화되어야 한다");
    assert!(names.contains(&"이순신"), "이순신이 복호화되어야 한다");
    assert!(names.contains(&"강감찬"), "강감찬이 복호화되어야 한다");

    let contents: Vec<&str> = records.iter().map(|r| r.content.as_str()).collect();
    assert!(contents.contains(&"우수한 발표 내용"));
    assert!(contents.contains(&"성실한 태도 기록"));
    assert!(contents.contains(&"창의적인 발표"));
}

// ── bulk_import 후 history 복호화 조합 테스트 ────────────────────

#[test]
fn test_bulk_import_history_readable_with_key() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");

    bulk_import_records_impl(
        &conn,
        &[make_import(1, 1, 1, Some("홍길동"), act_id, "발표 내용")],
        Some(key),
    )
    .unwrap();

    // bulk_import는 content가 있으면 history에도 암호화된 채로 복사한다
    // get_record_history_impl이 그것을 복호화해서 반환해야 한다
    let stu_id: i64 = conn
        .query_row("SELECT id FROM Student WHERE grade=1", [], |r| r.get(0))
        .unwrap();

    let history = get_record_history_impl(&conn, act_id, stu_id, 10, 0, Some(key)).unwrap();
    assert_eq!(history.len(), 1, "bulk_import 후 history가 1개 생성되어야 한다");
    assert_eq!(
        history[0].content, "발표 내용",
        "bulk_import로 저장된 history content가 복호화되어야 한다"
    );

    // history에 암호화된 값이 저장되어 있어야 한다 (None으로 읽으면 ciphertext)
    let raw_history: String = conn
        .query_row("SELECT content FROM ActivityRecordHistory LIMIT 1", [], |r| r.get(0))
        .unwrap();
    assert_ne!(raw_history, "발표 내용", "history DB에는 암호화된 값이 있어야 한다");
}

// ── 파일 DB 재시작 시나리오 ───────────────────────────────────────────

#[test]
fn test_persist_and_reload_with_encryption() {
    let (db_path_state, dir) = setup_temp_db_path_state();
    let db_path = db_path_state.0.lock().unwrap().clone().unwrap();

    // 1. 파일 DB 초기화 및 데이터 삽입
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(include_str!("../schema.sql")).unwrap();
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    let area_id = insert_area(&conn, "독서", 500);
    let act_id = insert_activity(&conn, "발표");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, ?3)",
        rusqlite::params![act_id, stu_id, "홍길동 발표 내용"],
    )
    .unwrap();

    // 2. 암호화 활성화
    let crypto = crypto_state(None);
    enable_encryption_impl(&conn, &crypto, &db_path_state, "password").unwrap();

    // 3. Connection 닫기 (재시작 시뮬레이션)
    drop(conn);

    // 4. 새 Connection으로 재오픈 + 잠금 상태 CryptoState
    let conn2 = Connection::open(&db_path).unwrap();
    conn2.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    let crypto2 = crypto_state(None);

    // 5. 구 비밀번호로 unlock → 성공
    unlock_encryption_impl(&conn2, &crypto2, "password").unwrap();
    let key = resolve_data_key(&conn2, &crypto2).unwrap().unwrap();

    // 6. 데이터 복호화 확인
    let grid = get_area_grid_impl(&conn2, area_id, Some(key)).unwrap();
    let student = grid.students.iter().find(|s| s.id == stu_id).unwrap();
    assert_eq!(student.name, "홍길동", "재시작 후 학생 이름이 올바르게 복호화되어야 한다");

    let record = grid
        .records
        .iter()
        .find(|r| r.student_id == stu_id && r.activity_id == act_id)
        .unwrap();
    assert_eq!(record.content, "홍길동 발표 내용", "재시작 후 기록 내용이 올바르게 복호화되어야 한다");

    drop(conn2);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_change_password_then_reload() {
    let (db_path_state, dir) = setup_temp_db_path_state();
    let db_path = db_path_state.0.lock().unwrap().clone().unwrap();

    // 1. 파일 DB 초기화 및 암호화 활성화
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(include_str!("../schema.sql")).unwrap();
    let stu_id = insert_student(&conn, 1, 1, 1, "김철수");
    let area_id = insert_area(&conn, "수학", 500);
    let act_id = insert_activity(&conn, "수행평가");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, ?3)",
        rusqlite::params![act_id, stu_id, "수행평가 우수"],
    )
    .unwrap();

    let crypto = crypto_state(None);
    enable_encryption_impl(&conn, &crypto, &db_path_state, "password").unwrap();

    // 2. 비밀번호 변경
    change_encryption_password_impl(&conn, &crypto, &db_path_state, "password", "new-password")
        .unwrap();

    // 3. Connection 닫기 (재시작 시뮬레이션)
    drop(conn);

    // 4. 새 Connection으로 재오픈
    let conn2 = Connection::open(&db_path).unwrap();
    conn2.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    let crypto2 = crypto_state(None);

    // 5. 구 비밀번호로 unlock → 실패
    assert!(
        unlock_encryption_impl(&conn2, &crypto2, "password").is_err(),
        "변경 전 비밀번호로는 unlock이 실패해야 한다"
    );

    // 6. 새 비밀번호로 unlock → 성공
    unlock_encryption_impl(&conn2, &crypto2, "new-password").unwrap();
    let key = resolve_data_key(&conn2, &crypto2).unwrap().unwrap();

    // 7. 데이터 복호화 확인
    let grid = get_area_grid_impl(&conn2, area_id, Some(key)).unwrap();
    let student = grid.students.iter().find(|s| s.id == stu_id).unwrap();
    assert_eq!(student.name, "김철수", "비밀번호 변경 후 학생 이름이 올바르게 복호화되어야 한다");

    let record = grid
        .records
        .iter()
        .find(|r| r.student_id == stu_id && r.activity_id == act_id)
        .unwrap();
    assert_eq!(record.content, "수행평가 우수", "비밀번호 변경 후 기록 내용이 올바르게 복호화되어야 한다");

    drop(conn2);
    std::fs::remove_dir_all(dir).unwrap();
}

// ── 빈 패스워드 거부 테스트 ───────────────────────────────────────────

#[test]
fn test_enable_encryption_rejects_empty_password() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let result = enable_encryption_impl(&conn, &crypto, &db_path, "");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("비밀번호"));
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_change_password_rejects_empty_new_password() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let result = change_encryption_password_impl(&conn, &crypto, &db_path, "password", "");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("비밀번호"));
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 레거시 스냅샷 × 암호화 활성화 경계 테스트 ────────────────────────
//
// 실제 발생 가능한 시나리오:
//   기존 버전(암호화 없음)에서 스냅샷을 생성한 뒤,
//   신규 버전에서 암호화를 활성화하고 스냅샷을 복원하는 경우.
//
// 동작 근거:
//   enable_encryption_impl이 ActivityRecordHistory.content도 암호화하므로,
//   복원 시 history에서 읽히는 값은 암호화된 상태이고 get_area_grid_impl이
//   키로 복호화하여 원래 평문을 반환한다.

#[test]
fn test_restore_plaintext_snapshot_after_encryption_enabled() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();

    // 1. 암호화 없이 v1 기록 및 스냅샷 생성 (레거시 동작)
    upsert_record_impl(&conn, act_id, stu_id, "v1 내용", None).unwrap();
    let snapshot = create_snapshot_impl(&conn, Some("v1 스냅샷".to_string())).unwrap();

    // 2. 암호화 없이 v2로 덮어쓰기
    upsert_record_impl(&conn, act_id, stu_id, "v2 내용", None).unwrap();

    // 3. 암호화 활성화
    //    → ActivityRecord.content AND ActivityRecordHistory.content 모두 암호화됨
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap().unwrap();

    // 4. 암호화 상태에서 현재 값이 v2인지 확인
    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.records[0].content, "v2 내용", "암호화 활성화 후 현재 값은 v2여야 한다");

    // 5. 암호화 전 생성된 스냅샷으로 복원
    //    restore_snapshot_impl은 암호화된 history에서 읽어 ActivityRecord에 복사
    restore_snapshot_impl(&conn, snapshot.id).unwrap();

    // 6. 복원 후 v1이 복호화되어 나와야 한다
    let grid_restored = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(
        grid_restored.records[0].content, "v1 내용",
        "암호화 전 스냅샷 복원 후에도 올바른 v1 평문이 반환되어야 한다"
    );

    // 7. DB에 암호화된 값이 저장되어 있어야 한다 (복원 후에도 암호화 유지)
    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw, "v1 내용", "복원 후에도 DB에는 암호화된 값이 저장되어야 한다");

    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ═══════════════════════════════════════════════════════════════════
// 추가 엣지 케이스 · 상태 전이 · 실사용 시나리오 (32개)
// ═══════════════════════════════════════════════════════════════════

// ── 이중 활성화 / 이중 비활성화 ──────────────────────────────────

#[test]
fn test_enable_encryption_when_already_enabled_returns_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let result = enable_encryption_impl(&conn, &crypto, &db_path, "password");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("이미 암호화가 활성화"),
        "에러 메시지 확인"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_disable_encryption_when_not_enabled_returns_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let result = disable_encryption_impl(&conn, &crypto, &db_path);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("암호화가 활성화되어 있지 않습니다"),
        "에러 메시지 확인"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_disable_encryption_twice_returns_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();
    let result = disable_encryption_impl(&conn, &crypto, &db_path);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("암호화가 활성화되어 있지 않습니다"),
        "에러 메시지 확인"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── unlock 오류 케이스 ────────────────────────────────────────────

#[test]
fn test_unlock_with_wrong_password_returns_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "correct_password").unwrap();
    clear_crypto_state(&crypto).unwrap();
    let result = unlock_encryption_impl(&conn, &crypto, "wrong_password");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("비밀번호가 올바르지 않습니다"),
        "에러 메시지 확인"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_unlock_with_missing_salt_returns_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    // encryption_enabled=true만 있고 salt 없는 비정상 DB
    set_config_impl(&conn, "encryption_enabled", "true").unwrap();
    let result = unlock_encryption_impl(&conn, &crypto, "any_password");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("암호화 설정이 없습니다"),
        "에러 메시지 확인"
    );
}

#[test]
fn test_unlock_with_missing_token_returns_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    // salt만 있고 verify token이 없는 비정상 DB
    set_config_impl(&conn, "encryption_enabled", "true").unwrap();
    set_config_impl(
        &conn,
        "encryption_pbkdf2_salt",
        &B64.encode([1u8; 16]),
    )
    .unwrap();
    let result = unlock_encryption_impl(&conn, &crypto, "any_password");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("검증 토큰이 없습니다"),
        "에러 메시지 확인"
    );
}

#[test]
fn test_unlock_with_corrupted_token_returns_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    // 유효한 형식처럼 보이지만 내용은 쓰레기인 token
    set_config_impl(&conn, "encryption_enabled", "true").unwrap();
    set_config_impl(
        &conn,
        "encryption_pbkdf2_salt",
        &B64.encode([42u8; 16]),
    )
    .unwrap();
    set_config_impl(
        &conn,
        "encryption_verify_token",
        "AAAAAAAAAAAAAAAA:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    )
    .unwrap();
    let result = unlock_encryption_impl(&conn, &crypto, "any_password");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("비밀번호가 올바르지 않습니다") || err.contains("복호화 실패"),
        "에러 메시지: {err}"
    );
}

// ── 특수 문자 포함 이름 / content ────────────────────────────────

#[test]
fn test_student_name_with_colon_encrypt_decrypt_roundtrip() {
    let conn = setup_test_db();
    let key = test_key();
    let id = create_student_impl(&conn, 1, 1, 1, "홍:길동", Some(key)).unwrap();
    let raw_name: String = conn
        .query_row(
            "SELECT name FROM Student WHERE id=?1",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw_name, "홍:길동", "콜론 포함 이름도 암호화되어야 한다");
    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert_eq!(students[0].name, "홍:길동", "콜론 포함 이름이 복호화되어야 한다");
}

#[test]
fn test_record_content_with_newline_tab_roundtrip() {
    let conn = setup_test_db();
    let key = test_key();
    let area_id = insert_area(&conn, "테스트영역", 5000);
    let act_id = insert_activity(&conn, "발표");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    let content = "발표\n내용\t탭포함\n두번째줄";
    upsert_record_impl(&conn, act_id, stu_id, content, Some(key)).unwrap();
    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw, content, "개행·탭 포함 content도 암호화되어야 한다");
    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.records[0].content, content, "복호화 후 개행·탭이 보존되어야 한다");
}

// ── change_password 케이스 ────────────────────────────────────────

#[test]
fn test_change_password_same_password_succeeds() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    enable_encryption_impl(&conn, &crypto, &db_path, "same_password").unwrap();
    let result =
        change_encryption_password_impl(&conn, &crypto, &db_path, "same_password", "same_password");
    assert!(result.is_ok(), "동일 비밀번호 변경도 성공해야 한다: {:?}", result);
    clear_crypto_state(&crypto).unwrap();
    unlock_encryption_impl(&conn, &crypto, "same_password").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert!(students.iter().any(|s| s.id == stu_id && s.name == "홍길동"));
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_change_password_wrong_old_password_returns_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "correct_password").unwrap();
    let result = change_encryption_password_impl(
        &conn,
        &crypto,
        &db_path,
        "wrong_password",
        "new_password",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("현재 비밀번호가 올바르지 않습니다"));
    // 데이터 미변조: 기존 비밀번호로 여전히 unlock 가능해야 한다
    clear_crypto_state(&crypto).unwrap();
    unlock_encryption_impl(&conn, &crypto, "correct_password").unwrap();
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 다중 스냅샷 복원 정합성 ───────────────────────────────────────

#[test]
fn test_multiple_snapshots_restore_integrity_after_encryption() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    // 암호화 없이 v1, v2, v3 스냅샷 생성
    // 인메모리 DB에서 datetime('now')는 1초 단위라 같은 타임스탬프가 나올 수 있으므로
    // updated_at / created_at을 명시적으로 다르게 설정해 스냅샷 복원 로직이 정확히 동작하게 한다.
    upsert_record_impl(&conn, act_id, stu_id, "v1 내용", None).unwrap();
    conn.execute(
        "UPDATE ActivityRecord SET updated_at = '2024-01-01 00:00:01' WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    ).unwrap();
    let snap1 = create_snapshot_impl(&conn, Some("v1".to_string())).unwrap();
    conn.execute(
        "UPDATE Snapshot SET created_at = '2024-01-01 00:00:01' WHERE id=?1",
        rusqlite::params![snap1.id],
    ).unwrap();

    upsert_record_impl(&conn, act_id, stu_id, "v2 내용", None).unwrap();
    conn.execute(
        "UPDATE ActivityRecord SET updated_at = '2024-01-01 00:00:02' WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    ).unwrap();
    let snap2 = create_snapshot_impl(&conn, Some("v2".to_string())).unwrap();
    conn.execute(
        "UPDATE Snapshot SET created_at = '2024-01-01 00:00:02' WHERE id=?1",
        rusqlite::params![snap2.id],
    ).unwrap();

    upsert_record_impl(&conn, act_id, stu_id, "v3 내용", None).unwrap();
    conn.execute(
        "UPDATE ActivityRecord SET updated_at = '2024-01-01 00:00:03' WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    ).unwrap();
    let snap3 = create_snapshot_impl(&conn, Some("v3".to_string())).unwrap();
    conn.execute(
        "UPDATE Snapshot SET created_at = '2024-01-01 00:00:03' WHERE id=?1",
        rusqlite::params![snap3.id],
    ).unwrap();
    // 암호화 활성화 → 모든 history / record 암호화
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    // 순서를 섞어 복원해도 올바른 내용이 나와야 한다
    restore_snapshot_impl(&conn, snap1.id).unwrap();
    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.records[0].content, "v1 내용", "snap1 복원 후 v1이어야 한다");
    restore_snapshot_impl(&conn, snap3.id).unwrap();
    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.records[0].content, "v3 내용", "snap3 복원 후 v3이어야 한다");
    restore_snapshot_impl(&conn, snap2.id).unwrap();
    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.records[0].content, "v2 내용", "snap2 복원 후 v2이어야 한다");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 대량 데이터 enable / disable 왕복 ────────────────────────────

#[test]
fn test_large_dataset_enable_disable_roundtrip() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let area_id = insert_area(&conn, "국어", 5000);
    let act_ids: Vec<i64> = (0..3).map(|i| insert_activity(&conn, &format!("활동{i}"))).collect();
    for i in 0..10i64 {
        let stu_id = insert_student(&conn, 1, 1, i + 1, &format!("학생{:02}", i + 1));
        conn.execute(
            "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
            rusqlite::params![area_id, stu_id],
        )
        .unwrap();
        for &act_id in &act_ids {
            conn.execute(
                "INSERT OR IGNORE INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
                rusqlite::params![area_id, act_id],
            )
            .unwrap();
            upsert_record_impl(&conn, act_id, stu_id, &format!("학생{} 기록", i + 1), None)
                .unwrap();
        }
    }
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert_eq!(students.len(), 10);
    for (i, s) in students.iter().enumerate() {
        assert_eq!(s.name, format!("학생{:02}", i + 1));
    }
    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();
    let students_plain = get_students_impl(&conn, None).unwrap();
    assert_eq!(students_plain.len(), 10);
    for (i, s) in students_plain.iter().enumerate() {
        assert_eq!(s.name, format!("학생{:02}", i + 1));
    }
    let grid = get_area_grid_impl(&conn, area_id, None).unwrap();
    assert_eq!(grid.records.len(), 30, "30개 레코드가 모두 복원되어야 한다");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 동일 위치 반복 upsert ─────────────────────────────────────────

#[test]
fn test_upsert_record_multiple_times_each_decryptable() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "1차 기록", Some(key)).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "2차 기록", Some(key)).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "3차 기록", Some(key)).unwrap();
    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw, "3차 기록", "최신 content도 암호화되어야 한다");
    let decrypted = crate::crypto::decrypt(&raw, &key).unwrap();
    assert_eq!(decrypted, "3차 기록", "마지막 upsert 값이 복호화되어야 한다");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "ActivityRecord는 항상 1행이어야 한다");
}

// ── 잠금 상태 전이 수명주기 ───────────────────────────────────────

#[test]
fn test_locked_state_prevents_resolve_data_key() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    clear_crypto_state(&crypto).unwrap();
    let err = resolve_data_key(&conn, &crypto).unwrap_err();
    assert!(err.contains("잠금"), "잠금 에러 메시지: {err}");
    unlock_encryption_impl(&conn, &crypto, "password").unwrap();
    let key_result = resolve_data_key(&conn, &crypto);
    assert!(key_result.is_ok());
    assert!(key_result.unwrap().is_some(), "unlock 후 key는 Some이어야 한다");
    clear_crypto_state(&crypto).unwrap();
    let err2 = resolve_data_key(&conn, &crypto).unwrap_err();
    assert!(err2.contains("잠금"), "재잠금 후 에러 메시지: {err2}");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── enable → disable → enable 재활성화 ───────────────────────────

#[test]
fn test_enable_disable_reenable_works() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    enable_encryption_impl(&conn, &crypto, &db_path, "password1").unwrap();
    let key1 = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    assert_eq!(get_students_impl(&conn, Some(key1)).unwrap()[0].name, "홍길동");
    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();
    assert_eq!(get_students_impl(&conn, None).unwrap()[0].name, "홍길동");
    enable_encryption_impl(&conn, &crypto, &db_path, "password2").unwrap();
    let key2 = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    assert_ne!(key1, key2, "새 활성화는 새 키를 생성해야 한다");
    let students2 = get_students_impl(&conn, Some(key2)).unwrap();
    assert!(students2.iter().any(|s| s.id == stu_id && s.name == "홍길동"));
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 잠금 상태에서 change_password ────────────────────────────────

#[test]
fn test_change_password_while_locked_succeeds() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    enable_encryption_impl(&conn, &crypto, &db_path, "old_pw").unwrap();
    clear_crypto_state(&crypto).unwrap();
    // 잠금 상태에서도 구 비밀번호로 change_password 성공 (salt/token 직접 검증)
    let result = change_encryption_password_impl(&conn, &crypto, &db_path, "old_pw", "new_pw");
    assert!(result.is_ok(), "잠금 상태에서도 비밀번호 변경 가능해야 한다: {:?}", result);
    clear_crypto_state(&crypto).unwrap();
    unlock_encryption_impl(&conn, &crypto, "new_pw").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert!(students.iter().any(|s| s.id == stu_id && s.name == "홍길동"));
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 이미 해제 상태에서 unlock 재호출 ─────────────────────────────

#[test]
fn test_unlock_when_already_unlocked_succeeds() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    // enable 직후 이미 해제 상태 – 재호출도 성공 (set_crypto_state 덮어쓰기)
    let result = unlock_encryption_impl(&conn, &crypto, "password");
    assert!(result.is_ok(), "이미 해제 상태에서 unlock 재호출도 성공해야 한다: {:?}", result);
    assert!(resolve_data_key(&conn, &crypto).unwrap().is_some());
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 긴 이름 암호화 ────────────────────────────────────────────────

#[test]
fn test_student_long_name_encrypt_decrypt_roundtrip() {
    let conn = setup_test_db();
    let key = test_key();
    let long_name = "가".repeat(500);
    let id = create_student_impl(&conn, 1, 1, 1, &long_name, Some(key)).unwrap();
    let raw_name: String = conn
        .query_row(
            "SELECT name FROM Student WHERE id=?1",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw_name, long_name, "긴 이름도 암호화되어야 한다");
    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert_eq!(students[0].name, long_name, "500자 이름이 복호화되어야 한다");
}

// ── 공백만 있는 content → maybe_encrypt 통과 ────────────────────

#[test]
fn test_record_content_spaces_only_gets_encrypted() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    let content = "     "; // 공백만 (is_empty() = false)
    upsert_record_impl(&conn, act_id, stu_id, content, Some(key)).unwrap();
    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw, content, "공백만 있는 content도 암호화되어야 한다");
    let decrypted = crate::crypto::decrypt(&raw, &key).unwrap();
    assert_eq!(decrypted, content);
}

// ── APP_CONFIGS 항목 생성 / 삭제 검증 ────────────────────────────

#[test]
fn test_enable_encryption_creates_config_entries() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let before: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM APP_CONFIGS WHERE config_key LIKE 'encryption%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(before, 0);
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM APP_CONFIGS WHERE config_key LIKE 'encryption%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(after, 3, "활성화 후 3개 config 항목(enabled, salt, token)이 생성되어야 한다");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_disable_encryption_removes_config_entries() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM APP_CONFIGS WHERE config_key LIKE 'encryption%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0, "비활성화 후 encryption config 항목이 모두 삭제되어야 한다");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── history 다건 복호화 ───────────────────────────────────────────

#[test]
fn test_record_history_multiple_entries_all_decryptable() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "최신 내용", Some(key)).unwrap();
    let record_id: i64 = conn
        .query_row(
            "SELECT id FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    for (i, content) in ["이전 내용1", "이전 내용2", "이전 내용3"].iter().enumerate() {
        let encrypted = crate::crypto::encrypt(content, &key).unwrap();
        conn.execute(
            "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note) VALUES (?1, ?2, ?3, NULL)",
            rusqlite::params![record_id, encrypted, format!("2024-01-{:02} 10:00:00", i + 1)],
        )
        .unwrap();
    }
    let history = get_record_history_impl(&conn, act_id, stu_id, 10, 0, Some(key)).unwrap();
    assert_eq!(history.len(), 3, "history가 3개이어야 한다");
    let contents: Vec<&str> = history.iter().map(|h| h.content.as_str()).collect();
    assert!(contents.contains(&"이전 내용1"));
    assert!(contents.contains(&"이전 내용2"));
    assert!(contents.contains(&"이전 내용3"));
}

// ── 정규식 치환 + 암호화 조합 ─────────────────────────────────────

#[test]
fn test_apply_replace_regex_with_encryption() {
    let conn = setup_test_db();
    let key = test_key();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "발표");
    let stu_id = create_student_impl(&conn, 1, 1, 1, "홍길동", Some(key)).unwrap();
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "홍길동은 100점을 받았다", Some(key)).unwrap();
    // 정규식으로 숫자 → N 치환
    create_replace_rule_db(&conn, r"\d+", "N", true, 1).unwrap();
    let mut cache = make_cache();
    let result = apply_replace_impl(&conn, "all", &[], Some(key), &mut cache).unwrap();
    assert_eq!(result.changed_count, 1);
    let grid = get_area_grid_impl(&conn, area_id, Some(key)).unwrap();
    assert_eq!(grid.records[0].content, "홍길동은 N점을 받았다");
    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw, "홍길동은 N점을 받았다", "치환 결과도 암호화되어야 한다");
}

// ── get_encryption_status 3가지 상태 ─────────────────────────────

#[test]
fn test_get_encryption_status_all_states() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let s = get_encryption_status_impl(&conn, &crypto).unwrap();
    assert!(!s.enabled && !s.unlocked, "비활성 상태: enabled=false, unlocked=false");
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let s = get_encryption_status_impl(&conn, &crypto).unwrap();
    assert!(s.enabled && s.unlocked, "활성+해제: enabled=true, unlocked=true");
    clear_crypto_state(&crypto).unwrap();
    let s = get_encryption_status_impl(&conn, &crypto).unwrap();
    assert!(s.enabled && !s.unlocked, "활성+잠금: enabled=true, unlocked=false");
    unlock_encryption_impl(&conn, &crypto, "password").unwrap();
    let s = get_encryption_status_impl(&conn, &crypto).unwrap();
    assert!(s.enabled && s.unlocked, "재해제: enabled=true, unlocked=true");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 잘못된 비밀번호 여러 번 후 성공 ──────────────────────────────

#[test]
fn test_multiple_wrong_then_correct_unlock() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "correct_pw").unwrap();
    clear_crypto_state(&crypto).unwrap();
    for _ in 0..3 {
        assert!(unlock_encryption_impl(&conn, &crypto, "wrong_pw").is_err());
    }
    unlock_encryption_impl(&conn, &crypto, "correct_pw").unwrap();
    assert!(resolve_data_key(&conn, &crypto).unwrap().is_some());
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 미활성 상태에서 change_password ──────────────────────────────

#[test]
fn test_change_password_when_not_enabled_returns_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let result = change_encryption_password_impl(&conn, &crypto, &db_path, "old", "new");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("암호화 설정이 없습니다"));
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 빈 content는 enable / disable 후에도 빈 문자열 유지 ───────────

#[test]
fn test_record_empty_content_preserved_through_enable_disable() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "", None).unwrap();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let after_enable: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(after_enable, "", "빈 content는 enable 후에도 빈 문자열이어야 한다 (skip_empty=true)");
    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();
    let after_disable: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(after_disable, "", "빈 content는 disable 후에도 빈 문자열이어야 한다");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 20명 대량 bulk_import 후 각각 복호화 ──────────────────────────

#[test]
fn test_bulk_import_large_batch_all_decryptable() {
    let conn = setup_test_db();
    let key = test_key();
    let act_id = insert_activity(&conn, "발표");
    let names: Vec<String> = (1..=20i64).map(|i| format!("학생{:02}", i)).collect();
    let contents: Vec<String> = (1..=20i64).map(|i| format!("내용{}", i)).collect();
    let inputs: Vec<_> = (0..20usize)
        .map(|i| make_import(1, 1, (i + 1) as i64, Some(names[i].as_str()), act_id, &contents[i]))
        .collect();
    bulk_import_records_impl(&conn, &inputs, Some(key)).unwrap();
    let raw_names: Vec<String> = {
        let mut stmt = conn.prepare("SELECT name FROM Student ORDER BY number").unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    for (i, raw) in raw_names.iter().enumerate() {
        assert_ne!(raw, &names[i], "DB에 평문이 저장되면 안 된다: {raw}");
    }
    let students = get_students_impl(&conn, Some(key)).unwrap();
    assert_eq!(students.len(), 20);
    for (i, s) in students.iter().enumerate() {
        assert_eq!(s.name, format!("학생{:02}", i + 1));
    }
    let records = get_all_records_for_inspect_impl(&conn, "all", vec![], Some(key)).unwrap();
    assert_eq!(records.len(), 20);
    for i in 1..=20i64 {
        let expected = format!("내용{}", i);
        assert!(records.iter().any(|r| r.content == expected), "내용{}이 없다", i);
    }
}

// ── 비밀번호 변경 후 스냅샷 복원 정합성 ──────────────────────────

#[test]
fn test_snapshot_after_change_password_still_restorable() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    )
    .unwrap();
    enable_encryption_impl(&conn, &crypto, &db_path, "old_password").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "v1 기록", Some(key)).unwrap();
    let snap = create_snapshot_impl(&conn, Some("v1".to_string())).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "v2 기록", Some(key)).unwrap();
    // 비밀번호 변경 → 모든 데이터 재암호화
    change_encryption_password_impl(&conn, &crypto, &db_path, "old_password", "new_password")
        .unwrap();
    let new_key = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    // 스냅샷 복원 → v1 기록이 새 키로 복호화되어야 한다
    restore_snapshot_impl(&conn, snap.id).unwrap();
    let grid = get_area_grid_impl(&conn, area_id, Some(new_key)).unwrap();
    assert_eq!(
        grid.records[0].content, "v1 기록",
        "비밀번호 변경 후 스냅샷 복원이 올바르게 동작해야 한다"
    );
    let raw: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw, "v1 기록", "복원 후에도 DB에는 암호화된 값이어야 한다");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 비밀번호 변경 후 inspect 복호화 ──────────────────────────────

#[test]
fn test_inspect_after_change_password_decryptable() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    enable_encryption_impl(&conn, &crypto, &db_path, "old_pw").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "검사 내용", Some(key)).unwrap();
    change_encryption_password_impl(&conn, &crypto, &db_path, "old_pw", "new_pw").unwrap();
    let new_key = resolve_data_key(&conn, &crypto).unwrap().unwrap();
    // 새 키로 inspect → 복호화된 원문
    let records = get_all_records_for_inspect_impl(&conn, "all", vec![], Some(new_key)).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "검사 내용", "비밀번호 변경 후 inspect가 복호화되어야 한다");
    // 구 키로 inspect → 에러 (재암호화된 데이터를 구 키로 복호화 불가)
    let result_old = get_all_records_for_inspect_impl(&conn, "all", vec![], Some(key));
    assert!(result_old.is_err(), "구 키로 조회하면 에러여야 한다");
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── update_student 후 단일 암호화 확인 ───────────────────────────

#[test]
fn test_update_student_name_reencrypted_not_double_encrypted() {
    let conn = setup_test_db();
    let key = test_key();
    let id = create_student_impl(&conn, 1, 1, 1, "원래이름", Some(key)).unwrap();
    update_student_impl(&conn, id, 1, 1, 1, "새이름", Some(key)).unwrap();
    let raw_name: String = conn
        .query_row(
            "SELECT name FROM Student WHERE id=?1",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .unwrap();
    // 1번 복호화하면 "새이름"이어야 한다
    let decrypted = crate::crypto::decrypt(&raw_name, &key).unwrap();
    assert_eq!(decrypted, "새이름", "update_student 후 이름이 정확히 1번 암호화되어야 한다");
    // 2번 복호화하면 실패 (이중 암호화가 아님)
    assert!(
        crate::crypto::decrypt(&decrypted, &key).is_err(),
        "이중 암호화가 아니어야 한다"
    );
}
