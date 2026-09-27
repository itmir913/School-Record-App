//! 스키마 고정(lock) 테스트.
//!
//! 목적: schema.sql이 바뀌었는데 `SCHEMA_VERSION`을 올리지 않은 채 배포되는 사고를 막는다.
//! 정식 출시 이후에는 사용자 PC에 이미 특정 구조의 DB 파일이 존재하므로,
//! 스키마 변경에는 반드시 버전 bump + 마이그레이션이 따라야 한다.
//!
//! ## ⚠️ 스키마 버전은 소유자의 명시적 승인 없이 올리지 않는다
//! 이 테스트가 실패했다고 버전을 올려 맞추지 말 것. 스키마 버전 bump는 배포된 모든
//! 파일을 되돌릴 수 없게 바꾸는 매우 민감한 결정이다. **에이전트·AI는 여기서 멈추고
//! 사용자(소유자)에게 변경 사유를 설명해 확정을 받은 뒤에만** 아래 절차를 수행한다.
//! 승인이 없으면 schema.sql·ENCRYPTED_COLUMNS 변경 자체를 되돌리는 것이 기본이다.
//!
//! ## 스키마를 변경할 때의 절차 (vN → vN+1, 승인 후에만)
//! 1. `schema.sql`을 수정한다.
//! 2. 기존 `tests/schema_history/vN.sql`은 **그대로 둔다**(배포된 구조의 기록).
//!    수정한 `schema.sql`을 `tests/schema_history/vN+1.sql`로 복사한다.
//! 3. `db.rs`의 `SCHEMA_VERSION`을 올리고, `MIGRATIONS`에 vN→vN+1 SQL을 추가한다.
//! 4. `SCHEMA_BASELINES`에 새 파일, `SCHEMA_FINGERPRINTS`에 새 지문을 추가한다.
//!    (지문 값은 이 테스트 실패 메시지에 실제 값이 출력된다)
//! 5. `tauri.conf.json`의 앱 버전도 함께 올린다 (릴리즈 노트 모달 표시 조건).
//!
//! 1~4 중 하나라도 빠지면 이 모듈의 테스트가 실패한다.
//!
//! ## DDL이 그대로여도 버전을 올려야 하는 경우
//! `ENCRYPTED_COLUMNS`(commands/crypto.rs)에 컬럼을 넣거나 빼면 테이블 모양은 같은데
//! 그 컬럼에 담긴 값의 표현이 달라진다. 어느 파일이 이미 변환됐는지 구분할 표식이
//! user_version뿐이므로 이때도 버전을 올린다. v1 → v2가 그 경우였고, 그래서
//! **두 버전의 지문이 같은 값이다.** 지문이 같다고 버전을 합치면 안 된다.
//! (빼는 쪽은 대응하는 복호화 훅이 아직 없다 — commands/crypto.rs 참고)

use crate::db;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

/// 버전별 스키마 지문(정규화된 sqlite_master 덤프의 SHA-256).
/// 인덱스 i = 스키마 버전 i+1.
///
/// ⚠️ 기존 항목은 절대 수정 금지. 이미 배포된 DB 파일의 구조를 기록한 값이다.
/// 스키마가 바뀌었다면 기존 값을 고치는 것이 아니라 새 항목을 추가해야 한다.
pub(crate) const SCHEMA_FINGERPRINTS: &[&str] = &[
    // v1 — 정식 출시 스키마
    "fc7c11a3d03c8d4a104f2ec9788745928bcb11cef517bab90b0be463dedb6be2",
    // v2 — DDL은 v1과 같다(그래서 지문도 같다). 바뀐 것은 note/memo의 암호화 여부다.
    "fc7c11a3d03c8d4a104f2ec9788745928bcb11cef517bab90b0be463dedb6be2",
];

