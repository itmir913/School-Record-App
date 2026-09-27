use rusqlite::{Connection, OptionalExtension, Result};
use std::path::Path;

/// 현재 앱이 지원하는 스키마 버전.
///
/// 정식 출시 이후이므로 사용자 PC에는 이미 특정 구조의 DB 파일이 존재한다.
///
/// ⚠️ **이 값은 소유자의 명시적 승인 없이 올리지 않는다.** 에이전트·AI는 schema.sql
/// 변경이나 테스트 실패를 이유로 스스로 올리지 말고, 사용자에게 사유를 설명해 확정을
/// 받은 뒤에만 아래 절차를 수행한다.
///
/// 승인 후 schema.sql을 고칠 때는 예외 없이 다음을 함께 해야 한다.
///   1. 이 값을 올린다.
///   2. MIGRATIONS에 이전 버전 → 새 버전 SQL을 추가한다.
///   3. tests/schema_history/vN.sql 스냅샷을 추가한다 (기존 파일은 수정 금지).
/// 이를 빠뜨리면 `tests/schema_lock_tests.rs`가 실패한다. 상세 절차는 해당 파일 참고.
///
/// **DDL이 그대로여도 버전을 올려야 하는 경우가 있다.** `ENCRYPTED_COLUMNS`에 컬럼을
/// 넣거나 빼면 테이블 모양은 같은데 그 컬럼에 담긴 값의 표현이 달라진다. 어느 파일이
/// 이미 변환됐는지 구분할 표식이 필요하고, user_version이 그 표식이다. v2가 그 예다.
/// (빼는 쪽은 대응하는 복호화 훅이 아직 없다 — commands/crypto.rs 참고)
///
/// 중요: 스키마 버전을 올릴 때는 반드시 tauri.conf.json의 version(app_version)도 함께 올려야 한다.
/// app_version이 바뀌지 않으면 릴리즈 노트 모달이 표시되지 않는다.
/// (Cargo.toml의 version은 0.0.0 고정 — 실제 앱 버전은 tauri.conf.json이 기준이다)
pub const SCHEMA_VERSION: u32 = 2;

/// 인덱스 i: 버전 i → i+1 로 올리는 SQL.
/// [0] v0→v1: 버전 도입 이전 DB를 v1으로 승격. 스키마는 IF NOT EXISTS로 생성되어 있으므로 SQL 없음.
/// [1] v1→v2: DDL 변경 없음. ActivityRecordHistory.note / Snapshot.memo가 암호화 대상이 된
///            데이터 표현 변경이며, 그 변환은 SQL로 할 수 없어 `migrate`의 data_step으로 간다.
pub(crate) const MIGRATIONS: &[&str] = &[
    "", // v0 → v1
    "", // v1 → v2
];

