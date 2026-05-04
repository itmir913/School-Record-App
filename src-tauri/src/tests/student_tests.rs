use crate::commands::student::{
    bulk_upsert_students_impl, create_student_impl, delete_student_impl, get_area_students_impl,
    get_students_impl, set_area_activities_impl, set_area_students_impl, update_student_impl,
};
use crate::types::StudentInput;
use super::{insert_activity, insert_area, insert_record, insert_student, setup_test_db};

#[test]
fn test_create_student_returns_id() {
    let conn = setup_test_db();
    let id = create_student_impl(&conn, 1, 1, 1, "홍길동", None).unwrap();
    assert!(id > 0);
}

#[test]
fn test_create_student_duplicate_key_error() {
    let conn = setup_test_db();
    create_student_impl(&conn, 1, 1, 1, "홍길동", None).unwrap();
    let err = create_student_impl(&conn, 1, 1, 1, "홍길동", None).unwrap_err();
    assert!(err.contains("이미 같은 학번의 학생"), "에러 메시지: {err}");
}

#[test]
fn test_get_students_ordered_by_grade_class_number() {
    let conn = setup_test_db();
    insert_student(&conn, 2, 1, 3, "다");
    insert_student(&conn, 1, 2, 1, "나");
    insert_student(&conn, 1, 1, 5, "가");

    let students = get_students_impl(&conn, None).unwrap();
    assert_eq!(students[0].name, "가");
    assert_eq!(students[1].name, "나");
    assert_eq!(students[2].name, "다");
}

#[test]
fn test_update_student() {
    let conn = setup_test_db();
    let id = create_student_impl(&conn, 1, 1, 1, "홍길동", None).unwrap();
    update_student_impl(&conn, id, 2, 3, 10, "김철수", None).unwrap();

    let students = get_students_impl(&conn, None).unwrap();
    assert_eq!(students[0].name, "김철수");
    assert_eq!(students[0].grade, 2);
    assert_eq!(students[0].class_num, 3);
    assert_eq!(students[0].number, 10);
}

#[test]
fn test_update_student_duplicate_key_error() {
    let conn = setup_test_db();
    let id1 = create_student_impl(&conn, 1, 1, 1, "홍길동", None).unwrap();
    let id2 = create_student_impl(&conn, 1, 1, 2, "김철수", None).unwrap();
    let _ = id1;
    let err = update_student_impl(&conn, id2, 1, 1, 1, "김철수", None).unwrap_err();
    assert!(err.contains("이미 같은 학번의 학생"), "에러 메시지: {err}");
}

#[test]
fn test_delete_student() {
    let conn = setup_test_db();
    let id = create_student_impl(&conn, 1, 1, 1, "홍길동", None).unwrap();
    delete_student_impl(&conn, id).unwrap();

    let students = get_students_impl(&conn, None).unwrap();
    assert!(students.is_empty());
}

#[test]
fn test_delete_student_cascades_activity_records() {
    let conn = setup_test_db();
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    let act_id = insert_activity(&conn, "수행평가");
    insert_record(&conn, act_id, stu_id, "내용");

    delete_student_impl(&conn, stu_id).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ActivityRecord WHERE student_id=?1",
            rusqlite::params![stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_bulk_upsert_all_new() {
    let conn = setup_test_db();
    let students = vec![
        StudentInput { grade: 1, class_num: 1, number: 1, name: "가".to_string() },
        StudentInput { grade: 1, class_num: 1, number: 2, name: "나".to_string() },
    ];
    let result = bulk_upsert_students_impl(&conn, &students, None).unwrap();
    assert_eq!(result.inserted, 2);
    assert_eq!(result.updated, 0);
}

#[test]
fn test_bulk_upsert_all_existing() {
    let conn = setup_test_db();
    insert_student(&conn, 1, 1, 1, "홍길동");
    insert_student(&conn, 1, 1, 2, "김철수");

    let students = vec![
        StudentInput { grade: 1, class_num: 1, number: 1, name: "홍길동(변경)".to_string() },
        StudentInput { grade: 1, class_num: 1, number: 2, name: "김철수(변경)".to_string() },
    ];
    let result = bulk_upsert_students_impl(&conn, &students, None).unwrap();
    assert_eq!(result.inserted, 0);
    assert_eq!(result.updated, 2);

    let list = get_students_impl(&conn, None).unwrap();
    assert_eq!(list[0].name, "홍길동(변경)");
    assert_eq!(list[1].name, "김철수(변경)");
}

#[test]
fn test_bulk_upsert_mixed() {
    let conn = setup_test_db();
    insert_student(&conn, 1, 1, 1, "기존");

    let students = vec![
        StudentInput { grade: 1, class_num: 1, number: 1, name: "기존(갱신)".to_string() },
        StudentInput { grade: 1, class_num: 1, number: 2, name: "신규".to_string() },
    ];
    let result = bulk_upsert_students_impl(&conn, &students, None).unwrap();
    assert_eq!(result.inserted, 1);
    assert_eq!(result.updated, 1);
}

#[test]
fn test_bulk_upsert_empty_list() {
    let conn = setup_test_db();
    let result = bulk_upsert_students_impl(&conn, &[], None).unwrap();
    assert_eq!(result.inserted, 0);
    assert_eq!(result.updated, 0);
}

#[test]
fn test_get_area_students_returns_ids() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "수학", 500);
    let stu1 = insert_student(&conn, 1, 1, 1, "가");
    let stu2 = insert_student(&conn, 1, 1, 2, "나");
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu1],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu2],
    )
    .unwrap();

    let ids = get_area_students_impl(&conn, area_id).unwrap();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&stu1));
    assert!(ids.contains(&stu2));
}

