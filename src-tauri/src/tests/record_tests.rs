use crate::commands::record::{
    bulk_import_records_impl, bulk_quick_replace_impl, get_area_grid_impl, get_record_history_impl,
    preview_import_records_impl, save_snapshot_internal, upsert_record_impl,
};
use crate::types::ImportRecordInput;
use super::{insert_activity, insert_area, insert_record, insert_student, setup_test_db};

// ── upsert_record ──────────────────────────────────────────────

#[test]
fn test_upsert_record_creates_new() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "훌륭한 발표", None).unwrap();

    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(content, "훌륭한 발표");
}

#[test]
fn test_upsert_record_updates_existing() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "초기 내용", None).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "수정된 내용", None).unwrap();

    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(content, "수정된 내용");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecord", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

// ── get_area_grid ──────────────────────────────────────────────

#[test]
fn test_get_area_grid_activities_and_students() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "독서");
    let stu_id = insert_student(&conn, 1, 1, 1, "김철수");

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
    upsert_record_impl(&conn, act_id, stu_id, "독후감", None).unwrap();

    let grid = get_area_grid_impl(&conn, area_id, None).unwrap();
    assert_eq!(grid.activities.len(), 1);
    assert_eq!(grid.activities[0].name, "독서");
    assert_eq!(grid.students.len(), 1);
    assert_eq!(grid.students[0].name, "김철수");
    assert_eq!(grid.records.len(), 1);
    assert_eq!(grid.records[0].content, "독후감");
}

#[test]
fn test_get_area_grid_empty_returns_empty_records() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "수학", 300);

    let grid = get_area_grid_impl(&conn, area_id, None).unwrap();
    assert!(grid.activities.is_empty());
    assert!(grid.students.is_empty());
    assert!(grid.records.is_empty());
}

#[test]
fn test_get_area_grid_activities_only_empty_records() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "국어", 500);
    let act_id = insert_activity(&conn, "독서");
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();

    let grid = get_area_grid_impl(&conn, area_id, None).unwrap();

    assert_eq!(grid.activities.len(), 1);
    assert_eq!(grid.activities[0].name, "독서");
    assert!(grid.students.is_empty());
    assert!(grid.records.is_empty(), "학생 없음 → records 조회 분기 스킵 → 빈 Vec");
}

#[test]
fn test_get_area_grid_activities_sorted_by_name() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "수학", 300);
    let act_z = insert_activity(&conn, "Z발표");
    let act_a = insert_activity(&conn, "A발표");
    for act_id in [act_z, act_a] {
        conn.execute(
            "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
            rusqlite::params![area_id, act_id],
        )
        .unwrap();
    }

    let grid = get_area_grid_impl(&conn, area_id, None).unwrap();

    assert_eq!(grid.activities.len(), 2);
    assert_eq!(grid.activities[0].name, "A발표");
    assert_eq!(grid.activities[1].name, "Z발표");
}

#[test]
fn test_get_area_grid_students_sorted() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "영어", 400);
    let stu_c = insert_student(&conn, 2, 2, 3, "다학생");
    let stu_a = insert_student(&conn, 1, 1, 2, "가학생");
    let stu_b = insert_student(&conn, 1, 2, 1, "나학생");
    for stu_id in [stu_c, stu_a, stu_b] {
        conn.execute(
            "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
            rusqlite::params![area_id, stu_id],
        )
        .unwrap();
    }

    let grid = get_area_grid_impl(&conn, area_id, None).unwrap();

    assert_eq!(grid.students.len(), 3);
    assert_eq!(grid.students[0].name, "가학생"); // (1,1,2)
    assert_eq!(grid.students[1].name, "나학생"); // (1,2,1)
    assert_eq!(grid.students[2].name, "다학생"); // (2,2,3)
}

// ── get_record_history ─────────────────────────────────────────

#[test]
fn test_get_record_history_empty() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    let entries = get_record_history_impl(&conn, act_id, stu_id, 10, 0, None).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn test_get_record_history_ordered_desc() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "첫 번째", None).unwrap();
    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         SELECT id, content, '2024-01-01 10:00:00', NULL FROM ActivityRecord
         WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         SELECT id, content, '2024-01-02 10:00:00', NULL FROM ActivityRecord
         WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    )
    .unwrap();

    let entries = get_record_history_impl(&conn, act_id, stu_id, 10, 0, None).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].changed_at > entries[1].changed_at, "최신 항목이 먼저여야 한다");
}

