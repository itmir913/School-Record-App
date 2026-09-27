use crate::commands::crypto::resolve_data_key;
use crate::commands::record::insert_history_if_changed_encrypted;
use crate::crypto::{maybe_decrypt, maybe_encrypt};
use crate::db::with_transaction;
use crate::state::{CryptoStateHandle, DbState};
use crate::types::SnapshotItem;
use rusqlite::Connection;
use tauri::State;

/// `memo`는 평문으로 받는다. 저장은 암호화해서 하고, 반환하는 SnapshotItem에는
/// 평문을 그대로 담는다 — 프론트엔드가 이 값을 목록에 바로 꽂기 때문이다.
pub fn create_snapshot_impl(
    conn: &Connection,
    memo: Option<String>,
    key: Option<[u8; 32]>,
) -> Result<SnapshotItem, String> {
    let stored_memo = memo.as_deref().map(|m| maybe_encrypt(m, key)).transpose()?;

    with_transaction(conn, || {
        match key {
            None => {
                conn.execute(
                    "INSERT INTO ActivityRecordHistory (activity_record_id, content, changed_at, note)
                     SELECT r.id, r.content, r.updated_at, NULL
                     FROM ActivityRecord r
                     WHERE NOT EXISTS (
                         SELECT 1 FROM ActivityRecordHistory h
                         WHERE h.id = (SELECT MAX(h2.id) FROM ActivityRecordHistory h2
                                       WHERE h2.activity_record_id = r.id)
                           AND h.content = r.content
                     )",
                    [],
                )
                .map_err(|e| e.to_string())?;
            }
            // 암호문은 nonce가 매번 달라 SQL로는 같은 내용을 가려내지 못한다.
            // 기록마다 복호화해서 비교한다(insert_history_if_changed_encrypted 참고).
            Some(k) => {
                let records: Vec<(i64, String)> = {
                    let mut stmt = conn
                        .prepare("SELECT id, content FROM ActivityRecord")
                        .map_err(|e| e.to_string())?;
                    let rows = stmt
                        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                        .map_err(|e| e.to_string())?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?;
                    rows
                };
                for (record_id, content) in records {
                    insert_history_if_changed_encrypted(conn, record_id, &content, None, k)?;
                }
            }
        }

        conn.execute(
            "INSERT INTO Snapshot (memo) VALUES (?1)",
            rusqlite::params![stored_memo],
        )
        .map_err(|e| e.to_string())?;

        let snapshot_id = conn.last_insert_rowid();
        let created_at: String = conn
            .query_row(
                "SELECT created_at FROM Snapshot WHERE id = ?1",
                rusqlite::params![snapshot_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        Ok(SnapshotItem { id: snapshot_id, memo, created_at })
    })
}

pub fn get_snapshots_impl(
    conn: &Connection,
    key: Option<[u8; 32]>,
) -> Result<Vec<SnapshotItem>, String> {
    let mut stmt = conn
        .prepare("SELECT id, memo, created_at FROM Snapshot ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    // 복호화는 query_map 클로저 밖에서 한다. 그 안은 rusqlite::Error만 돌려줄 수 있어
    // 복호화 실패를 그대로 올릴 수 없다.
    let raw = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut items = Vec::with_capacity(raw.len());
    for (id, memo, created_at) in raw {
        items.push(SnapshotItem {
            id,
            memo: memo.map(|m| maybe_decrypt(m, key)).transpose()?,
            created_at,
        });
    }
    Ok(items)
}

pub fn restore_snapshot_impl(conn: &Connection, snapshot_id: i64) -> Result<i64, String> {
    // 암호화 컬럼(ActivityRecord.content)에 쓰면서 키를 받지 않는 유일한 경로다.
    // 지금은 암호문을 암호문 그대로 옮기므로 표현이 섞이지 않지만, 그건 이 함수가
    // 값을 건드리지 않는 동안에만 참이다. 변환이나 note/memo 쓰기가 여기에 붙는
    // 순간 조용히 가드 밖에서 실행되므로, 다른 쓰기 경로와 같은 선에 세워 둔다.
    crate::commands::crypto::ensure_migrated(conn)?;

    let snapshot_at: String = conn
        .query_row(
            "SELECT created_at FROM Snapshot WHERE id = ?1",
            rusqlite::params![snapshot_id],
            |row| row.get(0),
        )
        .map_err(|_| format!("스냅샷을 찾을 수 없습니다. id={snapshot_id}"))?;

    // 시점 필터는 changed_at, 버전 선택은 id다. 레코드별로 changed_at이 id 순서와
    // 함께 증가하기 때문에 성립하는데, 이는 **시계가 뒤로 가지 않는다는 전제**에
    // 기댄다. PC 시계를 과거로 되돌리면 더 큰 id가 더 이른 시각을 가질 수 있고,
    // 그러면 필터 안에서 id가 가장 큰 행이 스냅샷 이후에 쓴 내용일 수 있다.
    // 되돌린 내용은 히스토리 모달에 그대로 남으므로 영구 손실은 아니다.
    // 아래 `COALESCE(..., '')`의 `''`는 **의도된 값이다(CLAUDE.md).** 스냅샷 시점에
    // 그 기록이 비어 있었다는 뜻이다. 셀 편집은 히스토리를 남기지 않으므로 스냅샷
    // 이후에 입력한 내용은 되찾을 수 없지만, 그것이 스냅샷의 정의다 — 남겨 두면
    // 복원 결과가 그 시점의 상태가 아니게 된다. 감사에서 "데이터 손실"로 반복
    // 보고되는 지점이지만 바꾸지 말 것.
    with_transaction(conn, || {
        let rows = conn
            .execute(
                "UPDATE ActivityRecord SET
                   content = COALESCE(
                     (SELECT h.content
                      FROM ActivityRecordHistory h
                      WHERE h.activity_record_id = ActivityRecord.id
                        AND h.changed_at <= ?1
                      ORDER BY h.id DESC LIMIT 1),
                     ''
                   ),
                   updated_at = datetime('now')",
                rusqlite::params![snapshot_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(rows as i64)
    })
}

#[tauri::command]
pub fn create_snapshot(
    memo: Option<String>,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<SnapshotItem, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    create_snapshot_impl(conn, memo, key)
}

#[tauri::command]
pub fn get_snapshots(
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<Vec<SnapshotItem>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    let key = resolve_data_key(conn, &crypto)?;
    get_snapshots_impl(conn, key)
}

#[tauri::command]
pub fn restore_snapshot(
    snapshot_id: i64,
    state: State<DbState>,
) -> Result<i64, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())?;
    restore_snapshot_impl(conn, snapshot_id)
}
