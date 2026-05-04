use crate::commands::record::upsert_record_impl;
use crate::commands::snapshot::{create_snapshot_impl, get_snapshots_impl, restore_snapshot_impl};
use super::{insert_activity, insert_student, setup_test_db};

// ── create_snapshot ────────────────────────────────────────────

#[test]
fn test_create_snapshot_creates_history_for_records() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "훌륭한 발표", None).unwrap();

    create_snapshot_impl(&conn, None).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_create_snapshot_no_duplicate_same_updated_at() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "내용", None).unwrap();

    create_snapshot_impl(&conn, None).unwrap();
    create_snapshot_impl(&conn, None).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "동일 updated_at 기록은 히스토리 중복 생성 안 됨");
}

#[test]
fn test_create_snapshot_empty_db_ok() {
    let conn = setup_test_db();
    let result = create_snapshot_impl(&conn, Some("메모".to_string()));
    assert!(result.is_ok());
}

#[test]
fn test_create_snapshot_returns_item_with_id() {
    let conn = setup_test_db();
    let item = create_snapshot_impl(&conn, Some("테스트".to_string())).unwrap();
    assert!(item.id > 0);
    assert!(!item.created_at.is_empty());
    assert_eq!(item.memo, Some("테스트".to_string()));
}

#[test]
fn test_create_snapshot_none_memo_returns_none() {
    let conn = setup_test_db();

    let item = create_snapshot_impl(&conn, None).unwrap();

    assert!(item.id > 0);
    assert!(item.memo.is_none(), "None으로 생성한 스냅샷의 memo는 None이어야 함");
}

// ── get_snapshots ──────────────────────────────────────────────

#[test]
fn test_get_snapshots_empty_db() {
    let conn = setup_test_db();
    let items = get_snapshots_impl(&conn).unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_get_snapshots_ordered_desc() {
    let conn = setup_test_db();
    create_snapshot_impl(&conn, Some("첫번째".to_string())).unwrap();
    // 동일 시각 방지를 위해 updated_at 강제 차이 — Snapshot.created_at은 DEFAULT datetime('now')
    // 두 INSERT 사이에 실제 시간 차이가 없을 수 있으므로 직접 삽입으로 보장
    conn.execute(
        "INSERT INTO Snapshot (memo, created_at) VALUES ('두번째', datetime('now', '+1 second'))",
        [],
    )
    .unwrap();

    let items = get_snapshots_impl(&conn).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].memo, Some("두번째".to_string()), "최신 스냅샷이 첫 번째여야 함");
}

#[test]
fn test_get_snapshots_none_memo() {
    let conn = setup_test_db();

    create_snapshot_impl(&conn, None).unwrap();
    create_snapshot_impl(&conn, Some("메모있음".to_string())).unwrap();

    let items = get_snapshots_impl(&conn).unwrap();

    assert_eq!(items.len(), 2);
    let none_count = items.iter().filter(|i| i.memo.is_none()).count();
    assert_eq!(none_count, 1, "memo=None 스냅샷이 목록에 포함되어야 함");
    let some_item = items.iter().find(|i| i.memo.is_some()).unwrap();
    assert_eq!(some_item.memo.as_deref(), Some("메모있음"));
}

// ── restore_snapshot ───────────────────────────────────────────

#[test]
fn test_restore_snapshot_reverts_content() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "초기 내용", None).unwrap();

    let snap = create_snapshot_impl(&conn, None).unwrap();

    upsert_record_impl(&conn, act_id, stu_id, "수정된 내용", None).unwrap();

    restore_snapshot_impl(&conn, snap.id).unwrap();

    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(content, "초기 내용");
}

#[test]
fn test_restore_snapshot_sets_empty_when_no_history() {
    let conn = setup_test_db();
    // 빈 DB에서 스냅샷 생성 (히스토리 없음)
    let snap = create_snapshot_impl(&conn, None).unwrap();

    // 스냅샷 이후에 기록 추가
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    conn.execute(
        "INSERT INTO ActivityRecord (activity_id, student_id, content) VALUES (?1, ?2, '새 내용')",
        rusqlite::params![act_id, stu_id],
    )
    .unwrap();

    restore_snapshot_impl(&conn, snap.id).unwrap();

    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(content, "", "스냅샷 이전에 히스토리 없는 기록은 빈 문자열로 복원");
}

#[test]
fn test_restore_nonexistent_snapshot_error() {
    let conn = setup_test_db();
    let result = restore_snapshot_impl(&conn, 9999);
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("스냅샷을 찾을 수 없습니다"), "에러 메시지: {msg}");
}

#[test]
fn test_restore_returns_affected_row_count() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu1 = insert_student(&conn, 1, 1, 1, "홍길동");
    let stu2 = insert_student(&conn, 1, 1, 2, "김철수");
    upsert_record_impl(&conn, act_id, stu1, "내용1", None).unwrap();
    upsert_record_impl(&conn, act_id, stu2, "내용2", None).unwrap();

    let snap = create_snapshot_impl(&conn, None).unwrap();
    let count = restore_snapshot_impl(&conn, snap.id).unwrap();

    assert_eq!(count, 2, "기록 2개가 업데이트되어야 함");
}

#[test]
fn test_restore_snapshot_correct_version_with_multiple_histories() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    conn.execute(
        "INSERT INTO ActivityRecord (activity_id, student_id, content, updated_at)
         VALUES (?1, ?2, '현재 내용', '2024-01-01 12:00:00')",
        rusqlite::params![act_id, stu_id],
    )
    .unwrap();
    let record_id: i64 = conn
        .query_row(
            "SELECT id FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();

    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         VALUES (?1, '버전1', '2024-01-01 09:00:00', NULL)",
        rusqlite::params![record_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         VALUES (?1, '버전2', '2024-01-01 10:00:00', NULL)",
        rusqlite::params![record_id],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO Snapshot (memo, created_at) VALUES (NULL, '2024-01-01 10:30:00')",
        [],
    )
    .unwrap();
    let snap_id = conn.last_insert_rowid();

    // 스냅샷 이후 히스토리 — 복원 시 무시되어야 함
    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         VALUES (?1, '버전3', '2024-01-01 11:00:00', NULL)",
        rusqlite::params![record_id],
    )
    .unwrap();

    restore_snapshot_impl(&conn, snap_id).unwrap();

    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(content, "버전2", "스냅샷 시점 기준 직전 최신 히스토리로 복원되어야 함");
}
