use super::{insert_activity, insert_area, insert_student, setup_temp_db_path_state, setup_test_db};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rusqlite::Connection;
use crate::commands::config::set_config_impl;
use crate::commands::crypto::{
    change_encryption_password_impl, combine_all, disable_encryption_impl, enable_encryption_impl,
    get_encryption_status_impl, is_purge_pending, purge_free_pages, resolve_data_key,
    resume_pending_purge, retry_pending_purge_impl, unlock_encryption_impl,
    vacuum_into_backup, with_purge_marked_transaction,
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
use crate::crypto::{derive_key, encrypt, generate_salt};
use crate::engine::get_records_for_scope;
use crate::state::{clear_crypto_state, CryptoState, CryptoStateHandle, DbPathState, DbState};
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
    save_snapshot_internal(&conn, act_id, stu_id, Some("before encryption"), None).unwrap();

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

/// tmp_dir 안에 주어진 접미사를 가진 백업 파일이 있는지 확인한다.
fn backup_exists(tmp_dir: &std::path::Path, suffix: &str) -> bool {
    std::fs::read_dir(tmp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().contains(suffix))
}

#[test]
fn test_enable_encryption_removes_plaintext_backup() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);

    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();

    // 평문 사본이 DB 옆에 남으면 암호화를 켠 의미가 없다.
    assert!(
        !backup_exists(&tmp_dir, "-pre-encrypt"),
        "암호화에 성공하면 평문 백업이 남아 있으면 안 된다"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_change_password_removes_old_key_backup() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);

    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    change_encryption_password_impl(&conn, &crypto, &db_path, "password", "new-password").unwrap();

    // 이 백업은 옛 비밀번호로 계속 열리므로 남기면 비밀번호 변경이 무의미해진다.
    assert!(
        !backup_exists(&tmp_dir, "-pre-reencrypt"),
        "비밀번호 변경에 성공하면 옛 키로 열리는 백업이 남아 있으면 안 된다"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_disable_encryption_keeps_backup() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);

    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();

    // 암호문 사본이고 본 DB가 평문이 되므로, 실수 복구용 안전망으로 남긴다.
    assert!(
        backup_exists(&tmp_dir, "-pre-decrypt"),
        "암호화 해제 전 백업은 남아 있어야 한다"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_disable_encryption_clears_key_from_memory() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();

    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();

    assert!(
        crate::state::current_crypto_key(&crypto).unwrap().is_none(),
        "암호화를 끄면 키가 메모리에 남으면 안 된다"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_status_not_unlocked_for_plain_file_even_with_key() {
    // 다른 파일에서 쓰던 키가 남아 있어도, 암호화를 쓰지 않는 파일은 "잠금 해제됨"이 아니다.
    let conn = setup_test_db();
    let crypto = crypto_state(Some(test_key()));

    let status = get_encryption_status_impl(&conn, &crypto).unwrap();

    assert!(!status.enabled);
    assert!(!status.unlocked);
}

#[test]
fn test_unlock_rejects_token_with_wrong_plaintext() {
    // 키로 풀리기만 하면 통과시키지 않는다. 풀린 값이 검증 문자열이어야 한다.
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    set_config_impl(&conn, "encryption_enabled", "true").unwrap();
    set_config_impl(&conn, "encryption_pbkdf2_salt", &B64.encode([42u8; 16])).unwrap();
    let token = encrypt("다른 값", &test_key()).unwrap();
    set_config_impl(&conn, "encryption_verify_token", &token).unwrap();

    assert!(unlock_encryption_impl(&conn, &crypto, "password").is_err());
    assert!(crate::state::current_crypto_key(&crypto).unwrap().is_none());
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
    let snapshot = create_snapshot_impl(&conn, Some("v1 스냅샷".to_string()), Some(key)).unwrap();

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

    // 치환 전 원본 content가 history에 암호화된 채로 저장되고 복호화 가능해야 한다
    let history = get_record_history_impl(&conn, act_id, stu_id, 10, 0, Some(key)).unwrap();
    assert!(!history.is_empty(), "치환 적용 시 history가 생성되어야 한다");
    assert_eq!(history[0].content, "가나다 발표", "history에는 치환 전 원본 content가 저장되어야 한다");
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

    // apply_replace는 치환 전 원본 content를 history에 저장한다.
    // upsert_record_impl만으로는 history가 생성되지 않으므로 항목은 1개여야 한다.
    let history = get_record_history_impl(&conn, act_id, stu_id, 10, 0, Some(key)).unwrap();
    assert_eq!(history.len(), 1, "apply_replace 후 history는 정확히 1개여야 한다");
    assert_eq!(history[0].content, "원본 내용 ABC", "history[0]에는 치환 전 원본 content가 저장되어야 한다");
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
    // db::create_new와 같은 상태로 맞춘다 — resolve_data_key가 이 값으로
    // "마이그레이션이 끝난 파일인가"를 판단한다.
    conn.pragma_update(None, "user_version", crate::db::SCHEMA_VERSION)
        .unwrap();
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
    // db::create_new와 같은 상태로 맞춘다 — resolve_data_key가 이 값으로
    // "마이그레이션이 끝난 파일인가"를 판단한다.
    conn.pragma_update(None, "user_version", crate::db::SCHEMA_VERSION)
        .unwrap();
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

// ── 비밀번호 최소 길이 ───────────────────────────────────

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

#[test]
fn test_enable_encryption_rejects_password_below_minimum() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    let result = enable_encryption_impl(&conn, &crypto, &db_path, "123");

    assert!(result.is_err(), "3자는 거부되어야 한다");
    // 검사에서 막혔으면 백업도 만들지 않아야 한다.
    assert!(
        !backup_exists(&tmp_dir, "-pre-encrypt"),
        "길이 검사에서 막힌 요청이 백업을 남기면 안 된다"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_enable_encryption_accepts_exactly_minimum_length() {
    // 하한은 4자 "미만"을 막는다. 4자(핀 번호 길이)는 허용이다.
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    enable_encryption_impl(&conn, &crypto, &db_path, "1234").unwrap();

    assert!(get_encryption_status_impl(&conn, &crypto).unwrap().enabled);
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_change_password_rejects_new_password_below_minimum() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();

    let result = change_encryption_password_impl(&conn, &crypto, &db_path, "password", "abc");

    assert!(result.is_err(), "새 비밀번호 3자는 거부되어야 한다");
    // 거부됐으면 옛 비밀번호가 그대로 살아 있어야 한다.
    clear_crypto_state(&crypto).unwrap();
    unlock_encryption_impl(&conn, &crypto, "password").unwrap();
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_password_minimum_counts_characters_not_bytes() {
    // "가나다"는 UTF-8로 9바이트지만 3글자다. 바이트 길이로 세면 통과해버려
    // 한글 사용자에게만 하한이 사라진다.
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    assert!(
        enable_encryption_impl(&conn, &crypto, &db_path, "가나다").is_err(),
        "한글 3글자도 거부되어야 한다"
    );
    enable_encryption_impl(&conn, &crypto, &db_path, "가나다라").unwrap();

    std::fs::remove_dir_all(&tmp_dir).ok();
}

/// 하한이 생기기 전에 만든 짧은 비밀번호 파일은 계속 열려야 한다.
///
/// 하한을 unlock에도 걸면, 사용자는 올바른 비밀번호를 알면서도 자기 파일을 영영
/// 열 수 없게 된다. enable_encryption_impl로는 이제 3자짜리를 만들 수 없으므로
/// salt와 검증 토큰을 직접 써 넣어 그런 파일을 재현한다.
#[test]
fn test_unlock_accepts_short_password_created_before_minimum() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);

    let salt = generate_salt();
    let key = derive_key("123", &salt);
    // crypto.rs의 VERIFY_PLAINTEXT와 같은 값 — 저장 형식을 함께 고정한다.
    let token = encrypt("school-record-verify", &key).unwrap();
    set_config_impl(&conn, "encryption_pbkdf2_salt", &B64.encode(salt)).unwrap();
    set_config_impl(&conn, "encryption_verify_token", &token).unwrap();
    set_config_impl(&conn, "encryption_enabled", "true").unwrap();

    unlock_encryption_impl(&conn, &crypto, "123").unwrap();

    assert!(
        resolve_data_key(&conn, &crypto).unwrap().is_some(),
        "짧은 비밀번호로 만든 기존 파일도 열려야 한다"
    );
}

// ── 정리(VACUUM) 재시도 표시 ──────────────────────────────

#[test]
fn test_enable_encryption_clears_purge_pending_on_success() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();

    assert!(
        !is_purge_pending(&conn).unwrap(),
        "정리까지 끝났으면 표시가 남아 있으면 안 된다"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_change_password_clears_purge_pending_on_success() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    change_encryption_password_impl(&conn, &crypto, &db_path, "password", "new-password").unwrap();

    assert!(!is_purge_pending(&conn).unwrap());
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn test_resume_pending_purge_is_noop_without_flag() {
    let conn = setup_test_db();
    resume_pending_purge(&conn).unwrap();
    assert!(!is_purge_pending(&conn).unwrap());
}

#[test]
fn test_resume_pending_purge_clears_flag_and_freelist() {
    // 커밋은 끝났는데 VACUUM 직전에 프로세스가 죽은 상태를 흉내낸다.
    let conn = setup_test_db();
    for i in 0..500 {
        insert_activity(&conn, &format!("활동{i}"));
    }
    conn.execute_batch("DELETE FROM Activity;").unwrap();
    assert!(freelist_count(&conn) > 0, "free page가 있어야 테스트가 의미 있다");
    set_config_impl(&conn, "encryption_purge_pending", "암호화").unwrap();

    resume_pending_purge(&conn).unwrap();

    assert_eq!(freelist_count(&conn), 0, "밀린 정리가 실제로 실행되어야 한다");
    assert!(!is_purge_pending(&conn).unwrap(), "성공했으면 표시를 지워야 한다");
}

#[test]
fn test_resume_pending_purge_keeps_encrypted_data_readable() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "활동 기록", None).unwrap();

    let (db_path, tmp_dir) = setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    std::fs::remove_dir_all(&tmp_dir).ok();

    // 정리가 밀린 상태를 만들고 이어받는다.
    set_config_impl(&conn, "encryption_purge_pending", "암호화").unwrap();
    resume_pending_purge(&conn).unwrap();

    let key = resolve_data_key(&conn, &crypto).unwrap();
    assert_eq!(get_students_impl(&conn, key).unwrap()[0].name, "홍길동");
    let stored: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        crate::crypto::maybe_decrypt(stored, key).unwrap(),
        "활동 기록"
    );
    assert!(!is_purge_pending(&conn).unwrap());
}

#[test]
fn test_status_reports_purge_pending() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    assert!(
        !get_encryption_status_impl(&conn, &crypto)
            .unwrap()
            .purge_pending
    );

    set_config_impl(&conn, "encryption_purge_pending", "암호화").unwrap();

    assert!(
        get_encryption_status_impl(&conn, &crypto)
            .unwrap()
            .purge_pending,
        "정리가 밀려 있으면 화면에 알릴 수 있어야 한다"
    );
}