#[test]
fn test_get_record_history_limit_offset() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "내용", None).unwrap();
    for ts in &["2024-01-01 10:00:00", "2024-01-02 10:00:00", "2024-01-03 10:00:00"] {
        conn.execute(
            "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
             SELECT id, content, ?3, NULL FROM ActivityRecord
             WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id, ts],
        )
        .unwrap();
    }

    let page1 = get_record_history_impl(&conn, act_id, stu_id, 1, 0, None).unwrap();
    let page2 = get_record_history_impl(&conn, act_id, stu_id, 1, 1, None).unwrap();
    assert_eq!(page1.len(), 1);
    assert_eq!(page2.len(), 1);
    assert_ne!(page1[0].id, page2[0].id);
}

#[test]
fn test_get_record_history_note_optional() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "내용", None).unwrap();
    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         SELECT id, content, '2024-01-01 10:00:00', 'snapshot' FROM ActivityRecord
         WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
         SELECT id, content, '2024-01-02 10:00:00', NULL FROM ActivityRecord
         WHERE activity_id=?1 AND student_id=?2",
        rusqlite::params![act_id, stu_id],
    )
    .unwrap();

    let entries = get_record_history_impl(&conn, act_id, stu_id, 10, 0, None).unwrap();
    assert_eq!(entries.len(), 2);
    let notes: Vec<Option<String>> = entries.iter().map(|e| e.note.clone()).collect();
    assert!(notes.iter().any(|n| n.is_some()), "note=Some 항목이 있어야 한다");
    assert!(notes.iter().any(|n| n.is_none()), "note=None 항목이 있어야 한다");
}

// ── save_snapshot_internal ─────────────────────────────────────

#[test]
fn test_save_snapshot_creates_history_entry() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "발표 내용", None).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, Some("스냅샷"), None).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ActivityRecordHistory h
             JOIN ActivityRecord r ON r.id = h.activity_record_id
             WHERE r.activity_id=?1 AND r.student_id=?2",
            rusqlite::params![act_id, stu_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_save_snapshot_no_duplicate_same_updated_at() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "발표 내용", None).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, None, None).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, None, None).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ActivityRecordHistory h
             JOIN ActivityRecord r ON r.id = h.activity_record_id
             WHERE r.activity_id=?1 AND r.student_id=?2",
            rusqlite::params![act_id, stu_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "동일 updated_at에 중복 히스토리가 생성되면 안 된다");
}

#[test]
fn test_save_snapshot_updates_note_when_exists() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "발표 내용", None).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, Some("초기 노트"), None).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, Some("수정 노트"), None).unwrap();

    let note: Option<String> = conn
        .query_row(
            "SELECT h.note FROM ActivityRecordHistory h
             JOIN ActivityRecord r ON r.id = h.activity_record_id
             WHERE r.activity_id=?1 AND r.student_id=?2",
            rusqlite::params![act_id, stu_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(note.as_deref(), Some("수정 노트"));
}

#[test]
fn test_save_snapshot_no_record_is_noop() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    // ActivityRecord 미삽입 — 의도적

    let result = save_snapshot_internal(&conn, act_id, stu_id, Some("노트"), None);

    assert!(result.is_ok());
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0, "ActivityRecord 없으면 히스토리 0행");
}

// ── bulk_import_records ────────────────────────────────────────

