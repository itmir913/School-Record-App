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

    create_snapshot_impl(&conn, None, None).unwrap();

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

    create_snapshot_impl(&conn, None, None).unwrap();
    create_snapshot_impl(&conn, None, None).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "동일 updated_at 기록은 히스토리 중복 생성 안 됨");
}

#[test]
fn test_create_snapshot_empty_db_ok() {
    let conn = setup_test_db();
    let result = create_snapshot_impl(&conn, Some("메모".to_string()), None);
    assert!(result.is_ok());
}

#[test]
fn test_create_snapshot_returns_item_with_id() {
    let conn = setup_test_db();
    let item = create_snapshot_impl(&conn, Some("테스트".to_string()), None).unwrap();
    assert!(item.id > 0);
    assert!(!item.created_at.is_empty());
    assert_eq!(item.memo, Some("테스트".to_string()));
}

#[test]
fn test_create_snapshot_none_memo_returns_none() {
    let conn = setup_test_db();

    let item = create_snapshot_impl(&conn, None, None).unwrap();

    assert!(item.id > 0);
    assert!(item.memo.is_none(), "None으로 생성한 스냅샷의 memo는 None이어야 함");
}

// ── get_snapshots ──────────────────────────────────────────────

#[test]
fn test_get_snapshots_empty_db() {
    let conn = setup_test_db();
    let items = get_snapshots_impl(&conn, None).unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_get_snapshots_ordered_desc() {
    let conn = setup_test_db();
    create_snapshot_impl(&conn, Some("첫번째".to_string()), None).unwrap();
    // 동일 시각 방지를 위해 updated_at 강제 차이 — Snapshot.created_at은 DEFAULT datetime('now')
    // 두 INSERT 사이에 실제 시간 차이가 없을 수 있으므로 직접 삽입으로 보장
    conn.execute(
        "INSERT INTO Snapshot (memo, created_at) VALUES ('두번째', datetime('now', '+1 second'))",
        [],
    )
    .unwrap();

    let items = get_snapshots_impl(&conn, None).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].memo, Some("두번째".to_string()), "최신 스냅샷이 첫 번째여야 함");
}

#[test]
fn test_get_snapshots_none_memo() {
    let conn = setup_test_db();

    create_snapshot_impl(&conn, None, None).unwrap();
    create_snapshot_impl(&conn, Some("메모있음".to_string()), None).unwrap();

    let items = get_snapshots_impl(&conn, None).unwrap();

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

    let snap = create_snapshot_impl(&conn, None, None).unwrap();

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
fn test_restore_snapshot_refused_before_migration() {
    // 다른 쓰기 경로와 같은 가드다. 마이그레이션이 끝나지 않은 파일에는 쓰지 않는다.
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act_id, stu_id, "초기 내용", None).unwrap();
    let snap = create_snapshot_impl(&conn, None, None).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "수정된 내용", None).unwrap();
    conn.pragma_update(None, "user_version", crate::db::SCHEMA_VERSION - 1)
        .unwrap();

    let err = restore_snapshot_impl(&conn, snap.id).unwrap_err();

    assert!(err.contains("파일 형식 업데이트"), "오류: {err}");
    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(content, "수정된 내용");
}

#[test]
fn test_restore_snapshot_sets_empty_when_no_history() {
    let conn = setup_test_db();
    // 빈 DB에서 스냅샷 생성 (히스토리 없음)
    let snap = create_snapshot_impl(&conn, None, None).unwrap();

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

    let snap = create_snapshot_impl(&conn, None, None).unwrap();
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

// ── 같은 초에 내용이 바뀐 경우 (감사 F1) ──────────────────────
//
// 히스토리 중복 판정이 changed_at == updated_at만 보면, 같은 초에 이미 다른 내용의
// 히스토리 행이 있을 때 스냅샷이 0행을 넣고도 성공한 것처럼 보인다. 그 상태로
// 복원하면 현재 내용이 아니라 옛 내용이 돌아온다.
//
// updated_at은 초 단위라 엑셀에 (학생, 활동) 중복 행이 있으면 import 한 번으로
// 결정론적으로 만들어진다. 아래 테스트는 그 상황을 SQL로 직접 재현한다.

/// 같은 activity_record에 대해 지정한 시각의 히스토리 행을 만든다.
fn insert_history_at(conn: &rusqlite::Connection, act: i64, stu: i64, content: &str, at: &str) {
    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         SELECT r.id, ?3, ?4, 'stale'
         FROM ActivityRecord r WHERE r.activity_id = ?1 AND r.student_id = ?2",
        rusqlite::params![act, stu, content, at],
    )
    .unwrap();
}

fn set_updated_at(conn: &rusqlite::Connection, act: i64, stu: i64, at: &str) {
    conn.execute(
        "UPDATE ActivityRecord SET updated_at = ?3 WHERE activity_id = ?1 AND student_id = ?2",
        rusqlite::params![act, stu, at],
    )
    .unwrap();
}