#[test]
fn test_retry_pending_purge_errors_when_nothing_pending() {
    // 무음 실패 금지 — 누를 것이 없으면 그렇다고 말해야 한다.
    let conn = setup_test_db();
    let err = retry_pending_purge_impl(&conn).unwrap_err();
    assert!(err.contains("정리할 항목이 없습니다"), "에러 메시지: {err}");
}

#[test]
fn test_purge_failure_keeps_pending_marker() {
    // VACUUM이 실패했는데 표시를 지우면 다시 시도할 근거가 사라져 잔재가 영구히 남는다.
    // 트랜잭션 안에서는 VACUUM이 실행되지 않으므로 그것으로 실패를 만든다.
    let conn = setup_test_db();
    set_config_impl(&conn, "encryption_purge_pending", "암호화").unwrap();
    conn.execute_batch("BEGIN").unwrap();

    let result = resume_pending_purge(&conn);
    let still_pending = is_purge_pending(&conn).unwrap();
    conn.execute_batch("ROLLBACK").unwrap();

    assert!(result.is_err(), "VACUUM 실패가 오류로 올라와야 한다");
    assert!(still_pending, "정리에 실패하면 표시가 남아 있어야 한다");
}

#[test]
fn test_retry_pending_purge_clears_flag() {
    let conn = setup_test_db();
    set_config_impl(&conn, "encryption_purge_pending", "비밀번호 변경").unwrap();
    retry_pending_purge_impl(&conn).unwrap();
    assert!(!is_purge_pending(&conn).unwrap());
}

