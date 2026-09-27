use crate::commands::activity::{
    create_activity_impl, delete_activity_impl, get_activities_impl, save_activity_impl,
    set_activity_areas_impl, update_activity_impl,
};
use super::{insert_activity, insert_area, insert_record, insert_student, setup_test_db};

#[test]
fn test_create_activity_returns_id() {
    let conn = setup_test_db();
    let id = create_activity_impl(&conn, "수행평가").unwrap();
    assert!(id > 0);
}

#[test]
fn test_create_activity_duplicate_name_error() {
    let conn = setup_test_db();
    create_activity_impl(&conn, "수행평가").unwrap();
    let err = create_activity_impl(&conn, "수행평가").unwrap_err();
    assert!(err.contains("이미 같은 이름의 활동"), "에러 메시지: {err}");
}

#[test]
fn test_get_activities_empty_db() {
    let conn = setup_test_db();
    let acts = get_activities_impl(&conn).unwrap();
    assert!(acts.is_empty());
}

#[test]
fn test_get_activities_with_areas() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "발표");
    let area_id = insert_area(&conn, "국어", 500);
    conn.execute(
        "INSERT INTO AreaActivity (area_id, activity_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, act_id],
    )
    .unwrap();

    let acts = get_activities_impl(&conn).unwrap();
    assert_eq!(acts.len(), 1);
    assert_eq!(acts[0].areas.len(), 1);
    assert_eq!(acts[0].areas[0].name, "국어");
}

#[test]
fn test_get_activities_record_count_zero() {
    let conn = setup_test_db();
    insert_activity(&conn, "보고서");
    let acts = get_activities_impl(&conn).unwrap();
    assert_eq!(acts[0].record_count, 0);
}

#[test]
fn test_get_activities_record_count_nonzero() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "실험");
    let stu1 = insert_student(&conn, 1, 1, 1, "홍길동");
    let stu2 = insert_student(&conn, 1, 1, 2, "김철수");
    insert_record(&conn, act_id, stu1, "실험 내용");
    insert_record(&conn, act_id, stu2, "두 번째 내용");

    let acts = get_activities_impl(&conn).unwrap();
    assert_eq!(acts[0].record_count, 2);
}

#[test]
fn test_update_activity_name() {
    let conn = setup_test_db();
    let id = create_activity_impl(&conn, "발표").unwrap();
    update_activity_impl(&conn, id, "발표(개정)").unwrap();

    let acts = get_activities_impl(&conn).unwrap();
    assert_eq!(acts[0].name, "발표(개정)");
}

#[test]
fn test_update_activity_duplicate_name_error() {
    let conn = setup_test_db();
    let id1 = create_activity_impl(&conn, "발표").unwrap();
    let id2 = create_activity_impl(&conn, "보고서").unwrap();
    let _ = id1;
    let err = update_activity_impl(&conn, id2, "발표").unwrap_err();
    assert!(err.contains("이미 같은 이름의 활동"), "에러 메시지: {err}");
}