#[test]
fn test_snapshot_captures_current_content_despite_same_second_history() {
    let conn = setup_test_db();
    let act = insert_activity(&conn, "발표");
    let stu = insert_student(&conn, 1, 1, 1, "홍길동");

    // 같은 초에 옛 내용의 히스토리가 이미 있고, 현재 내용은 그와 다르다.
    upsert_record_impl(&conn, act, stu, "최종 내용", None).unwrap();
    set_updated_at(&conn, act, stu, "2026-01-01 09:00:00");
    insert_history_at(&conn, act, stu, "옛 내용", "2026-01-01 09:00:00");

    create_snapshot_impl(&conn, None, None).unwrap();

    // 현재 내용이 히스토리에 반드시 담겨야 한다.
    let captured: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ActivityRecordHistory WHERE content = '최종 내용'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        captured, 1,
        "같은 초에 다른 내용의 히스토리가 있다고 현재 내용을 건너뛰면 안 된다"
    );
}

#[test]
fn test_snapshot_does_not_relabel_stale_row_as_if_saved() {
    let conn = setup_test_db();
    let act = insert_activity(&conn, "발표");
    let stu = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act, stu, "최종 내용", None).unwrap();
    set_updated_at(&conn, act, stu, "2026-01-01 09:00:00");
    insert_history_at(&conn, act, stu, "옛 내용", "2026-01-01 09:00:00");

    crate::commands::record::save_snapshot_internal(&conn, act, stu, Some("치환 적용 전"), None).unwrap();

    // 옛 내용 행의 note를 덧씌우면, 저장하지 않은 것을 저장한 것처럼 보이게 된다.
    let mislabeled: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ActivityRecordHistory
             WHERE content = '옛 내용' AND note = '치환 적용 전'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(mislabeled, 0, "저장하지 않은 행에 note를 덧씌우면 안 된다");
}

#[test]
fn test_snapshot_still_dedupes_identical_content_at_same_time() {
    let conn = setup_test_db();
    let act = insert_activity(&conn, "발표");
    let stu = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act, stu, "그대로", None).unwrap();

    // 변경 없이 두 번 스냅샷하면 히스토리는 하나여야 한다(기존 동작 유지).
    create_snapshot_impl(&conn, None, None).unwrap();
    create_snapshot_impl(&conn, None, None).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "내용이 같으면 중복 저장하지 않는 기존 동작은 유지되어야 한다");
}

// ── 같은 초 동률에서 복원이 엉뚱한 버전을 고르는 문제 ─────────
//
// F1으로 "현재 내용이 히스토리에 없는" 구멍은 막았지만, 중복 판정이
// "그 (내용, 시각) 행이 **어딘가** 있는가"라 A→B→A처럼 값이 되돌아오면
// 중간값 행이 같은 초에 남는다. restore는 ORDER BY changed_at DESC뿐이라
// 동률 순서가 미정의고, 중간값 B를 골라 덮어쓸 수 있다.
//
// 엑셀에 (학생, 활동) 중복 행 [A, B, A]가 있으면 import 한 번으로 재현된다.

#[test]
fn test_restore_returns_current_content_after_same_second_aba() {
    use crate::commands::record::bulk_import_records_impl;
    use crate::types::ImportRecordInput;

    let conn = setup_test_db();
    let act = insert_activity(&conn, "발표");

    let row = |content: &str| ImportRecordInput {
        grade: 1,
        class_num: 1,
        number: 1,
        name: Some("홍길동".to_string()),
        activity_id: act,
        content: content.to_string(),
    };

    // 한 번의 가져오기 안에서 같은 학생·활동이 A → B → A로 세 번 등장한다.
    bulk_import_records_impl(&conn, &[row("A 내용"), row("B 내용"), row("A 내용")], None).unwrap();

    let stu: i64 = conn
        .query_row("SELECT id FROM Student LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let current: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act, stu],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(current, "A 내용", "가져오기 직후 현재 내용은 마지막 행이어야 한다");

    // 스냅샷을 찍고, 이후 내용을 바꾼 뒤 되돌린다.
    let snap = create_snapshot_impl(&conn, Some("복원 기준".to_string()), None).unwrap();
    upsert_record_impl(&conn, act, stu, "그 뒤에 쓴 내용", None).unwrap();
    restore_snapshot_impl(&conn, snap.id).unwrap();

    let restored: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act, stu],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        restored, "A 내용",
        "스냅샷 시점의 내용으로 돌아와야 한다. 같은 초의 중간값(B)을 고르면 안 된다"
    );
}

// ── 기존(레거시) 데이터 안전성 ────────────────────────────────
//
// 배포된 사용자 파일에는 초 단위 changed_at으로 기록된 히스토리가 이미 쌓여 있다.
// 버전 순서를 changed_at에서 id로 옮겨도 그 데이터의 복원 결과가 달라지면 안 된다.
// (실제 사용자 파일에서 id 순서와 changed_at 순서가 한 건도 어긋나지 않음을 확인했다.
//  히스토리 행은 항상 그 시점의 updated_at을 복사해 넣고 updated_at은 앞으로만 가므로
//  레코드별로 id 오름차순 = 시간 오름차순이 구조적으로 성립한다.)