fn make_import(grade: i64, class_num: i64, number: i64, name: Option<&str>, activity_id: i64, content: &str) -> ImportRecordInput {
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
fn test_bulk_import_creates_new_student() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");

    bulk_import_records_impl(&conn, &[make_import(1, 1, 1, Some("홍길동"), act_id, "내용")], None).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM Student", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_bulk_import_updates_name_when_existing_is_blank() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    insert_student(&conn, 1, 1, 1, "");

    bulk_import_records_impl(&conn, &[make_import(1, 1, 1, Some("홍길동"), act_id, "내용")], None).unwrap();

    let name: String = conn
        .query_row(
            "SELECT name FROM Student WHERE grade=1 AND class_num=1 AND number=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "홍길동");
}

#[test]
fn test_bulk_import_skips_name_update_when_existing_nonempty() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    insert_student(&conn, 1, 1, 1, "기존이름");

    bulk_import_records_impl(&conn, &[make_import(1, 1, 1, Some("새이름"), act_id, "내용")], None).unwrap();

    let name: String = conn
        .query_row(
            "SELECT name FROM Student WHERE grade=1 AND class_num=1 AND number=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "기존이름");
}

#[test]
fn test_bulk_import_upserts_record() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");

    bulk_import_records_impl(&conn, &[make_import(1, 1, 1, Some("홍길동"), act_id, "처음")], None).unwrap();
    bulk_import_records_impl(&conn, &[make_import(1, 1, 1, Some("홍길동"), act_id, "갱신")], None).unwrap();

    let content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1",
            rusqlite::params![act_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(content, "갱신");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecord", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_bulk_import_creates_history_for_nonempty_content() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");

    bulk_import_records_impl(&conn, &[make_import(1, 1, 1, Some("홍길동"), act_id, "내용")], None).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_bulk_import_no_history_for_empty_content() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");

    bulk_import_records_impl(&conn, &[make_import(1, 1, 1, Some("홍길동"), act_id, "")], None).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_bulk_import_student_cache_deduplication() {
    let conn = setup_test_db();
    let act_id1 = insert_activity(&conn, "발표1");
    let act_id2 = insert_activity(&conn, "발표2");

    bulk_import_records_impl(
        &conn,
        &[
            make_import(1, 1, 1, Some("홍길동"), act_id1, "내용1"),
            make_import(1, 1, 1, Some("홍길동"), act_id2, "내용2"),
        ],
        None,
    )
    .unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM Student", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "동일 학생은 한 번만 삽입되어야 한다");
}

#[test]
fn test_bulk_import_result_counts() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    insert_student(&conn, 2, 1, 1, ""); // 이름 없는 기존 학생

    let result = bulk_import_records_impl(
        &conn,
        &[
            make_import(1, 1, 1, Some("신규1"), act_id, "내용A"),
            make_import(1, 1, 2, Some("신규2"), act_id, "내용B"),
            make_import(2, 1, 1, Some("이름채움"), act_id, "내용C"),
        ],
        None,
    )
    .unwrap();

    assert_eq!(result.students_created, 2);
    assert_eq!(result.students_updated, 1);
    assert_eq!(result.records_saved, 3);
}

#[test]
fn test_bulk_import_uses_default_name_when_none() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");

    bulk_import_records_impl(&conn, &[make_import(1, 1, 1, None, act_id, "내용")], None).unwrap();

    let name: String = conn
        .query_row(
            "SELECT name FROM Student WHERE grade=1 AND class_num=1 AND number=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "이름 없음");
}

// ── preview_import_records ─────────────────────────────────────

#[test]
fn test_preview_existing_student_shows_existing_content() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    insert_record(&conn, act_id, stu_id, "기존 내용");

    let items = preview_import_records_impl(
        &conn,
        &[make_import(1, 1, 1, Some("홍길동"), act_id, "새 내용")],
        None,
    )
    .unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].existing_content, "기존 내용");
    assert_eq!(items[0].new_content, "새 내용");
}

#[test]
fn test_preview_new_student_shows_empty_existing_content() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");

    let items = preview_import_records_impl(
        &conn,
        &[make_import(1, 1, 99, Some("신규학생"), act_id, "새 내용")],
        None,
    )
    .unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].existing_content, "");
    assert_eq!(items[0].student_name, "신규학생");
}

#[test]
fn test_preview_activity_name_fallback() {
    let conn = setup_test_db();
    let fake_act_id: i64 = 9999;

    let items = preview_import_records_impl(
        &conn,
        &[make_import(1, 1, 1, Some("홍길동"), fake_act_id, "내용")],
        None,
    )
    .unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].activity_name, format!("활동 #{}", fake_act_id));
}

// ── import 엣지 케이스 ─────────────────────────────────────────

#[test]
fn test_bulk_import_empty_input_returns_zeros() {
    let conn = setup_test_db();

    let result = bulk_import_records_impl(&conn, &[], None).unwrap();

    assert_eq!(result.students_created, 0);
    assert_eq!(result.students_updated, 0);
    assert_eq!(result.records_saved, 0);
}