/// 파일 DB 하나와, VACUUM을 막을 두 번째 연결을 만든다.
///
/// `BEGIN IMMEDIATE`는 RESERVED 락을 잡는다. 읽기는 통과하므로 파일을 여는 것은
/// 되지만, 쓰기가 필요한 VACUUM은 SQLITE_BUSY로 실패한다. 디스크가 가득 찬 상황을
/// 흉내내는 것보다 확실하고 빠르다.
///
/// 표시는 락을 걸기 **전에** 심는다. RESERVED가 잡힌 뒤에는 어떤 쓰기도 막히므로,
/// 나중에 심으려 하면 테스트하려는 VACUUM 실패가 아니라 준비 단계에서 넘어진다.
fn file_db_with_vacuum_blocked(pending: &str) -> (Connection, Connection, std::path::PathBuf) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "school_record_purge_fail_{}_{}",
        std::process::id(),
        nanos
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.db");
    let conn = crate::db::create_new(&path).unwrap();
    set_config_impl(&conn, "encryption_purge_pending", pending).unwrap();

    let blocker = Connection::open(&path).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    (conn, blocker, dir)
}

#[test]
fn test_with_purge_marked_transaction_commits_flag_with_the_change() {
    // 표시는 호출부가 고르는 것이 아니라 이 헬퍼가 항상 함께 커밋한다.
    let conn = setup_test_db();

    with_purge_marked_transaction(&conn, "암호화", || {
        insert_activity(&conn, "발표");
        Ok(())
    })
    .unwrap();

    assert!(is_purge_pending(&conn).unwrap(), "정리 표시가 커밋되어야 한다");
}

