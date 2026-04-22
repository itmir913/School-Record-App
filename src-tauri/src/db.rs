use rusqlite::{Connection, Result};
use std::path::Path;

/// 현재 앱이 지원하는 스키마 버전.
/// 스키마 변경 시 이 값을 올리고 MIGRATIONS 배열에 SQL을 추가한다.
pub const SCHEMA_VERSION: u32 = 3;

/// 인덱스 i: 버전 i → i+1 로 올리는 SQL.
/// [0] v0→v1: 버전 도입 이전 DB를 v1으로 승격. 스키마는 IF NOT EXISTS로 생성되어 있으므로 SQL 없음.
/// [1] v1→v2: 1초 단위 트리거 제거 — 히스토리 저장을 앱 레이어에서 명시적으로 제어.
const MIGRATIONS: &[&str] = &[
    // v0 → v1
    "",

    // v1 → v2
    "DROP TRIGGER IF EXISTS trg_record_history;",

    // v2 → v3
    "DROP TRIGGER IF EXISTS trg_display_order_on_activity_add;
     ALTER TABLE Area RENAME TO Area_old;
     CREATE TABLE Area (
         id         INTEGER PRIMARY KEY AUTOINCREMENT,
         name       TEXT    NOT NULL UNIQUE,
         byte_limit INTEGER NOT NULL
     );
     INSERT INTO Area SELECT * FROM Area_old;
     DROP TABLE Area_old;
     ALTER TABLE Activity RENAME TO Activity_old;
     CREATE TABLE Activity (
         id   INTEGER PRIMARY KEY AUTOINCREMENT,
         name TEXT NOT NULL UNIQUE
     );
     INSERT INTO Activity SELECT * FROM Activity_old;
     DROP TABLE Activity_old;",
];

// ── 내부 헬퍼 ────────────────────────────────────────────────

fn get_version(conn: &Connection) -> Result<u32> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
}

fn set_version(conn: &Connection, version: u32) -> Result<()> {
    conn.execute_batch(&format!("PRAGMA user_version = {version}"))
}

/// 현재 버전에서 SCHEMA_VERSION까지 마이그레이션을 단계별로 실행한다.
/// 각 단계는 트랜잭션으로 감싸져 있어 중간 실패 시 해당 단계만 롤백된다.
/// 테이블 재생성이 포함된 마이그레이션을 위해 foreign_keys를 OFF로 설정 후 실행한다.
fn migrate(conn: &Connection, from: u32) -> Result<()> {
    // SQLite 권고: 테이블 재생성(RENAME/CREATE/DROP) 포함 마이그레이션은
    // foreign_keys OFF 상태에서 실행해야 후속 구문에서 이름 변경된 테이블을 인식한다.
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
    for v in from..SCHEMA_VERSION {
        let idx = v as usize;
        // 배열 범위 초과 시 rusqlite 오류로 변환 (패닉 방지)
        let sql = MIGRATIONS.get(idx).copied().ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(
                format!("마이그레이션 스크립트 누락: v{v} → v{}", v + 1),
            )
        })?;
        conn.execute_batch(&format!(
            "BEGIN;\n{sql}\nPRAGMA user_version = {next};\nCOMMIT;",
            next = v + 1
        ))?;
    }
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(())
}

// ── 공개 API ─────────────────────────────────────────────────

/// 새 DB 파일 생성 후 스키마 초기화 및 버전 기록
pub fn create_new(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(include_str!("schema.sql"))?;
    set_version(&conn, SCHEMA_VERSION)?;
    Ok(conn)
}

/// 기존 DB 파일 열기 — 버전 검사 및 마이그레이션 수행
pub fn open_existing(db_path: &Path) -> Result<Connection, OpenError> {
    let conn = Connection::open(db_path).map_err(OpenError::Db)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;").map_err(OpenError::Db)?;

    let db_version = get_version(&conn).map_err(OpenError::Db)?;

    if db_version > SCHEMA_VERSION {
        return Err(OpenError::TooNew { db_version, app_version: SCHEMA_VERSION });
    }

    if db_version < SCHEMA_VERSION {
        migrate(&conn, db_version).map_err(OpenError::Db)?;
    }

    Ok(conn)
}

// ── 오류 타입 ─────────────────────────────────────────────────

#[derive(Debug)]
pub enum OpenError {
    Db(rusqlite::Error),
    /// DB 파일이 현재 앱보다 상위 버전
    TooNew { db_version: u32, app_version: u32 },
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Db(e) => write!(f, "데이터베이스 오류: {e}"),
            OpenError::TooNew { db_version, app_version } => write!(
                f,
                "이 파일은 더 최신 버전의 앱에서 생성되었습니다. \
                 앱을 업데이트해주세요. (파일 버전: v{db_version}, 현재 앱: v{app_version})"
            ),
        }
    }
}