#[test]
fn test_preview_import_empty_input_returns_empty() {
    let conn = setup_test_db();

    let items = preview_import_records_impl(&conn, &[], None).unwrap();

    assert!(items.is_empty());
}

// ── 트랜잭션 롤백 검증 ─────────────────────────────────────────

#[test]
fn test_bulk_import_rollback_on_fk_violation() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");

    // 첫 번째: 유효한 레코드 / 두 번째: 존재하지 않는 activity_id → FK 위반
    let records = vec![
        make_import(1, 1, 1, Some("홍길동"), act_id, "내용A"),
        make_import(1, 1, 2, Some("김철수"), 9999, "내용B"),
    ];

    conn.execute_batch("BEGIN").unwrap();
    let result = bulk_import_records_impl(&conn, &records, None);
    match result {
        Ok(_) => { conn.execute_batch("COMMIT").unwrap(); }
        Err(_) => { let _ = conn.execute_batch("ROLLBACK"); }
    }

    let student_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM Student", [], |row| row.get(0))
        .unwrap();
    assert_eq!(student_count, 0, "FK 위반으로 전체 트랜잭션 롤백 시 Student도 0이어야 함");

    let record_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecord", [], |row| row.get(0))
        .unwrap();
    assert_eq!(record_count, 0, "FK 위반으로 전체 트랜잭션 롤백 시 ActivityRecord도 0이어야 함");
}

#[test]
fn test_bulk_import_same_student_multiple_activities_records_saved() {
    // test_bulk_import_student_cache_deduplication 은 Student COUNT만 검증하므로
    // 이 테스트는 ActivityRecord 가 2개 모두 생성되는지를 추가 검증한다.
    let conn = setup_test_db();
    let act_id1 = insert_activity(&conn, "독서");
    let act_id2 = insert_activity(&conn, "발표");

    let result = bulk_import_records_impl(
        &conn,
        &[
            make_import(1, 1, 1, Some("홍길동"), act_id1, "독서 내용"),
            make_import(1, 1, 1, Some("홍길동"), act_id2, "발표 내용"),
        ],
        None,
    )
    .unwrap();

    assert_eq!(result.students_created, 1, "동일 학생은 한 번만 생성되어야 한다");
    assert_eq!(result.records_saved, 2, "활동이 2개이므로 기록 2건이어야 한다");

    let record_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ActivityRecord", [], |row| row.get(0))
        .unwrap();
    assert_eq!(record_count, 2);
}

#[test]
fn test_save_snapshot_without_note_stores_null() {
    // 메모는 선택 사항이다. UI에서 빈 값을 보내면 note가 NULL로 남는다.
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "발표 내용", None).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, None, None).unwrap();

    let note: Option<String> = conn
        .query_row(
            "SELECT h.note FROM ActivityRecordHistory h
             JOIN ActivityRecord r ON r.id = h.activity_record_id
             WHERE r.activity_id=?1 AND r.student_id=?2",
            rusqlite::params![act_id, stu_id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(note.is_none(), "메모 없이 저장하면 note는 NULL이어야 한다");
}

// ── 가져오기 전 원본 보존 (감사 F4) ───────────────────────────
//
// apply_replace·bulk_quick_replace는 덮어쓰기 전 원본을 히스토리에 남기는데
// 가져오기만 빠져 있었다. 셀 편집은 히스토리를 만들지 않는 설계이므로,
// 손으로 고친 뒤 재가져오기를 하면 복구 수단 없이 사라졌다.

#[test]
fn test_bulk_import_preserves_overwritten_content_in_history() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    // 선생님이 손으로 쓴 내용 (셀 편집은 히스토리를 만들지 않는다)
    upsert_record_impl(&conn, act_id, stu_id, "손으로 고친 내용", None).unwrap();

    // 엑셀에서 다시 가져오기 → 덮어씀
    bulk_import_records_impl(
        &conn,
        &[make_import(1, 1, 1, Some("홍길동"), act_id, "엑셀에서 가져온 내용")],
        None,
    )
    .unwrap();

    // 현재 내용은 가져온 것
    let current: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE activity_id=?1 AND student_id=?2",
            rusqlite::params![act_id, stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(current, "엑셀에서 가져온 내용");

    // 덮어쓰인 내용이 히스토리에 남아 되돌릴 수 있어야 한다
    let preserved: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ActivityRecordHistory WHERE content = '손으로 고친 내용'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        preserved, 1,
        "덮어쓰기 전 내용이 히스토리에 없으면 되돌릴 방법이 없다"
    );
}

