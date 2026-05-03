use crate::state::{clear_crypto_state, CryptoStateHandle, DbState};
use tauri::State;

pub(crate) fn new_project_impl(
    path: &str,
    state: &DbState,
    crypto: &CryptoStateHandle,
) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.exists() {
        return Err(format!("이미 파일이 존재합니다: {path}"));
    }
    let conn = crate::db::create_new(p).map_err(|e| e.to_string())?;
    *state.0.lock().map_err(|e| e.to_string())? = Some(conn);
    clear_crypto_state(crypto)?;
    Ok(())
}

pub(crate) fn open_project_impl(
    path: &str,
    state: &DbState,
    crypto: &CryptoStateHandle,
) -> Result<(), String> {
    let src = std::path::Path::new(&path);

    if let Some(parent) = src.parent() {
        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("backup");
        let ts = chrono::Local::now().format("%y%m%d-%H%M").to_string();
        let bak_name = format!("{stem}.{ts}.db.backup");
        let _ = std::fs::copy(src, parent.join(bak_name));
    }

    let conn = crate::db::open_existing(src).map_err(|e| e.to_string())?;
    *state.0.lock().map_err(|e| e.to_string())? = Some(conn);
    clear_crypto_state(crypto)?;
    Ok(())
}

#[tauri::command]
pub fn new_project(
    path: String,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<(), String> {
    new_project_impl(&path, &state, &crypto)
}

#[tauri::command]
pub fn open_project(
    path: String,
    state: State<DbState>,
    crypto: State<CryptoStateHandle>,
) -> Result<(), String> {
    open_project_impl(&path, &state, &crypto)
}