#[test]
fn test_delete_activity_cascades_activity_records() {
    let conn = setup_test_db();
    let act_id = create_activity_impl(&conn, "삭제될활동").unwrap();
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    insert_record(&conn, act_id, stu_id, "내용");

    delete_activity_impl(&conn, act_id).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ActivityRecord WHERE activity_id=?1",
            rusqlite::params![act_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_set_activity_areas_replaces() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "과제");
    let area1 = insert_area(&conn, "국어", 500);
    let area2 = insert_area(&conn, "수학", 500);

    set_activity_areas_impl(&conn, act_id, &[area1]).unwrap();
    set_activity_areas_impl(&conn, act_id, &[area2]).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM AreaActivity WHERE activity_id=?1",
            rusqlite::params![act_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    let linked_area: i64 = conn
        .query_row(
            "SELECT area_id FROM AreaActivity WHERE activity_id=?1",
            rusqlite::params![act_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(linked_area, area2);
}

#[test]
fn test_set_activity_areas_empty_clears_all() {
    let conn = setup_test_db();
    let act_id = insert_activity(&conn, "과제");
    let area_id = insert_area(&conn, "영어", 400);
    set_activity_areas_impl(&conn, act_id, &[area_id]).unwrap();

    set_activity_areas_impl(&conn, act_id, &[]).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM AreaActivity WHERE activity_id=?1",
            rusqlite::params![act_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

// ── save_activity: 저장과 영역 연결을 한 트랜잭션으로 ─────────────
// 둘을 따로 부르던 때는 활동만 만들어진 채 연결에서 실패하면 영역 없는 활동이 남았고,
// 같은 모달에서 다시 저장하면 이름 UNIQUE에 걸렸다. 연결이 실패하면 활동 쪽도 되돌려야 한다.

fn activity_links(conn: &rusqlite::Connection, activity_id: i64) -> Vec<i64> {
    let mut stmt = conn
        .prepare("SELECT area_id FROM AreaActivity WHERE activity_id = ?1 ORDER BY area_id")
        .unwrap();
    stmt.query_map(rusqlite::params![activity_id], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
}

#[test]
fn test_save_activity_add_creates_activity_with_areas() {
    let conn = setup_test_db();
    let ar1 = insert_area(&conn, "자율", 500);
    let ar2 = insert_area(&conn, "진로", 500);

    let id = save_activity_impl(&conn, None, "봉사", &[ar1, ar2]).unwrap();

    let acts = get_activities_impl(&conn).unwrap();
    assert_eq!(acts.len(), 1);
    assert_eq!(acts[0].id, id);
    assert_eq!(acts[0].name, "봉사");
    assert_eq!(activity_links(&conn, id), vec![ar1, ar2]);
}

#[test]
fn test_save_activity_edit_updates_and_replaces_areas() {
    let conn = setup_test_db();
    let ar1 = insert_area(&conn, "자율", 500);
    let ar2 = insert_area(&conn, "진로", 500);
    let id = save_activity_impl(&conn, None, "봉사", &[ar1]).unwrap();

    let returned = save_activity_impl(&conn, Some(id), "봉사(개정)", &[ar2]).unwrap();

    assert_eq!(returned, id);
    let acts = get_activities_impl(&conn).unwrap();
    assert_eq!(acts[0].name, "봉사(개정)");
    assert_eq!(activity_links(&conn, id), vec![ar2]);
}

#[test]
fn test_save_activity_add_rolls_back_activity_when_link_fails() {
    let conn = setup_test_db();
    let ar1 = insert_area(&conn, "자율", 500);

    // 없는 영역 id → 연결 INSERT가 FK에 걸린다. 활동 INSERT는 이미 실행된 뒤다.
    let err = save_activity_impl(&conn, None, "봉사", &[ar1, 9999]);
    assert!(err.is_err());

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM Activity", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0, "연결이 실패하면 활동도 남으면 안 된다");
    let links: i64 = conn
        .query_row("SELECT COUNT(*) FROM AreaActivity", [], |r| r.get(0))
        .unwrap();
    assert_eq!(links, 0);

    // 같은 이름으로 다시 저장할 수 있어야 한다(UNIQUE에 걸리지 않는다).
    let id = save_activity_impl(&conn, None, "봉사", &[ar1]).unwrap();
    assert_eq!(activity_links(&conn, id), vec![ar1]);
}

#[test]
fn test_save_activity_edit_rolls_back_all_when_link_fails() {
    let conn = setup_test_db();
    let ar1 = insert_area(&conn, "자율", 500);
    let id = save_activity_impl(&conn, None, "봉사", &[ar1]).unwrap();

    let err = save_activity_impl(&conn, Some(id), "봉사(개정)", &[9999]);
    assert!(err.is_err());

    let acts = get_activities_impl(&conn).unwrap();
    assert_eq!(acts[0].name, "봉사", "이름 변경도 되돌려야 한다");
    assert_eq!(activity_links(&conn, id), vec![ar1], "기존 연결 삭제도 되돌려야 한다");
}