/// 버전별 스키마 원본. 인덱스 i = 스키마 버전 i+1.
///
/// ⚠️ 기존 파일은 절대 수정 금지 (`SCHEMA_FINGERPRINTS`와 같은 이유).
const SCHEMA_BASELINES: &[&str] = &[
    include_str!("schema_history/v1.sql"), // v1
    include_str!("schema_history/v2.sql"), // v2
];

// ── 헬퍼 ─────────────────────────────────────────────────────

/// sqlite_master를 정규화해 덤프한다.
/// 공백은 단일 스페이스로 축약하므로 들여쓰기·줄바꿈만 바뀐 경우는 동일하게 취급된다.
fn schema_dump(conn: &Connection) -> String {
    let mut stmt = conn
        .prepare(
            "SELECT type, name, COALESCE(sql, '') FROM sqlite_master
             WHERE name NOT LIKE 'sqlite\\_%' ESCAPE '\\'
             ORDER BY type, name",
        )
        .unwrap();

    let rows = stmt
        .query_map([], |r| {
            let obj_type: String = r.get(0)?;
            let name: String = r.get(1)?;
            let sql: String = r.get(2)?;
            let normalized = sql.split_whitespace().collect::<Vec<_>>().join(" ");
            Ok(format!("{obj_type}|{name}|{normalized}"))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    rows.join("\n")
}

fn fingerprint(conn: &Connection) -> String {
    let mut hasher = Sha256::new();
    hasher.update(schema_dump(conn).as_bytes());
    // sha2 0.11의 finalize()는 LowerHex를 구현하지 않는 Array를 반환하므로 직접 hex 변환한다.
    // 바이트당 소문자 2자리 = 이전 `{:x}` 출력과 동일한 문자열이어야 한다.
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 현재 schema.sql로 만든 DB (= 신규 프로젝트 생성 결과)
fn fresh_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(include_str!("../schema.sql")).unwrap();
    conn
}

/// 지정 버전 시점의 스키마로 만든 DB
fn baseline_db(version: u32) -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(SCHEMA_BASELINES[(version - 1) as usize])
        .unwrap();
    conn.pragma_update(None, "user_version", version).unwrap();
    conn
}

// ── 테스트 ───────────────────────────────────────────────────

#[test]
fn test_fresh_schema_matches_locked_fingerprint() {
    let conn = fresh_db();
    let actual = fingerprint(&conn);
    let expected = *SCHEMA_FINGERPRINTS
        .get((db::SCHEMA_VERSION - 1) as usize)
        .unwrap_or_else(|| {
            panic!(
                "SCHEMA_VERSION={}에 해당하는 지문이 SCHEMA_FINGERPRINTS에 없습니다. \
                 새 버전의 지문 {}을(를) 추가하세요. \
                 (스키마 버전 bump는 소유자의 명시적 승인을 받은 경우에만 허용된다)",
                db::SCHEMA_VERSION, actual
            )
        });

    assert_eq!(
        actual,
        expected,
        "\n\n\
         ===== 스키마가 변경되었습니다 (v{ver} 지문 불일치) =====\n\
         schema.sql이 스키마 버전 {ver}로 고정된 구조와 다릅니다.\n\
         버전을 올리지 않고 배포하면 기존 사용자의 DB 파일이 마이그레이션되지 않습니다.\n\n\
         ⚠️ 에이전트·AI라면 스스로 버전을 올리지 말고 여기서 멈추세요.\n\
         스키마 버전 bump는 사용자(소유자)의 명시적 승인을 받은 뒤에만 허용됩니다.\n\
         승인이 없으면 schema.sql 변경을 되돌리는 것이 기본입니다.\n\
         승인을 받았다면 schema_lock_tests.rs 상단의 '스키마를 변경할 때의 절차'를 따르세요.\n\
         새 버전의 지문: {actual}\n\n\
         현재 schema.sql 덤프:\n{dump}\n",
        ver = db::SCHEMA_VERSION,
        actual = actual,
        dump = schema_dump(&conn),
    );
}

#[test]
fn test_baseline_snapshots_match_locked_fingerprints() {
    // 과거 버전 스냅샷 파일이 몰래 수정되지 않았는지 확인한다.
    for (i, expected) in SCHEMA_FINGERPRINTS.iter().enumerate() {
        let version = (i + 1) as u32;
        let conn = baseline_db(version);
        assert_eq!(
            &fingerprint(&conn),
            expected,
            "schema_history/v{version}.sql이 수정되었습니다. \
             배포된 구조의 기록이므로 되돌려야 합니다."
        );
    }
}

#[test]
fn test_migration_path_matches_fresh_install() {
    // 모든 과거 버전 DB가 마이그레이션 후 신규 설치와 동일한 구조가 되어야 한다.
    let target = fingerprint(&fresh_db());

    for i in 0..SCHEMA_BASELINES.len() {
        let version = (i + 1) as u32;
        let mut conn = baseline_db(version);
        // 이 테스트가 보는 것은 스키마의 **모양**이다. 데이터 변환은 지문에 영향을 주지
        // 않으므로 여기서는 아무것도 하지 않는 훅을 넘긴다.
        // 프로덕션 경로(migrate_schema_impl)는 암호화 키를 받는 훅을 넘긴다 — 그쪽이
        // no-op으로 도는 일이 없는지는 migration_v2_tests가 본다.
        db::migrate(&mut conn, version, &|_, _| Ok(())).unwrap();

        assert_eq!(
            fingerprint(&conn),
            target,
            "\n\nv{version} DB를 마이그레이션한 결과가 신규 설치 스키마와 다릅니다.\n\
             MIGRATIONS[{i}..]가 schema.sql의 변경 내용을 모두 반영하는지 확인하세요.\n\n\
             마이그레이션 결과:\n{migrated}\n\n신규 설치:\n{fresh}\n",
            migrated = schema_dump(&conn),
            fresh = schema_dump(&fresh_db()),
        );

        let user_version: u32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(user_version, db::SCHEMA_VERSION);
    }
}

#[test]
fn test_migrations_cover_every_schema_version() {
    assert_eq!(
        db::MIGRATIONS.len(),
        db::SCHEMA_VERSION as usize,
        "SCHEMA_VERSION={}이면 MIGRATIONS는 {}개여야 합니다 (v0→v1 … v{}→v{}).",
        db::SCHEMA_VERSION,
        db::SCHEMA_VERSION,
        db::SCHEMA_VERSION - 1,
        db::SCHEMA_VERSION,
    );
}

#[test]
fn test_lock_tables_cover_every_schema_version() {
    assert_eq!(
        SCHEMA_FINGERPRINTS.len(),
        db::SCHEMA_VERSION as usize,
        "SCHEMA_VERSION을 올렸다면 SCHEMA_FINGERPRINTS에도 항목을 추가해야 합니다."
    );
    assert_eq!(
        SCHEMA_BASELINES.len(),
        db::SCHEMA_VERSION as usize,
        "SCHEMA_VERSION을 올렸다면 schema_history/vN.sql을 추가하고 \
         SCHEMA_BASELINES에 등록해야 합니다."
    );
}

// ── 암호화 대상 컬럼 고정 ────────────────────────────────────
//
// schema.sql이 그대로여도 ENCRYPTED_COLUMNS가 바뀌면 저장된 값의 표현이 달라진다.
// 위 지문 테스트들은 sqlite_master만 보므로 그 변화를 감지하지 못한다. 여기서 본다.

/// 버전별 암호화 대상. 인덱스 i = 스키마 버전 i+1에서 **새로 추가된** 컬럼들.
/// 각 항목은 (table, column, skip_empty).
///
/// ⚠️ 기존 슬라이스는 절대 수정 금지. 이미 배포된 파일의 데이터 표현 기록이다.
///
/// 버전별로 나눈 이유가 있다. 컬럼을 하나의 평평한 목록에 두면, 새 컬럼에
/// **현재 버전**을 적고 `SCHEMA_VERSION`은 올리지 않는 실수를 아무것도 막지 못한다.
/// 그 결과는 이렇다 — 이미 그 버전인 사용자 파일은 마이그레이션이 다시 돌지 않으므로
/// 기존 행은 평문으로 남고 새로 쓰는 것만 암호문이 된다. 한 컬럼에 둘이 섞이면
/// `decrypt_all_data`가 실패해 **암호화 해제와 비밀번호 변경이 영구히 막힌다.**
///
/// 나눠 두면 컬럼 추가 = 새 슬라이스 추가 = `SCHEMA_VERSION` bump가 아래 길이 단언으로
/// 강제된다(`schema_history/vN.sql`과 같은 구조다).
///
/// 새 슬라이스를 추가할 때 함께 확인할 것:
///   - 그 컬럼을 읽고 쓰는 **개별 행 경로**를 전부 고쳤는가 (SQL 문자열 안의 리터럴 포함)
///   - nullable이면 `skip_empty`가 true인가 (아래 테스트가 강제한다)
const LOCKED_ENCRYPTED_COLUMNS: &[&[(&str, &str, bool)]] = &[
    // v1 — 정식 출시 스키마
    &[
        ("Student", "name", false),
        ("ActivityRecord", "content", true),
        ("ActivityRecordHistory", "content", true),
    ],
    // v2 — 기록 히스토리 메모와 스냅샷 메모
    &[
        ("ActivityRecordHistory", "note", true),
        ("Snapshot", "memo", true),
    ],
];

#[test]
fn test_encrypted_columns_match_locked_list() {
    let expected: Vec<(&str, &str, bool, u32)> = LOCKED_ENCRYPTED_COLUMNS
        .iter()
        .enumerate()
        .flat_map(|(i, cols)| cols.iter().map(move |(t, c, s)| (*t, *c, *s, (i + 1) as u32)))
        .collect();
    let actual: Vec<(&str, &str, bool, u32)> = crate::commands::crypto::ENCRYPTED_COLUMNS
        .iter()
        .map(|c| (c.table, c.column, c.skip_empty, c.since_version))
        .collect();
    assert_eq!(
        actual, expected,
        "암호화 대상 컬럼이 바뀌었습니다. LOCKED_ENCRYPTED_COLUMNS 주석의 확인 사항을 먼저 읽으세요. \
         이 변경은 스키마 버전 bump를 부르므로 소유자의 명시적 승인 없이 진행하지 마세요."
    );
}

#[test]
fn test_encrypted_columns_cover_every_schema_version() {
    // 컬럼을 추가하려면 새 슬라이스를 만들어야 하고, 그러면 이 단언이 SCHEMA_VERSION
    // bump를 강제한다. 평평한 목록이었다면 현재 버전을 적고 bump를 빠뜨리는 실수를
    // 막을 방법이 없었다.
    assert_eq!(
        LOCKED_ENCRYPTED_COLUMNS.len() as u32,
        db::SCHEMA_VERSION,
        "암호화 대상 컬럼을 추가했다면 SCHEMA_VERSION도 올려야 합니다."
    );
}

#[test]
fn test_nullable_encrypted_columns_skip_empty() {
    // NULL이 들어갈 수 있는 컬럼에서 skip_empty=false면, select_column_sql이
    // `WHERE col != ''`를 붙이지 않아 NULL 행이 조회에 섞이고 fetch_id_text의
    // row.get::<String>()이 타입 변환에서 터진다.
    let conn = fresh_db();
    for c in crate::commands::crypto::ENCRYPTED_COLUMNS {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({})", c.table))
            .unwrap();
        let found = stmt
            .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(3)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .find(|(name, _)| name == c.column);

        let (_, notnull) = found.unwrap_or_else(|| {
            panic!("{}.{}가 schema.sql에 없습니다", c.table, c.column)
        });
        if notnull == 0 {
            assert!(
                c.skip_empty,
                "{}.{}는 nullable이므로 skip_empty가 true여야 합니다",
                c.table, c.column
            );
        }
    }
}