/// BEGIN ~ COMMIT/ROLLBACK을 감싸 트랜잭션이 열린 채 남지 않도록 보장한다.
///
/// DB Connection은 Mutex로 공유되는 하나뿐이다. 트랜잭션을 연 채 함수를 빠져나가면
/// 세션 내내 그 상태로 남아, 이후 모든 BEGIN이 실패하고 트랜잭션 없는 쓰기(셀 편집 등)가
/// 열린 트랜잭션에 묶인다. 그 상태로 앱을 닫으면 작업이 통째로 롤백된다.
///
/// 따라서 트랜잭션 안에서 조기 반환이 필요하면 반드시 클로저 안에서 `?`를 쓴다.
/// 클로저 밖에서 `?`를 쓰면 ROLLBACK을 건너뛴다.
pub fn with_transaction<T>(
    conn: &Connection,
    action: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    match action() {
        Ok(value) => {
            if let Err(e) = conn.execute_batch("COMMIT") {
                // COMMIT 실패도 트랜잭션을 열어둔다. 반드시 되돌린다.
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e.to_string());
            }
            Ok(value)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

// ── 내부 헬퍼 ────────────────────────────────────────────────

fn get_version(conn: &Connection) -> Result<u32> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
}

/// 마이그레이션 한 단계에서 실행할 데이터 변환.
///
/// 인자는 (그 단계의 트랜잭션, 올라갈 목표 버전)이다. **버전 승격과 같은 트랜잭션 안에서**
/// 호출되므로, Err를 돌려주면 데이터 변환과 user_version 승격이 함께 롤백된다.
///
/// 이 훅이 있는 이유는 SQL만으로 할 수 없는 변환이 있기 때문이다. 암호화 키가 필요한
/// 변환이 그렇다. **키가 필요한 단계에 아무것도 하지 않는 훅을 넘기면, 옛 표현의 데이터를
/// 담은 채 버전만 오른 파일이 만들어진다.** 그 파일은 다음에 열릴 때 새 표현으로 읽히므로
/// 복구가 안 된다. 프로덕션 경로는 commands/project.rs의 migrate_schema_impl 하나뿐이다.
pub type MigrationDataStep<'a> = &'a dyn Fn(&Connection, u32) -> Result<(), String>;

/// 현재 버전에서 SCHEMA_VERSION까지 마이그레이션을 단계별로 실행한다.
/// - 각 단계는 rusqlite Transaction으로 감싸 실패 시 자동 ROLLBACK된다.
/// - DDL → `data_step` → user_version 승격이 **한 트랜잭션**이다.
/// - foreign_keys는 트랜잭션 외부에서만 변경 가능하므로, IIFE 종료 후 복구한다.
/// - 각 단계 커밋 전 PRAGMA foreign_key_check로 무결성을 검증한다.
pub fn migrate(
    conn: &mut Connection,
    from: u32,
    data_step: MigrationDataStep<'_>,
) -> Result<(), String> {
    // foreign_keys 변경은 트랜잭션 외부에서만 유효 (SQLite 공식 권고)
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .map_err(|e| e.to_string())?;

    // IIFE로 마이그레이션 실행 — 성공·실패 모두 이후 foreign_keys = ON 복구 보장
    let result: Result<(), String> = (|| {
        for v in from..SCHEMA_VERSION {
            let idx = v as usize;
            let sql = MIGRATIONS
                .get(idx)
                .copied()
                .ok_or_else(|| format!("마이그레이션 스크립트 누락: v{v} → v{}", v + 1))?;

            // Transaction: 스코프 이탈(에러 포함) 시 자동 ROLLBACK
            let tx = conn.transaction().map_err(|e| e.to_string())?;

            if !sql.is_empty() {
                tx.execute_batch(sql).map_err(|e| e.to_string())?;
            }

            // 지켜야 하는 것은 **이 두 문장이 같은 트랜잭션 안에 있다**는 것이다.
            // (둘의 앞뒤 순서는 상관없다 — 어느 쪽이 먼저든 실패하면 함께 롤백된다.)
            // data_step을 트랜잭션 밖으로 빼는 순간 "새 버전인데 데이터는 옛 표현"인
            // 파일이 만들어지고, 그 파일은 다음에 열릴 때 복구가 안 된다.
            //
            // data_step 쪽은 테스트가 잡는다. **user_version 승격 쪽은 못 잡는다** —
            // 커밋 밖으로 빼도 실패 시에는 승격 자체가 일어나지 않아 기존 단언이
            // 그대로 참이고, 차이는 커밋과 승격 사이의 크래시 창에서만 드러난다.
            // 그래서 구조로 막는다: 승격은 `tx`를 받아야만 쓸 수 있다.
            // (`with_purge_marked_transaction`이 표시를 함수 안에 가둔 것과 같은 이유)
            data_step(&tx, v + 1)?;

            // user_version을 pragma_update API로 설정 (format! 없이 안전하게)
            tx.pragma_update(None, "user_version", v + 1)
                .map_err(|e| e.to_string())?;

            // 커밋 전 외래키 무결성 검증 — 위반 행이 하나라도 있으면 롤백.
            //
            // **DDL을 실제로 실행한 단계에서만 돈다.** 이 검사는 전 DB를 훑으므로,
            // 스키마를 바꾸지 않는 단계에서 돌리면 마이그레이션이 만든 위반이 아니라
            // **원래 파일에 있던** 위반을 잡는다. 그러면 그 파일은 열 때마다 같은
            // 지점에서 멈추고, 데이터는 멀쩡한데 앱 안에 탈출구가 없어진다.
            //
            // v2가 그 경우였다. SCHEMA_VERSION이 줄곧 1이라 db::migrate는 배포된
            // 파일에서 한 번도 실행된 적이 없었고, v1→v2는 DDL이 없는데도 모든
            // 사용자 파일이 이 검사를 난생처음 통과해야 했다.
            //
            // 데이터 변환(data_step)이 FK 컬럼을 건드리는 단계를 새로 만든다면
            // 그때는 이 조건을 함께 손봐야 한다. 지금까지의 변환은 TEXT 컬럼의
            // 값만 바꾼다.
            if !sql.is_empty() {
                let mut stmt = tx
                    .prepare("PRAGMA foreign_key_check;")
                    .map_err(|e| e.to_string())?;
                if stmt.exists([]).map_err(|e| e.to_string())? {
                    return Err(format!(
                        "v{v} → v{} 마이그레이션 후 외래키 무결성 위반",
                        v + 1
                    ));
                }
            }

            tx.commit().map_err(|e| e.to_string())?;
        }
        Ok(())
    })();

    // 트랜잭션이 모두 닫힌 후 복구 — 열린 트랜잭션이 없으므로 PRAGMA가 반드시 적용됨
    // 복구 실패 시 conn이 foreign_keys = OFF 상태로 남으므로 에러로 처리
    let fk_result = conn
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| e.to_string());

    // 둘 다 실패하면 `Result::and`는 복구 실패를 버린다. 그러면 커넥션이 세션 내내
    // foreign_keys = OFF로 남아 CASCADE가 안 도는데 아무도 모른다. 합쳐서 올린다.
    //
    // (Err, Err) 분기는 **테스트로 고정할 수 없다.** rusqlite에서
    // `PRAGMA foreign_keys = ON`은 트랜잭션 안에서도 오류가 아니라 조용히 무시되므로
    // 인위적으로 실패시킬 방법이 없다. 지키는 테스트가 없다는 것을 알고 남긴다.
    match (result, fk_result) {
        (Ok(()), fk) => fk,
        (Err(e), Ok(())) => Err(e),
        (Err(e), Err(fk)) => Err(format!("{e}
외래키 설정 복구에도 실패했습니다: {fk}")),
    }
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

/// 어느 버전에서도 이 앱의 파일에 반드시 있는 테이블.
///
/// 파일 대화상자는 확장자만 거르므로 다른 프로그램의 `.db`나 잘려서 0바이트가 된
/// 파일이 들어올 수 있다. 예전에는 `get_encryption_status`가 `APP_CONFIGS`를 찾지
/// 못해 **우연히** 걸러냈는데, 그 조회는 이 앱의 옛 파일(v0)까지 같이 막았다.
/// 우연에 기대지 않고 여기서 판별한다.
///
/// 목록에는 **첫 릴리즈부터 있던 테이블만** 넣는다. 나중에 추가된 것을 넣으면 옛
/// 파일이 거부되고, 그건 사용자가 자기 파일을 잃는 것과 같다. 나중에 추가된
/// 테이블을 메우는 일은 마이그레이션의 몫이다.
const CORE_TABLES: &[&str] = &[
    "Student",
    "Area",
    "Activity",
    "AreaActivity",
    "AreaStudent",
    "ActivityRecord",
    "ActivityRecordHistory",
    "Snapshot",
];

/// 기존 DB 파일 열기 — 이 앱의 파일인지와 버전만 검사한다
/// (마이그레이션은 migrate_schema 커맨드에서 별도 실행).
///
/// **여기서 거부한 파일은 1바이트도 건드리지 않는다.** 통과시키면 그다음 단계들이
/// 남의 DB에 `user_version`을 쓰고 그 옆에 사본까지 만든다.
pub fn open_existing(db_path: &Path) -> Result<Connection, OpenError> {
    let conn = Connection::open(db_path).map_err(OpenError::Db)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;").map_err(OpenError::Db)?;

    // 조회 실패를 "테이블 없음"으로 뭉개지 않는다. 잠긴 파일을 "남의 DB"라고
    // 잘못 알리면 사용자가 멀쩡한 자기 파일을 의심하게 된다.
    let mut missing: Vec<&str> = Vec::new();
    for table in CORE_TABLES {
        let found = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |_| Ok(()),
            )
            .optional()
            .map_err(OpenError::Db)?
            .is_some();
        if !found {
            missing.push(table);
        }
    }
    if !missing.is_empty() {
        return Err(OpenError::NotAppDatabase { missing: missing.join(", ") });
    }

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
    /// 이 앱이 만든 학생부 파일이 아니다
    NotAppDatabase { missing: String },
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
            OpenError::NotAppDatabase { missing } => write!(
                f,
                "이 프로그램에서 만든 학생부 파일이 아닙니다.                  다른 파일을 선택해주세요. (파일 안에 {missing} 정보가 없습니다)"
            ),
        }
    }
}
