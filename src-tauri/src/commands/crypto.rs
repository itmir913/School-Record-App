use crate::commands::config::{get_config_impl, set_config_impl};
use crate::crypto::{decrypt, derive_key, encrypt, generate_salt, maybe_decrypt, maybe_encrypt};
use crate::state::{
    clear_crypto_state, current_crypto_key, set_crypto_state, CryptoStateHandle, DbPathState,
    DbState,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rusqlite::Connection;
use tauri::State;
use zeroize::Zeroizing;

const VERIFY_PLAINTEXT: &str = "school-record-verify";
const KEY_ENCRYPTION_ENABLED: &str = "encryption_enabled";
const KEY_PBKDF2_SALT: &str = "encryption_pbkdf2_salt";
const KEY_VERIFY_TOKEN: &str = "encryption_verify_token";

#[derive(serde::Serialize)]
pub struct EncryptionStatus {
    pub enabled: bool,
    pub unlocked: bool,
}

#[derive(Clone, Copy)]
enum DataTransform {
    Encrypt,
    Decrypt,
}

struct EncryptedColumn {
    table: &'static str,
    column: &'static str,
    skip_empty: bool,
}

const ENCRYPTED_COLUMNS: &[EncryptedColumn] = &[
    EncryptedColumn {
        table: "Student",
        column: "name",
        skip_empty: false,
    },
    EncryptedColumn {
        table: "ActivityRecord",
        column: "content",
        skip_empty: true,
    },
    EncryptedColumn {
        table: "ActivityRecordHistory",
        column: "content",
        skip_empty: true,
    },
];

fn run_transaction<T>(
    conn: &Connection,
    action: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    match action() {
        Ok(value) => {
            conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            Ok(value)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

fn fetch_id_text(conn: &Connection, sql: &str) -> Result<Vec<(i64, String)>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn select_column_sql(spec: &EncryptedColumn) -> String {
    if spec.skip_empty {
        format!(
            "SELECT id, {} FROM {} WHERE {} != ''",
            spec.column, spec.table, spec.column
        )
    } else {
        format!("SELECT id, {} FROM {}", spec.column, spec.table)
    }
}

fn update_column_sql(spec: &EncryptedColumn) -> String {
    format!("UPDATE {} SET {}=?1 WHERE id=?2", spec.table, spec.column)
}

fn transform_all_data(
    conn: &Connection,
    key: [u8; 32],
    transform: DataTransform,
) -> Result<(), String> {
    for spec in ENCRYPTED_COLUMNS {
        let rows = fetch_id_text(conn, &select_column_sql(spec))?;
        let update_sql = update_column_sql(spec);
        for (id, value) in rows {
            let transformed = match transform {
                DataTransform::Encrypt => maybe_encrypt(&value, Some(key))?,
                DataTransform::Decrypt => maybe_decrypt(value, Some(key))?,
            };
            conn.execute(&update_sql, rusqlite::params![transformed, id])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn encrypt_all_data(conn: &Connection, key: [u8; 32]) -> Result<(), String> {
    transform_all_data(conn, key, DataTransform::Encrypt)
}

pub(crate) fn decrypt_all_data(conn: &Connection, key: [u8; 32]) -> Result<(), String> {
    transform_all_data(conn, key, DataTransform::Decrypt)
}

pub(crate) fn is_encryption_enabled(conn: &Connection) -> Result<bool, String> {
    Ok(get_config_impl(conn, KEY_ENCRYPTION_ENABLED)?.as_deref() == Some("true"))
}

fn encryption_material(conn: &Connection) -> Result<(Vec<u8>, String), String> {
    let salt_b64 = get_config_impl(conn, KEY_PBKDF2_SALT)?.ok_or("암호화 설정이 없습니다.")?;
    let salt = B64
        .decode(&salt_b64)
        .map_err(|e| format!("salt 디코딩 실패: {e}"))?;
    let token = get_config_impl(conn, KEY_VERIFY_TOKEN)?.ok_or("검증 토큰이 없습니다.")?;
    Ok((salt, token))
}

fn verify_password(
    password: &str,
    salt: &[u8],
    verify_token: &str,
    error_message: &str,
) -> Result<[u8; 32], String> {
    let key = derive_key(password, salt);
    let verified = decrypt(verify_token, &key)
        .map(|s| s == VERIFY_PLAINTEXT)
        .unwrap_or(false);
    if verified {
        Ok(key)
    } else {
        Err(error_message.to_string())
    }
}

pub(crate) fn resolve_data_key(
    conn: &Connection,
    crypto: &CryptoStateHandle,
) -> Result<Option<[u8; 32]>, String> {
    if !is_encryption_enabled(conn)? {
        return Ok(None);
    }

    current_crypto_key(crypto)?
        .map(Some)
        .ok_or_else(|| "암호화가 잠금 상태입니다.".to_string())
}

pub(crate) fn get_encryption_status_impl(
    conn: &Connection,
    crypto: &CryptoStateHandle,
) -> Result<EncryptionStatus, String> {
    let enabled = is_encryption_enabled(conn)?;
    let unlocked = enabled && current_crypto_key(crypto)?.is_some();
    Ok(EncryptionStatus { enabled, unlocked })
}

pub(crate) fn unlock_encryption_impl(
    conn: &Connection,
    crypto: &CryptoStateHandle,
    password: &str,
) -> Result<(), String> {
    let (salt, verify_token) = encryption_material(conn)?;
    let key = verify_password(
        password,
        &salt,
        &verify_token,
        "비밀번호가 올바르지 않습니다.",
    )?;
    set_crypto_state(crypto, key, salt)
}

fn backup_db_file(db_path_state: &DbPathState, suffix: &str) -> Result<(), String> {
    let guard = db_path_state.0.lock().map_err(|e| e.to_string())?;
    let src = guard.as_ref().ok_or("열린 프로젝트가 없습니다.")?;
    let parent = src.parent().ok_or("DB 파일의 상위 디렉토리를 찾을 수 없습니다.")?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("backup");
    let ts = chrono::Local::now().format("%y%m%d-%H%M").to_string();
    let bak_name = format!("{stem}.{ts}{suffix}.db.backup");
    std::fs::copy(src, parent.join(bak_name)).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn enable_encryption_impl(
    conn: &Connection,
    crypto: &CryptoStateHandle,
    db_path_state: &DbPathState,
    password: &str,
) -> Result<(), String> {
    if is_encryption_enabled(conn)? {
        return Err("이미 암호화가 활성화되어 있습니다.".to_string());
    }

    backup_db_file(db_path_state, "-pre-encrypt")?;

    let salt = generate_salt();
    let key = derive_key(password, &salt);
    let salt_b64 = B64.encode(salt);
    let verify_token = encrypt(VERIFY_PLAINTEXT, &key)?;

    run_transaction(conn, || {
        encrypt_all_data(conn, key)?;
        set_config_impl(conn, KEY_PBKDF2_SALT, &salt_b64)?;
        set_config_impl(conn, KEY_VERIFY_TOKEN, &verify_token)?;
        set_config_impl(conn, KEY_ENCRYPTION_ENABLED, "true")?;
        Ok(())
    })?;

    set_crypto_state(crypto, key, salt.to_vec())
}

pub(crate) fn disable_encryption_impl(
    conn: &Connection,
    crypto: &CryptoStateHandle,
    db_path_state: &DbPathState,
) -> Result<(), String> {
    let key = resolve_data_key(conn, crypto)?.ok_or("암호화가 활성화되어 있지 않습니다.")?;

    backup_db_file(db_path_state, "-pre-decrypt")?;

    run_transaction(conn, || {
        decrypt_all_data(conn, key)?;
        conn.execute(
            "DELETE FROM APP_CONFIGS WHERE config_key IN (?1, ?2, ?3)",
            rusqlite::params![KEY_ENCRYPTION_ENABLED, KEY_PBKDF2_SALT, KEY_VERIFY_TOKEN],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;

    clear_crypto_state(crypto)
}

pub(crate) fn change_encryption_password_impl(
    conn: &Connection,
    crypto: &CryptoStateHandle,
    db_path_state: &DbPathState,
    old_password: &str,
    new_password: &str,
) -> Result<(), String> {
    let (salt, verify_token) = encryption_material(conn)?;
    let old_key = verify_password(
        old_password,
        &salt,
        &verify_token,
        "현재 비밀번호가 올바르지 않습니다.",
    )?;

    backup_db_file(db_path_state, "-pre-reencrypt")?;

    let new_salt = generate_salt();
    let new_key = derive_key(new_password, &new_salt);
    let new_salt_b64 = B64.encode(new_salt);
    let new_verify_token = encrypt(VERIFY_PLAINTEXT, &new_key)?;

    run_transaction(conn, || {
        decrypt_all_data(conn, old_key)?;
        encrypt_all_data(conn, new_key)?;
        set_config_impl(conn, KEY_PBKDF2_SALT, &new_salt_b64)?;
        set_config_impl(conn, KEY_VERIFY_TOKEN, &new_verify_token)?;
        Ok(())
    })?;

    set_crypto_state(crypto, new_key, new_salt.to_vec())
}

fn db_conn<'a>(guard: &'a Option<Connection>) -> Result<&'a Connection, String> {
    guard
        .as_ref()
        .ok_or_else(|| "DB가 열려있지 않습니다.".to_string())
}

#[tauri::command]
pub fn get_encryption_status(
    db: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<EncryptionStatus, String> {
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    get_encryption_status_impl(db_conn(&guard)?, &crypto)
}

#[tauri::command]
pub fn unlock_encryption(
    password: String,
    db: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<(), String> {
    let password = Zeroizing::new(password);
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    unlock_encryption_impl(db_conn(&guard)?, &crypto, &password)
}

#[tauri::command]
pub fn enable_encryption(
    password: String,
    db: State<DbState>,
    db_path: State<DbPathState>,
    crypto: State<CryptoStateHandle>,
) -> Result<(), String> {
    let password = Zeroizing::new(password);
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    enable_encryption_impl(db_conn(&guard)?, &crypto, &db_path, &password)
}

#[tauri::command]
pub fn disable_encryption(
    db: State<DbState>,
    db_path: State<DbPathState>,
    crypto: State<CryptoStateHandle>,
) -> Result<(), String> {
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    disable_encryption_impl(db_conn(&guard)?, &crypto, &db_path)
}

#[tauri::command]
pub fn change_encryption_password(
    old_password: String,
    new_password: String,
    db: State<DbState>,
    db_path: State<DbPathState>,
    crypto: State<CryptoStateHandle>,
) -> Result<(), String> {
    let old_password = Zeroizing::new(old_password);
    let new_password = Zeroizing::new(new_password);
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    change_encryption_password_impl(db_conn(&guard)?, &crypto, &db_path, &old_password, &new_password)
}