#[test]
fn test_purge_mark_rolls_back_with_the_change() {
    // 데이터 변경이 롤백되면 표시도 함께 사라져야 한다. 남으면 멀쩡한 파일에
    // 쓸데없는 VACUUM이 걸리고, "정리가 밀렸다"는 경고까지 뜬다.
    let conn = setup_test_db();

    let err = with_purge_marked_transaction(&conn, "암호화", || {
        insert_activity(&conn, "발표");
        Err("일부러 실패".to_string())
    })
    .unwrap_err();

    assert_eq!(err, "일부러 실패");
    assert!(!is_purge_pending(&conn).unwrap(), "롤백되면 표시도 없어야 한다");
    let activities: i64 = conn
        .query_row("SELECT COUNT(*) FROM Activity", [], |r| r.get(0))
        .unwrap();
    assert_eq!(activities, 0, "같은 트랜잭션이면 데이터도 함께 롤백된다");
}

#[test]
fn test_resume_pending_purge_keeps_flag_when_vacuum_fails() {
    // 실패했는데 표시를 지우면 다시 시도할 근거가 사라진다.
    let (conn, blocker, dir) = file_db_with_vacuum_blocked("암호화");

    let err = resume_pending_purge(&conn).unwrap_err();

    assert!(err.contains("암호화"), "무엇을 하다 남았는지 알려야 한다: {err}");
    assert!(
        is_purge_pending(&conn).unwrap(),
        "정리에 실패하면 표시가 남아 다음에 다시 시도되어야 한다"
    );

    blocker.execute_batch("ROLLBACK").unwrap();
    drop(blocker);
    drop(conn);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_retry_pending_purge_keeps_flag_when_vacuum_fails() {
    let (conn, blocker, dir) = file_db_with_vacuum_blocked("비밀번호 변경");

    assert!(retry_pending_purge_impl(&conn).is_err());
    assert!(is_purge_pending(&conn).unwrap());

    blocker.execute_batch("ROLLBACK").unwrap();
    drop(blocker);
    drop(conn);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_disable_encryption_keeps_purge_pending_flag() {
    // 암호화를 켜다 만 파일을 해제해도 잔재는 남아 있을 수 있다.
    // 표시를 남겨두면 다음에 열 때 정리된다. 손해는 VACUUM 한 번뿐이다.
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    set_config_impl(&conn, "encryption_purge_pending", "암호화").unwrap();
    disable_encryption_impl(&conn, &crypto, &db_path).unwrap();

    assert!(is_purge_pending(&conn).unwrap());
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
    let snapshot = create_snapshot_impl(&conn, Some("v1 스냅샷".to_string()), None).unwrap();

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
// 추가 엣지 케이스 · 상태 전이 · 실사용 시나리오
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
    let snap1 = create_snapshot_impl(&conn, Some("v1".to_string()), None).unwrap();
    conn.execute(
        "UPDATE Snapshot SET created_at = '2024-01-01 00:00:01' WHERE id=?1",
        rusqlite::params![snap1.id],
    ).unwrap();

    upsert_record_impl(&conn, act_id, stu_id, "v2 내용", None).unwrap();
    conn.execute(
        "UPDATE ActivityRecord SET updated_at = '2024-01-01 00:00:02' WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    ).unwrap();
    let snap2 = create_snapshot_impl(&conn, Some("v2".to_string()), None).unwrap();
    conn.execute(
        "UPDATE Snapshot SET created_at = '2024-01-01 00:00:02' WHERE id=?1",
        rusqlite::params![snap2.id],
    ).unwrap();

    upsert_record_impl(&conn, act_id, stu_id, "v3 내용", None).unwrap();
    conn.execute(
        "UPDATE ActivityRecord SET updated_at = '2024-01-01 00:00:03' WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    ).unwrap();
    let snap3 = create_snapshot_impl(&conn, Some("v3".to_string()), None).unwrap();
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
    // 길이 하한에 걸리지 않는 비밀번호를 써야 "설정이 없다"는 판정을 검증할 수 있다.
    let result =
        change_encryption_password_impl(&conn, &crypto, &db_path, "old-password", "new-password");
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
    let snap = create_snapshot_impl(&conn, Some("v1".to_string()), Some(key)).unwrap();
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

// ── combine_all: 마무리 작업 오류 합치기 ─────────────────────

#[test]
fn test_combine_all_reports_every_failure() {
    // 하나가 실패했다고 다음을 건너뛰면 평문 백업이나 free page 잔재가 남는다.
    // 모두 시도하고 실패한 것을 전부 알려야 한다.
    let err = combine_all([
        Err("키 실패".into()),
        Err("삭제 실패".into()),
        Err("정리 실패".into()),
    ])
    .unwrap_err();
    assert!(err.contains("키 실패"), "에러 메시지: {err}");
    assert!(err.contains("삭제 실패"), "에러 메시지: {err}");
    assert!(err.contains("정리 실패"), "에러 메시지: {err}");
}

#[test]
fn test_combine_all_reports_single_failure() {
    assert_eq!(
        combine_all([Ok(()), Err("삭제 실패".into()), Ok(())]).unwrap_err(),
        "삭제 실패"
    );
}

#[test]
fn test_combine_all_ok_when_all_succeed() {
    assert!(combine_all([Ok(()), Ok(()), Ok(())]).is_ok());
}

// ── purge_free_pages: 평문 잔재 제거 ─────────────────────────

fn freelist_count(conn: &rusqlite::Connection) -> i64 {
    conn.query_row("PRAGMA freelist_count", [], |r| r.get(0)).unwrap()
}

#[test]
fn test_purge_free_pages_clears_freelist() {
    let conn = setup_test_db();
    // 많이 넣었다 지워 free page를 실제로 만든다.
    for i in 0..500 {
        insert_activity(&conn, &format!("활동{i}"));
    }
    conn.execute_batch("DELETE FROM Activity;").unwrap();
    assert!(freelist_count(&conn) > 0, "free page가 생겨야 테스트가 의미 있다");

    purge_free_pages(&conn, "테스트").unwrap();

    assert_eq!(freelist_count(&conn), 0, "VACUUM 후 free page가 없어야 한다");
}

#[test]
fn test_purge_free_pages_preserves_user_version() {
    // VACUUM이 user_version을 초기화하면, 정리할 때마다 DB가 구버전으로 보여
    // 열 때마다 마이그레이션이 다시 돌게 된다.
    let conn = setup_test_db();
    conn.pragma_update(None, "user_version", crate::db::SCHEMA_VERSION)
        .unwrap();

    purge_free_pages(&conn, "테스트").unwrap();

    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, crate::db::SCHEMA_VERSION);
}

#[test]
fn test_enable_encryption_purges_free_pages() {
    // 암호화는 행을 제자리에서 UPDATE하므로 옛 페이지에 평문이 남는다.
    //
    // 이 테스트가 의미를 가지려면 (1) 암호화 전에 free page가 넉넉히 있어야 하고
    // (2) 암호화가 그것을 다 써버리지 않아야 한다. 암호문은 평문보다 길어서
    // UPDATE가 freelist에서 페이지를 가져다 쓴다. 그래서 긴 기록을 많이 만들었다
    // 지워 free page를 크게 만들고, 남겨서 암호화할 대상은 적게 둔다.
    // 그러지 않으면 purge_free_pages를 호출하지 않아도 통과하는 허수 테스트가 된다.
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    let (db_path, tmp_dir) = setup_temp_db_path_state();

    let act_id = insert_activity(&conn, "발표");
    let long_text = "긴 활동 기록 내용을 여러 번 반복한다. ".repeat(40);
    let mut throwaway = Vec::new();
    for i in 1..=300 {
        let stu_id = insert_student(&conn, 1, 1, i, &format!("학생{i}"));
        upsert_record_impl(&conn, act_id, stu_id, &long_text, None).unwrap();
        if i > 5 {
            throwaway.push(stu_id);
        }
    }
    // 지우면 기록도 CASCADE로 사라지면서 페이지가 freelist로 간다.
    for stu_id in throwaway {
        conn.execute("DELETE FROM Student WHERE id = ?1", rusqlite::params![stu_id])
            .unwrap();
    }
    let before = freelist_count(&conn);
    assert!(before > 100, "암호화 전 free page가 충분해야 한다: {before}");

    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();

    assert_eq!(
        freelist_count(&conn),
        0,
        "암호화가 끝나면 평문이 남은 free page가 없어야 한다 (VACUUM 미호출 의심)"
    );
    std::fs::remove_dir_all(&tmp_dir).ok();
}

// ── 손상된 암호화 설정: salt 쪽 ───────────────────────────────────
//
// 기존 테스트는 salt가 **없는** 경우만 다룬다. 값이 있지만 망가진 경우는
// 에러 메시지가 달라야 사용자가 원인을 알 수 있다.

#[test]
fn test_unlock_with_malformed_base64_salt_returns_decode_error() {
    let conn = setup_test_db();
    let crypto = crypto_state(None);
    set_config_impl(&conn, "encryption_enabled", "true").unwrap();
    // base64가 아닌 문자열이 salt 자리에 들어간 경우
    set_config_impl(&conn, "encryption_pbkdf2_salt", "!!!not-base64!!!").unwrap();

    let err = unlock_encryption_impl(&conn, &crypto, "any_password").unwrap_err();
    assert!(
        err.contains("salt 디코딩 실패"),
        "salt가 깨졌다는 것이 드러나야 한다. 실제: {err}"
    );
}

#[test]
fn test_unlock_with_wrong_length_salt_fails_as_wrong_password() {
    let conn = setup_test_db();
    let db_path = setup_temp_db_path_state();
    let crypto = crypto_state(None);

    // 정상적으로 암호화를 켠 뒤
    enable_encryption_impl(&conn, &crypto, &db_path.0, "correct_password").unwrap();
    clear_crypto_state(&crypto).unwrap();

    // salt만 길이가 다른 값으로 바꿔치기한다 (base64로는 멀쩡히 디코딩된다)
    set_config_impl(&conn, "encryption_pbkdf2_salt", &B64.encode([1u8; 4])).unwrap();

    // 파생 키가 달라지므로 올바른 비밀번호로도 열리지 않는다.
    // derive_key는 salt 길이를 검사하지 않으므로 패닉 없이 검증 실패로 떨어진다.
    let err = unlock_encryption_impl(&conn, &crypto, "correct_password").unwrap_err();
    assert!(
        !err.is_empty(),
        "salt가 바뀌면 올바른 비밀번호여도 실패해야 한다"
    );
    assert!(
        crate::state::current_crypto_key(&crypto).unwrap().is_none(),
        "실패했는데 키가 남아 있으면 안 된다"
    );
    std::fs::remove_dir_all(&db_path.1).ok();
}

// ── encryption_enabled 플래그는 정확히 "true"만 참 ────────────────
//
// 이 판정이 느슨해지면 암호화된 DB를 평문 DB로 오인해 UI에 암호문을
// 그대로 내보내게 된다. 값이 다르면 반드시 "꺼짐"으로 판정되어야 한다.

#[test]
fn test_is_encryption_enabled_matches_only_exact_true() {
    for value in ["True", "TRUE", "1", "yes", "true ", " true", ""] {
        let conn = setup_test_db();
        set_config_impl(&conn, "encryption_enabled", value).unwrap();
        let status = get_encryption_status_impl(&conn, &crypto_state(None)).unwrap();
        assert!(
            !status.enabled,
            "{value:?}는 활성화로 판정되면 안 된다 (정확히 \"true\"만 참)"
        );
    }

    let conn = setup_test_db();
    set_config_impl(&conn, "encryption_enabled", "true").unwrap();
    let status = get_encryption_status_impl(&conn, &crypto_state(None)).unwrap();
    assert!(status.enabled, "\"true\"는 활성화로 판정되어야 한다");
}

#[test]
fn test_non_true_flag_leaves_data_readable_as_plaintext_path() {
    let conn = setup_test_db();
    let db_path = setup_temp_db_path_state();
    let crypto = crypto_state(None);

    create_student_impl(&conn, 1, 1, 1, "홍길동", None).unwrap();
    enable_encryption_impl(&conn, &crypto, &db_path.0, "password").unwrap();

    // 플래그만 망가뜨리면 앱은 "암호화 꺼짐"으로 보고 키를 요구하지 않는다.
    // 이때 조회 결과는 평문이 아니라 암호문이어야 한다 — 즉 복호화된 척하지 않는다.
    set_config_impl(&conn, "encryption_enabled", "TRUE").unwrap();
    assert!(!get_encryption_status_impl(&conn, &crypto).unwrap().enabled);

    let students = get_students_impl(&conn, None).unwrap();
    assert_ne!(
        students[0].name, "홍길동",
        "플래그가 깨져도 암호문이 평문으로 둔갑하면 안 된다"
    );
    std::fs::remove_dir_all(&db_path.1).ok();
}

// ── 암호화 전 백업이 실제로 복구 가능한 파일인가 ──────────────
//
// 암호화 백업은 실패 시 사용자에게 "이 백업으로 되돌리라"고 안내하는 파일이다.
// fs::copy는 SQLite 락을 거치지 않아 반쯤 커밋된 페이지를 담을 수 있었으므로
// VACUUM INTO로 바꿨다.
//
// **이 테스트가 보증하는 것**: 백업이 실제로 열리고, 무결성이 온전하며, 데이터가
// 실려 있다. disable은 백업을 남기는 것이 의도된 설계이므로 -pre-decrypt를 본다.
// **보증하지 못하는 것**: fs::copy의 원래 실패 모드(동시 쓰기로 찢어진 사본)는
// 단일 스레드 정지 상태에서 재현되지 않으므로 이 테스트로는 잡히지 않는다.
// 즉 회귀 방지용이지, VACUUM INTO 전환의 근거는 아니다.

#[test]
fn test_disable_encryption_backup_is_valid_and_readable() {
    let dir = std::env::temp_dir().join(format!(
        "preenc_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("학생부.db");

    let db = DbState(std::sync::Mutex::new(None));
    let db_path = DbPathState(std::sync::Mutex::new(None));
    let crypto = crypto_state(None);
    let cache: crate::state::ReplaceCacheState = std::sync::Mutex::new(ReplaceCache {
        ruleset_version: 0,
        entries: std::collections::HashMap::new(),
    });
    crate::commands::project::new_project_impl(
        path.to_str().unwrap(), "0.2.22", &db, &db_path, &crypto, &cache,
    )
    .unwrap();

    {
        let guard = db.0.lock().unwrap();
        let conn = guard.as_ref().unwrap();
        create_student_impl(conn, 1, 1, 1, "홍길동", None).unwrap();
        // 암호화를 켜면 -pre-encrypt 백업이 만들어지고, 성공 시 삭제된다.
        // 삭제 전 상태를 보기 위해 백업 함수 대신 전체 흐름을 쓰되,
        // 실패 경로가 아니므로 여기서는 disable 쪽 백업(-pre-decrypt)을 검사한다.
        enable_encryption_impl(conn, &crypto, &db_path, "password").unwrap();
        disable_encryption_impl(conn, &crypto, &db_path).unwrap();
    }

    // disable은 백업을 남기는 것이 의도된 설계다(CLAUDE.md).
    let backups: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().contains("-pre-decrypt"))
        .collect();
    assert_eq!(backups.len(), 1, "-pre-decrypt 백업이 남아야 한다");

    // 그 백업이 실제로 열리고 무결성이 온전해야 한다.
    let b = rusqlite::Connection::open(&backups[0]).unwrap();
    let integrity: String = b
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok", "복구용 백업이 손상되면 안 된다");

    // VACUUM INTO는 프리 페이지를 옮기지 않는다.
    let freelist: i64 = b.query_row("PRAGMA freelist_count", [], |r| r.get(0)).unwrap();
    assert_eq!(freelist, 0, "백업에 프리 페이지가 남으면 안 된다");

    // 학생 행이 그대로 실려 있어야 한다(백업 시점은 암호화 상태).
    let cnt: i64 = b
        .query_row("SELECT COUNT(*) FROM Student", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cnt, 1, "백업에 데이터가 실려 있어야 한다");

    std::fs::remove_dir_all(&dir).ok();
}

// ── 백업의 임시 이름(.part) ───────────────────────────────────────
//
// `vacuum_into_backup`은 완성 전 이름으로 쓰고 성공해야 최종 이름으로 옮긴다.
// 이 방어에는 테스트가 없었다 — `.part`를 빼고 바로 최종 이름으로 쓰도록 되돌려도
// 전 테스트가 통과했다. 백업을 보는 테스트가 전부 성공 경로만 탔기 때문이다.

/// 파일 DB 하나를 임시 디렉터리에 만든다. 반환한 디렉터리는 호출부가 지운다.
fn file_db_for_backup() -> (Connection, std::path::PathBuf) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "school_record_backup_part_{}_{}",
        std::process::id(),
        nanos
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.db");
    let conn = crate::db::create_new(&path).unwrap();
    (conn, dir)
}

/// 디렉터리에 남은 `.part` 파일. 성공하든 실패하든 하나도 없어야 한다.
fn part_files(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".part"))
        .collect()
}

#[test]
fn test_backup_leaves_only_the_final_name_on_success() {
    let (conn, dir) = file_db_for_backup();
    insert_activity(&conn, "발표");
    let dest = dir.join("out.db.backup");

    vacuum_into_backup(&conn, &dest).unwrap();

    assert!(dest.exists(), "최종 이름의 백업이 있어야 한다");
    assert!(
        part_files(&dir).is_empty(),
        "성공했으면 임시 파일이 남으면 안 된다: {:?}",
        part_files(&dir)
    );

    // 옮겨진 파일이 실제로 열리는 DB인지까지 본다. 이름만 맞고 내용이 반쪽이면
    // 정작 복구하려는 순간에야 알게 된다.
    let restored = Connection::open(&dest).unwrap();
    let cnt: i64 = restored
        .query_row("SELECT COUNT(*) FROM Activity", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cnt, 1);

    drop(restored);
    drop(conn);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_backup_makes_nothing_when_the_path_cannot_be_written() {
    let (conn, dir) = file_db_for_backup();
    // 없는 디렉터리 안을 가리키면 VACUUM INTO 단계에서 실패한다.
    let dest = dir.join("no_such_dir").join("out.db.backup");

    let err = vacuum_into_backup(&conn, &dest).unwrap_err();

    assert!(err.contains("백업 생성 실패"), "실제 오류: {err}");
    // 이 테스트가 주장하는 것은 "아무것도 만들지 않는다"까지다. 실패 지점의 상위
    // 디렉터리가 없는 채로 남아 있으면 그 안에 임시 파일도 있을 수 없다.
    // (.part를 만든 **뒤** 실패하는 경로의 정리는 아래 rename 실패 테스트가 본다.
    //  여기서 `part_files(&dir)`를 보던 예전 단언은 엉뚱한 디렉터리를 훑고 있어,
    //  정리 코드를 통째로 지워도 통과하는 공허한 단언이었다.)
    assert!(
        !dest.parent().unwrap().exists(),
        "실패 경로에 디렉터리가 생기면 안 된다"
    );
    assert!(
        part_files(&dir).is_empty(),
        "원본 옆에는 아무것도 남으면 안 된다: {:?}",
        part_files(&dir)
    );

    drop(conn);
    std::fs::remove_dir_all(&dir).ok();
}

/// APP_CONFIGS가 없는 파일에서 암호화를 켜려 하면 **백업을 뜨기 전에** 막아야 한다.
///
/// 이 순서가 뒤집히면 성공은 구조적으로 불가능한데(설정을 저장할 테이블이 없다)
/// 평문 전체 사본만 디스크에 남는다. 게다가 매번 새 이름이라 누를 때마다 쌓인다.
#[test]
fn test_enable_encryption_on_old_file_leaves_no_plaintext_backup() {
    let (conn, dir) = file_db_for_backup();
    insert_student(&conn, 1, 1, 1, "홍길동");
    conn.execute_batch("DROP TABLE APP_CONFIGS").unwrap();

    let path_state = DbPathState(std::sync::Mutex::new(Some(dir.join("test.db"))));
    let err = enable_encryption_impl(&conn, &crypto_state(None), &path_state, "password")
        .unwrap_err();

    assert!(err.contains("옛 형식"), "실제 오류: {err}");
    let leftovers: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .filter(|n| n.contains("backup"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "실패할 것이 뻔한 작업이 평문 사본을 남겼다: {leftovers:?}"
    );

    drop(conn);
    std::fs::remove_dir_all(&dir).ok();
}

/// **임시 이름을 거친다는 사실 자체**를 고정한다.
///
/// 최종 이름 자리에 디렉터리를 둔다. 임시 이름으로 먼저 쓰면 VACUUM은 성공하고
/// 그 다음 rename에서 넘어지므로 오류가 "이름을 바꾸지 못했습니다"가 된다.
/// `.part`를 지우고 바로 최종 이름으로 쓰도록 되돌리면 VACUUM 자체가 실패해
/// "백업 생성 실패"가 나온다 — 그래서 이 단언이 그 변이를 잡는다.
#[test]
fn test_backup_is_written_under_a_temporary_name_first() {
    let (conn, dir) = file_db_for_backup();
    let dest = dir.join("out.db.backup");
    std::fs::create_dir(&dest).unwrap();

    let err = vacuum_into_backup(&conn, &dest).unwrap_err();

    assert!(
        err.contains("이름을 바꾸지 못했습니다"),
        "임시 이름으로 먼저 쓰지 않았다. 실제 오류: {err}"
    );
    assert!(
        part_files(&dir).is_empty(),
        "옮기지 못했으면 임시 파일을 지워야 한다: {:?}",
        part_files(&dir)
    );

    drop(conn);
    std::fs::remove_dir_all(&dir).ok();
}