#[test]
fn test_restore_on_legacy_second_granularity_history() {
    let conn = setup_test_db();
    let act = insert_activity(&conn, "발표");
    let stu = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act, stu, "현재", None).unwrap();

    // 예전 앱이 남긴 모양: 초 단위 시각, 서로 다른 시각, id 오름차순 = 시간 오름차순
    for (content, at) in [
        ("3월 내용", "2026-03-01 09:00:00"),
        ("5월 내용", "2026-05-01 09:00:00"),
        ("7월 내용", "2026-07-01 09:00:00"),
    ] {
        insert_history_at(&conn, act, stu, content, at);
    }

    // 6월 시점 스냅샷으로 복원하면 그 이전의 가장 최근 버전(5월)이어야 한다.
    conn.execute(
        "INSERT INTO Snapshot (memo, created_at) VALUES ('6월', '2026-06-01 09:00:00')",
        [],
    )
    .unwrap();
    let snap_id: i64 = conn
        .query_row("SELECT id FROM Snapshot LIMIT 1", [], |r| r.get(0))
        .unwrap();
    restore_snapshot_impl(&conn, snap_id).unwrap();

    let restored: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act, stu],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        restored, "5월 내용",
        "레거시 데이터의 시점 필터(changed_at)는 그대로 동작해야 한다"
    );
}

#[test]
fn test_legacy_history_is_never_rewritten_by_new_logic() {
    let conn = setup_test_db();
    let act = insert_activity(&conn, "발표");
    let stu = insert_student(&conn, 1, 1, 1, "홍길동");
    upsert_record_impl(&conn, act, stu, "현재 내용", None).unwrap();
    insert_history_at(&conn, act, stu, "예전 내용", "2026-03-01 09:00:00");

    let before: Vec<(i64, String, String)> = conn
        .prepare("SELECT id, content, changed_at FROM ActivityRecordHistory ORDER BY id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    // 스냅샷을 여러 번 찍어도 기존 행의 내용·시각은 절대 바뀌지 않아야 한다.
    create_snapshot_impl(&conn, None, None).unwrap();
    create_snapshot_impl(&conn, None, None).unwrap();

    let after: Vec<(i64, String, String)> = conn
        .prepare("SELECT id, content, changed_at FROM ActivityRecordHistory WHERE id <= ?1 ORDER BY id")
        .unwrap()
        .query_map(rusqlite::params![before.last().unwrap().0], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(before, after, "기존 히스토리 행이 덮어써지면 안 된다");
}

// 암호문은 매번 다른 nonce로 만들어진다. 같은 평문을 다시 저장한 뒤 스냅샷을
// 찍어도 평문 파일처럼 히스토리 행이 늘지 않아야 한다.
#[test]
fn test_encrypted_create_snapshot_same_plaintext_adds_no_history() {
    let conn = setup_test_db();
    let key = Some(crate::crypto::derive_key("password", &[42u8; 16]));
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    let count = |conn: &rusqlite::Connection| -> i64 {
        conn.query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |r| r.get(0))
            .unwrap()
    };

    upsert_record_impl(&conn, act_id, stu_id, "같은 내용", key).unwrap();
    create_snapshot_impl(&conn, None, key).unwrap();
    assert_eq!(count(&conn), 1);

    upsert_record_impl(&conn, act_id, stu_id, "같은 내용", key).unwrap();
    create_snapshot_impl(&conn, None, key).unwrap();
    assert_eq!(count(&conn), 1, "같은 내용을 다시 저장해도 히스토리 행이 늘면 안 된다");

    upsert_record_impl(&conn, act_id, stu_id, "바뀐 내용", key).unwrap();
    create_snapshot_impl(&conn, None, key).unwrap();
    assert_eq!(count(&conn), 2, "내용이 바뀌면 새 행이 생겨야 한다");
}

// 암호화를 켜면 ActivityRecord.content와 ActivityRecordHistory.content가 각각
// 다른 nonce로 암호화된다. 전환 직후 스냅샷이 모든 기록에 중복 행을 만들면 안 된다.
#[test]
fn test_create_snapshot_after_enable_encryption_adds_no_history() {
    use crate::commands::crypto::{enable_encryption_impl, resolve_data_key};
    use crate::state::CryptoState;

    let conn = setup_test_db();
    let crypto = std::sync::Mutex::new(CryptoState { key: None });
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    let count = |conn: &rusqlite::Connection| -> i64 {
        conn.query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |r| r.get(0))
            .unwrap()
    };

    upsert_record_impl(&conn, act_id, stu_id, "평문 내용", None).unwrap();
    create_snapshot_impl(&conn, None, None).unwrap();
    assert_eq!(count(&conn), 1);

    let (db_path, tmp_dir) = super::setup_temp_db_path_state();
    enable_encryption_impl(&conn, &crypto, &db_path, "password").unwrap();
    let key = resolve_data_key(&conn, &crypto).unwrap();
    assert!(key.is_some());

    create_snapshot_impl(&conn, None, key).unwrap();
    std::fs::remove_dir_all(&tmp_dir).ok();
    assert_eq!(count(&conn), 1, "암호화 전환만으로 히스토리 행이 늘면 안 된다");
}
