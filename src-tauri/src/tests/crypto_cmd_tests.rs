use super::{insert_activity, insert_area, insert_student, setup_temp_db_path_state, setup_test_db};
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
    std::sync::Mutex::new(CryptoState { key, salt: None })
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
    assert!(raw_name.contains(':'), "nonce:ciphertext 형식이어야 한다");
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
    assert!(raw.contains(':'), "nonce:ciphertext 형식이어야 한다");
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
    assert!(encrypted_content.contains(':'));
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
    assert!(raw_name.contains(':'), "nonce:ciphertext 형식이어야 한다");

    let raw_content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1",
            rusqlite::params![act_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_ne!(raw_content, "발표 내용", "content가 평문으로 저장되면 안 된다");
    assert!(raw_content.contains(':'));

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
    assert!(raw_name.contains(':'));
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
    assert!(raw.contains(':'), "재암호화된 nonce:ciphertext 형식이어야 한다");

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

    // history가 존재하며 모든 항목이 복호화된 평문이어야 한다
    let history = get_record_history_impl(&conn, act_id, stu_id, 10, 0, Some(key)).unwrap();
    assert!(!history.is_empty(), "치환 적용 후 history가 있어야 한다");
    for entry in &history {
        assert!(
            !entry.content.contains(':') || entry.content.is_empty(),
            "history content가 암호화된 ciphertext 형식이면 안 된다: {}",
            entry.content
        );
    }
    // 가장 최근 history의 content는 치환된 결과여야 한다
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
    let raw_names: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM Student ORDER BY number")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    for name in &raw_names {
        assert!(name.contains(':'), "DB 이름이 암호화 형식이어야 한다: {name}");
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
    assert!(raw_history.contains(':'));
}
