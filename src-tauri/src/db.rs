use rusqlite::{Connection, Result};
use std::path::Path;

/// 현재 앱이 지원하는 스키마 버전.
/// 스키마 변경 시 이 값을 올리고 MIGRATIONS 배열에 SQL을 추가한다.
/// 중요: 스키마 버전을 올릴 때는 반드시 Cargo.toml의 version(app_version)도 함께 올려야 한다.
/// app_version이 바뀌지 않으면 릴리즈 노트 모달이 표시되지 않는다.
pub const SCHEMA_VERSION: u32 = 1;

/// 인덱스 i: 버전 i → i+1 로 올리는 SQL.
/// [0] v0→v1: 버전 도입 이전 DB를 v1으로 승격. 스키마는 IF NOT EXISTS로 생성되어 있으므로 SQL 없음.
const MIGRATIONS: &[&str] = &[
    "", // v0 → v1
];

// ── 내부 헬퍼 ────────────────────────────────────────────────

fn get_version(conn: &Connection) -> Result<u32> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
}

/// 현재 버전에서 SCHEMA_VERSION까지 마이그레이션을 단계별로 실행한다.
/// - 각 단계는 rusqlite Transaction으로 감싸 실패 시 자동 ROLLBACK된다.
/// - foreign_keys는 트랜잭션 외부에서만 변경 가능하므로, IIFE 종료 후 복구한다.
/// - 각 단계 커밋 전 PRAGMA foreign_key_check로 무결성을 검증한다.
pub fn migrate(conn: &mut Connection, from: u32) -> Result<()> {
    // foreign_keys 변경은 트랜잭션 외부에서만 유효 (SQLite 공식 권고)
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;

    // IIFE로 마이그레이션 실행 — 성공·실패 모두 이후 foreign_keys = ON 복구 보장
    let result: Result<()> = (|| {
        for v in from..SCHEMA_VERSION {
            let idx = v as usize;
            let sql = MIGRATIONS.get(idx).copied().ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(
                    format!("마이그레이션 스크립트 누락: v{v} → v{}", v + 1),
                )
            })?;

            // Transaction: 스코프 이탈(에러 포함) 시 자동 ROLLBACK
            let tx = conn.transaction()?;

            if !sql.is_empty() {
                tx.execute_batch(sql)?;
            }

            // user_version을 pragma_update API로 설정 (format! 없이 안전하게)
            tx.pragma_update(None, "user_version", v + 1)?;

            // 커밋 전 외래키 무결성 검증 — 위반 행이 하나라도 있으면 롤백
            {
                let mut stmt = tx.prepare("PRAGMA foreign_key_check;")?;
                if stmt.exists([])? {
                    return Err(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                        Some(format!(
                            "v{v} → v{} 마이그레이션 후 외래키 무결성 위반",
                            v + 1
                        )),
                    ));
                }
            }

            tx.commit()?;
        }
        Ok(())
    })();

    // 트랜잭션이 모두 닫힌 후 복구 — 열린 트랜잭션이 없으므로 PRAGMA가 반드시 적용됨
    // 복구 실패 시 conn이 foreign_keys = OFF 상태로 남으므로 에러로 처리
    let fk_result = conn.execute_batch("PRAGMA foreign_keys = ON;");

    // 마이그레이션 에러 우선, 복구 에러는 마이그레이션 성공 시에만 반환
    result.and(fk_result)
}

// ── 공개 API ─────────────────────────────────────────────────

/// 새 DB 파일 생성 후 스키마 초기화 및 버전 기록
pub fn create_new(db_path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    {
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("schema.sql"))?;
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        tx.commit()?;
    }
    Ok(conn)
}

/// 기존 DB 파일 열기 — 버전 검사만 수행 (마이그레이션은 migrate_schema 커맨드에서 별도 실행)
pub fn open_existing(db_path: &Path) -> Result<Connection, OpenError> {
    let conn = Connection::open(db_path).map_err(OpenError::Db)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;").map_err(OpenError::Db)?;

    let db_version = get_version(&conn).map_err(OpenError::Db)?;

    if db_version > SCHEMA_VERSION {
        return Err(OpenError::TooNew { db_version, app_version: SCHEMA_VERSION });
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