#[test]
fn test_bulk_import_new_record_creates_no_pre_image() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");

    // 기존 기록이 없으면 남길 원본도 없다 — 빈 히스토리가 생기면 안 된다.
    bulk_import_records_impl(
        &conn,
        &[make_import(1, 1, 1, Some("신규"), act_id, "처음 가져온 내용")],
        None,
    )
    .unwrap();

    let empty_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ActivityRecordHistory WHERE note = '가져오기 전'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(empty_rows, 0, "새 기록에는 보존할 원본이 없다");
}

// ── 암호화 파일의 히스토리 중복 방지 ─────────────────────────────
//
// 암호문은 매번 다른 nonce로 만들어져 같은 평문이라도 값이 달라진다. 저장된
// 문자열끼리 비교하면 같은 내용이 다시 저장될 때마다 히스토리 행이 쌓인다.
// 평문 파일과 같은 결과가 나와야 한다.

fn enc_key() -> [u8; 32] {
    crate::crypto::derive_key("password", &[42u8; 16])
}

fn history_count(conn: &rusqlite::Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM ActivityRecordHistory", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn test_encrypted_reimport_same_content_adds_no_history() {
    let conn = setup_test_db();
    let key = Some(enc_key());
    let act_id = insert_activity(&conn, "발표");
    let rows = [make_import(1, 1, 1, Some("홍길동"), act_id, "같은 내용")];

    bulk_import_records_impl(&conn, &rows, key).unwrap();
    let after_first = history_count(&conn);
    assert_eq!(after_first, 1, "첫 가져오기는 import 행 하나를 남긴다");

    bulk_import_records_impl(&conn, &rows, key).unwrap();
    assert_eq!(
        history_count(&conn),
        after_first,
        "같은 CSV를 다시 가져와도 히스토리 행이 늘면 안 된다"
    );
}

#[test]
fn test_encrypted_save_snapshot_same_plaintext_updates_note_only() {
    let conn = setup_test_db();
    let key = Some(enc_key());
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "같은 내용", key).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, Some("첫 메모"), key).unwrap();
    assert_eq!(history_count(&conn), 1);

    // 같은 평문을 다시 저장하면 암호문이 새로 만들어진다
    upsert_record_impl(&conn, act_id, stu_id, "같은 내용", key).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, Some("두 번째"), key).unwrap();
    assert_eq!(history_count(&conn), 1, "내용이 같으면 새 행을 만들지 않는다");

    // 의도된 동작(CLAUDE.md): 가장 최근 행의 note가 이번 작업 이름으로 갱신된다
    let entries = get_record_history_impl(&conn, act_id, stu_id, 10, 0, key).unwrap();
    assert_eq!(entries[0].note.as_deref(), Some("두 번째"));
    assert_eq!(entries[0].content, "같은 내용");
}

#[test]
fn test_encrypted_save_snapshot_changed_content_adds_row() {
    let conn = setup_test_db();
    let key = Some(enc_key());
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "처음", key).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, None, key).unwrap();
    upsert_record_impl(&conn, act_id, stu_id, "바뀜", key).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, None, key).unwrap();

    let entries = get_record_history_impl(&conn, act_id, stu_id, 10, 0, key).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].content, "바뀜");
    assert_eq!(entries[1].content, "처음");
}

