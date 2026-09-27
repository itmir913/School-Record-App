use crate::commands::project::migrate_schema_impl;
use crate::db;
use crate::state::{CryptoState, CryptoStateHandle};
use rusqlite::Connection;

/// 암호화를 쓰지 않는 파일용. 키가 없으므로 마이그레이션은 버전만 올린다.
fn no_crypto() -> CryptoStateHandle {
    std::sync::Mutex::new(CryptoState { key: None })
}


fn temp_path(label: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let mut p = std::env::temp_dir();
    p.push(format!("school_test_{}_{}.db", label, nanos));
    p
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        rusqlite::params![name],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

#[test]
fn test_create_new_creates_required_tables() {
    let path = temp_path("create_tables");
    let conn = db::create_new(&path).unwrap();

    let required = [
        "Student",
        "Area",
        "Activity",
        "ActivityRecord",
        "ActivityRecordHistory",
        "Snapshot",
        "ReplaceRule",
        "SynonymGroup",
        "SynonymItem",
        "APP_CONFIGS",
    ];
    for table in &required {
        assert!(table_exists(&conn, table), "테이블 없음: {table}");
    }

    drop(conn);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_create_new_sets_schema_version() {
    let path = temp_path("schema_version");
    let conn = db::create_new(&path).unwrap();

    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, db::SCHEMA_VERSION);

    drop(conn);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_open_existing_same_version_ok() {
    let path = temp_path("open_existing");
    {
        let conn = db::create_new(&path).unwrap();
        drop(conn);
    }

    let result = db::open_existing(&path);
    assert!(result.is_ok(), "open_existing 실패: {:?}", result.err());

    let conn = result.unwrap();
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, db::SCHEMA_VERSION);

    drop(conn);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_open_too_new_returns_error() {
    let path = temp_path("too_new");
    {
        let conn = db::create_new(&path).unwrap();
        // user_version을 앱 버전보다 높게 수동 설정
        conn.pragma_update(None, "user_version", db::SCHEMA_VERSION + 1)
            .unwrap();
        drop(conn);
    }

    let result = db::open_existing(&path);
    assert!(
        matches!(result, Err(db::OpenError::TooNew { .. })),
        "TooNew 에러 예상, 실제: {:?}",
        result.map(|_| ())
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_open_existing_does_not_auto_migrate() {
    // open_existing은 마이그레이션을 수행하지 않아야 함
    let path = temp_path("no_auto_migrate");
    {
        let conn = db::create_new(&path).unwrap();
        conn.pragma_update(None, "user_version", 0u32).unwrap();
        drop(conn);
    }

    let conn = db::open_existing(&path).unwrap();
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 0, "open_existing가 마이그레이션을 수행하면 안 됨");

    drop(conn);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_migrate_schema_upgrades_to_current_version() {
    // migrate_schema_impl 호출 후 user_version이 SCHEMA_VERSION이 되어야 함
    let path = temp_path("migrate_upgrade");
    {
        let conn = db::create_new(&path).unwrap();
        conn.pragma_update(None, "user_version", 0u32).unwrap();
        drop(conn);
    }

    let mut conn = db::open_existing(&path).unwrap();
    migrate_schema_impl(&mut conn, &no_crypto()).unwrap();

    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, db::SCHEMA_VERSION);

    drop(conn);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_migrate_schema_is_noop_when_already_current() {
    // 이미 최신 버전이면 migrate_schema_impl은 아무것도 하지 않음
    let path = temp_path("migrate_noop");
    let mut conn = db::create_new(&path).unwrap();
    migrate_schema_impl(&mut conn, &no_crypto()).unwrap();

    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, db::SCHEMA_VERSION);

    drop(conn);
    let _ = std::fs::remove_file(&path);
}

// ── with_transaction ─────────────────────────────────────────

/// 트랜잭션이 열린 채 남아 있으면 BEGIN이 실패한다. 그 상태를 감지한다.
fn transaction_is_open(conn: &Connection) -> bool {
    match conn.execute_batch("BEGIN") {
        Ok(_) => {
            let _ = conn.execute_batch("ROLLBACK");
            false
        }
        Err(_) => true,
    }
}

#[test]
fn test_with_transaction_commits_on_ok() {
    let conn = crate::tests::setup_test_db();
    db::with_transaction(&conn, || {
        conn.execute("INSERT INTO Activity (name) VALUES ('발표')", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    })
    .unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM Activity", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert!(!transaction_is_open(&conn));
}

#[test]
fn test_with_transaction_rolls_back_on_err() {
    let conn = crate::tests::setup_test_db();
    let result: Result<(), String> = db::with_transaction(&conn, || {
        conn.execute("INSERT INTO Activity (name) VALUES ('발표')", [])
            .map_err(|e| e.to_string())?;
        Err("중간 실패".to_string())
    });
    assert!(result.is_err());

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM Activity", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0, "실패하면 삽입이 되돌아가야 한다");
}

#[test]
fn test_with_transaction_leaves_no_open_transaction_after_err() {
    // 트랜잭션이 열린 채 남으면 이후 모든 쓰기가 그 트랜잭션에 묶여
    // 앱 종료 시 통째로 사라진다. 실패 경로에서도 반드시 닫혀야 한다.
    let conn = crate::tests::setup_test_db();
    let _: Result<(), String> = db::with_transaction(&conn, || Err("실패".to_string()));

    assert!(
        !transaction_is_open(&conn),
        "실패 후에도 트랜잭션이 닫혀 있어야 한다"
    );
}

#[test]
fn test_with_transaction_early_return_inside_closure_rolls_back() {
    // 클로저 안에서 `?`로 조기 반환해도 ROLLBACK을 거친다.
    let conn = crate::tests::setup_test_db();
    let result: Result<(), String> = db::with_transaction(&conn, || {
        conn.execute("INSERT INTO Activity (name) VALUES ('발표')", [])
            .map_err(|e| e.to_string())?;
        let _missing: &str = None::<&str>.ok_or("필드 누락")?;
        Ok(())
    });

    assert!(result.is_err());
    assert!(!transaction_is_open(&conn));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM Activity", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_with_transaction_rolls_back_when_commit_fails() {
    // COMMIT 자체가 실패해도 트랜잭션은 열린 채 남는다. 되돌리지 않으면 이후 모든
    // 쓰기가 그 트랜잭션에 묶인다. 지연된 외래키 위반으로 COMMIT을 실패시킨다.
    let conn = crate::tests::setup_test_db();
    let result: Result<(), String> = db::with_transaction(&conn, || {
        conn.execute_batch(
            "PRAGMA defer_foreign_keys = ON;
             INSERT INTO Activity (name) VALUES ('발표');
             INSERT INTO AreaStudent (area_id, student_id) VALUES (999, 999);",
        )
        .map_err(|e| e.to_string())
    });

    assert!(result.is_err(), "COMMIT 실패가 오류로 올라와야 한다");
    assert!(!transaction_is_open(&conn), "COMMIT 실패 후에도 트랜잭션이 닫혀 있어야 한다");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM Activity", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

// ── constraint_err ───────────────────────────────────────────

#[test]
fn test_constraint_err_translates_check_violation() {
    // 진입부 검증이 놓친 CHECK 위반이 영문 원문으로 새어나가지 않는지 확인한다.
    let conn = crate::tests::setup_test_db();
    let e = conn
        .execute(
            "INSERT INTO Student (grade, class_num, number, name) VALUES (0, 1, 1, '홍길동')",
            [],
        )
        .unwrap_err();

    let msg = crate::state::constraint_err(&e, "중복입니다");
    assert!(!msg.contains("CHECK constraint"), "영문 원문 노출: {msg}");
    assert!(msg.contains("허용 범위"), "메시지: {msg}");
}

#[test]
fn test_constraint_err_translates_unique_violation() {
    let conn = crate::tests::setup_test_db();
    conn.execute("INSERT INTO Activity (name) VALUES ('발표')", []).unwrap();
    let e = conn
        .execute("INSERT INTO Activity (name) VALUES ('발표')", [])
        .unwrap_err();

    assert_eq!(crate::state::constraint_err(&e, "이미 있습니다"), "이미 있습니다");
}

#[test]
fn test_constraint_err_passes_through_other_errors() {
    let conn = crate::tests::setup_test_db();
    let e = conn.execute("INSERT INTO 없는테이블 (a) VALUES (1)", []).unwrap_err();

    let msg = crate::state::constraint_err(&e, "중복입니다");
    assert_ne!(msg, "중복입니다", "무관한 오류를 충돌 메시지로 바꾸면 안 된다");
    assert_eq!(msg, e.to_string(), "무관한 오류는 원문 그대로 넘겨야 한다");
}

/// 이 앱의 파일이 아니면 **1바이트도 건드리지 않고** 거부해야 한다.
///
/// 파일 대화상자는 확장자만 거른다. 통과시키면 그다음 단계들이 남의 DB에
/// `user_version`을 쓰고(migrate) 그 옆에 사본까지 만든다(backup).
#[test]
fn test_open_rejects_a_database_that_is_not_ours() {
    let path = temp_path("not_ours");
    {
        let other = rusqlite::Connection::open(&path).unwrap();
        other
            .execute_batch("CREATE TABLE Bookmarks (id INTEGER PRIMARY KEY);")
            .unwrap();
        other.pragma_update(None, "user_version", 7u32).unwrap();
    }
    let before = std::fs::read(&path).unwrap();

    let result = db::open_existing(&path);

    assert!(
        matches!(result, Err(db::OpenError::NotAppDatabase { .. })),
        "NotAppDatabase 예상, 실제: {:?}",
        result.map(|_| ())
    );
    let message = db::open_existing(&path).unwrap_err().to_string();
    assert!(
        message.contains("학생부 파일이 아닙니다"),
        "사용자가 읽을 수 있는 안내여야 한다: {message}"
    );
    // 문자열 줄 이음(`\`)이 빠지면 들여쓰기 공백이 그대로 안내문에 들어간다.
    assert!(!message.contains("  "), "안내문에 연속 공백이 섞였다: {message}");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "거부한 파일을 건드리면 안 된다"
    );

    let _ = std::fs::remove_file(&path);
}

/// 첫 릴리즈 시절의 파일(APP_CONFIGS·치환·유의어 테이블이 없다)은 **열려야 한다.**
///
/// 핵심 테이블만 보고 판별하는 이유가 이것이다. 나중에 추가된 테이블까지 요구하면
/// 옛 파일을 쓰던 사용자가 자기 파일을 열 방법을 잃는다.
#[test]
fn test_open_accepts_an_old_file_missing_later_tables() {
    let path = temp_path("old_shape");
    {
        let conn = db::create_new(&path).unwrap();
        conn.execute_batch(
            "DROP TABLE APP_CONFIGS; DROP TABLE SynonymItem;              DROP TABLE SynonymGroup; DROP TABLE ReplaceRule;",
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 0u32).unwrap();
    }

    assert!(
        db::open_existing(&path).is_ok(),
        "옛 파일은 열려야 한다"
    );

    let _ = std::fs::remove_file(&path);
}

/// 핵심 테이블은 하나만 빠져도 거부한다. 통과시키면 그 테이블을 쓰는 화면에서
/// `no such table`로 뒤늦게 실패한다.
#[test]
fn test_open_rejects_file_missing_any_core_table() {
    for table in [
        "Student",
        "Area",
        "Activity",
        "AreaActivity",
        "AreaStudent",
        "ActivityRecord",
        "ActivityRecordHistory",
        "Snapshot",
    ] {
        let path = temp_path(&format!("missing_{table}"));
        {
            let conn = db::create_new(&path).unwrap();
            conn.execute_batch(&format!("PRAGMA foreign_keys = OFF; DROP TABLE {table};"))
                .unwrap();
        }

        match db::open_existing(&path) {
            Err(db::OpenError::NotAppDatabase { missing }) => {
                assert!(missing.contains(table), "{table} 누락이 안내돼야 한다: {missing}")
            }
            other => panic!("{table}가 없는 파일은 거부해야 한다: {:?}", other.map(|_| ())),
        }

        let _ = std::fs::remove_file(&path);
    }
}