#[test]
fn test_get_area_students_empty_area() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "과학", 400);
    let ids = get_area_students_impl(&conn, area_id).unwrap();
    assert!(ids.is_empty());
}

#[test]
fn test_set_area_students_replaces() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "영어", 400);
    let stu1 = insert_student(&conn, 1, 1, 1, "가");
    let stu2 = insert_student(&conn, 1, 1, 2, "나");

    set_area_students_impl(&conn, area_id, &[stu1]).unwrap();
    set_area_students_impl(&conn, area_id, &[stu2]).unwrap();

    let ids = get_area_students_impl(&conn, area_id).unwrap();
    assert_eq!(ids, vec![stu2]);
}

#[test]
fn test_set_area_students_empty_clears_all() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "체육", 300);
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    set_area_students_impl(&conn, area_id, &[stu_id]).unwrap();

    set_area_students_impl(&conn, area_id, &[]).unwrap();

    let ids = get_area_students_impl(&conn, area_id).unwrap();
    assert!(ids.is_empty());
}

#[test]
fn test_set_area_activities_replaces() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "미술", 300);
    let act1 = insert_activity(&conn, "소묘");
    let act2 = insert_activity(&conn, "채색");

    set_area_activities_impl(&conn, area_id, &[act1]).unwrap();
    set_area_activities_impl(&conn, area_id, &[act2]).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM AreaActivity WHERE area_id=?1",
            rusqlite::params![area_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    let linked: i64 = conn
        .query_row(
            "SELECT activity_id FROM AreaActivity WHERE area_id=?1",
            rusqlite::params![area_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(linked, act2);
}

#[test]
fn test_set_area_activities_empty_clears_all() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "음악", 300);
    let act_id = insert_activity(&conn, "합창");
    set_area_activities_impl(&conn, area_id, &[act_id]).unwrap();

    set_area_activities_impl(&conn, area_id, &[]).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM AreaActivity WHERE area_id=?1",
            rusqlite::params![area_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_delete_student_cascades_area_student() {
    let conn = setup_test_db();
    let area_id = insert_area(&conn, "수학", 500);
    let stu_id = insert_student(&conn, 1, 1, 1, "홍길동");
    conn.execute(
        "INSERT INTO AreaStudent (area_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![area_id, stu_id],
    ).unwrap();

    delete_student_impl(&conn, stu_id).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM AreaStudent WHERE student_id=?1",
            rusqlite::params![stu_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_get_students_empty() {
    let conn = setup_test_db();
    let students = get_students_impl(&conn, None).unwrap();
    assert!(students.is_empty());
}

// ── CHECK 제약 검증 ────────────────────────────────────────────

#[test]
fn test_create_student_grade_zero_violates_check() {
    let conn = setup_test_db();
    let err = create_student_impl(&conn, 0, 1, 1, "홍길동", None).unwrap_err();
    assert!(err.contains("CHECK constraint failed"), "grade=0 CHECK 위반이어야 함: {err}");
}

#[test]
fn test_create_student_class_num_zero_violates_check() {
    let conn = setup_test_db();
    let err = create_student_impl(&conn, 1, 0, 1, "홍길동", None).unwrap_err();
    assert!(err.contains("CHECK constraint failed"), "class_num=0 CHECK 위반이어야 함: {err}");
}

#[test]
fn test_create_student_number_zero_violates_check() {
    let conn = setup_test_db();
    let err = create_student_impl(&conn, 1, 1, 0, "홍길동", None).unwrap_err();
    assert!(err.contains("CHECK constraint failed"), "number=0 CHECK 위반이어야 함: {err}");
}

#[test]
fn test_update_student_negative_grade_violates_check() {
    let conn = setup_test_db();
    let id = create_student_impl(&conn, 1, 1, 1, "홍길동", None).unwrap();
    let err = update_student_impl(&conn, id, -1, 1, 1, "홍길동", None).unwrap_err();
    assert!(err.contains("CHECK constraint failed"), "grade=-1 CHECK 위반이어야 함: {err}");
}