#[test]
fn test_encrypted_save_snapshot_compares_latest_not_oldest() {
    // A → B → A로 되돌린 경우 세 번째 A는 가장 최근 행(B)과 달라 새 행이 된다.
    // 가장 오래된 행(A)과 비교하면 "변경 없음"으로 보고 B 행의 note까지 덮는다.
    let conn = setup_test_db();
    let key = Some(enc_key());
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    for content in ["A", "B", "A"] {
        upsert_record_impl(&conn, act_id, stu_id, content, key).unwrap();
        save_snapshot_internal(&conn, act_id, stu_id, Some(content), key).unwrap();
    }

    let entries = get_record_history_impl(&conn, act_id, stu_id, 10, 0, key).unwrap();
    let contents: Vec<&str> = entries.iter().map(|e| e.content.as_str()).collect();
    assert_eq!(contents, ["A", "B", "A"]);
    assert_eq!(entries[1].note.as_deref(), Some("B"));
}

#[test]
fn test_save_snapshot_without_note_keeps_existing_note_on_dedup() {
    // 내용이 같을 때 note는 **이번 작업 이름으로** 갱신된다(CLAUDE.md). 이번 작업에
    // 이름이 없으면 갱신할 것이 없으므로 기존 note를 그대로 둔다 — 지우지 않는다.
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");

    upsert_record_impl(&conn, act_id, stu_id, "발표 내용", None).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, Some("메모"), None).unwrap();
    save_snapshot_internal(&conn, act_id, stu_id, None, None).unwrap();

    let entries = get_record_history_impl(&conn, act_id, stu_id, 10, 0, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].note.as_deref(), Some("메모"));
}

// ── 영역 범위 ────────────────────────────────────────────────

#[test]
fn test_bulk_quick_replace_skips_students_not_in_area() {
    // 활동이 영역에 속해도, 영역에 등록되지 않은 학생의 기록은 바꾸면 안 된다.
    let conn = setup_test_db();
    let area = insert_area(&conn, "영역", 500);
    let act = insert_activity(&conn, "활동");
    let member = insert_student(&conn, 1, 1, 1, "등록");
    let outsider = insert_student(&conn, 1, 1, 2, "미등록");
    conn.execute_batch(&format!(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES ({area}, {act});
         INSERT INTO AreaStudent (area_id, student_id) VALUES ({area}, {member});"
    ))
    .unwrap();
    insert_record(&conn, act, member, "사과");
    insert_record(&conn, act, outsider, "사과");

    let changed = bulk_quick_replace_impl(&conn, area, "사과", "배", None).unwrap();

    assert_eq!(changed, 1);
    let outsider_content: String = conn
        .query_row(
            "SELECT content FROM ActivityRecord WHERE student_id = ?1",
            [outsider],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(outsider_content, "사과");
}

#[test]
fn test_area_grid_returns_only_area_activity_student_pairs() {
    // 그리드에는 (영역 활동 × 영역 학생)의 기록만 와야 한다. 한쪽만 맞는 기록이 섞이면
    // 다른 영역의 내용(암호화 파일이면 복호화된 평문)까지 화면으로 넘어간다.
    let conn = setup_test_db();
    let area = insert_area(&conn, "영역", 500);
    let in_act = insert_activity(&conn, "영역 활동");
    let out_act = insert_activity(&conn, "다른 활동");
    let member = insert_student(&conn, 1, 1, 1, "등록");
    let outsider = insert_student(&conn, 1, 1, 2, "미등록");
    conn.execute_batch(&format!(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES ({area}, {in_act});
         INSERT INTO AreaStudent (area_id, student_id) VALUES ({area}, {member});"
    ))
    .unwrap();
    insert_record(&conn, in_act, member, "보여야 함");
    insert_record(&conn, out_act, member, "다른 활동 기록");
    insert_record(&conn, in_act, outsider, "다른 학생 기록");

    let grid = get_area_grid_impl(&conn, area, None).unwrap();

    let pairs: Vec<(i64, i64)> = grid.records.iter().map(|r| (r.activity_id, r.student_id)).collect();
    assert_eq!(pairs, [(in_act, member)]);
}

#[test]
fn test_bulk_import_existing_named_student_not_counted_as_updated() {
    // 이미 이름이 있는 학생은 이름을 바꾸지 않는다. 바꾸지 않았으면 갱신 수에도 들지 않는다.
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    insert_student(&conn, 1, 1, 1, "홍길동");

    let result =
        bulk_import_records_impl(&conn, &[make_import(1, 1, 1, Some("다른이름"), act_id, "내용")], None)
            .unwrap();

    assert_eq!(result.students_created, 0);
    assert_eq!(result.students_updated, 0);
}
