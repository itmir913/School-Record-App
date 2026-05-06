use crate::commands::project::{new_project_impl, open_project_impl};
use crate::state::{current_crypto_key, CryptoState, CryptoStateHandle, DbPathState, DbState};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

fn crypto_state_with_key() -> CryptoStateHandle {
    Mutex::new(CryptoState {
        key: Some([7u8; 32]),
        salt: Some(vec![1, 2, 3]),
    })
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "school_record_app_{label}_{}_{}",
        std::process::id(),
        nanos
    ))
}

#[test]
fn test_new_project_clears_crypto_state() {
    let dir = unique_temp_dir("new_project");
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("new.db");
    let db = DbState(Mutex::new(None));
    let db_path = DbPathState(Mutex::new(None));
    let crypto = crypto_state_with_key();

    new_project_impl(path.to_str().unwrap(), &db, &db_path, &crypto).unwrap();

    assert!(db.0.lock().unwrap().is_some());
    assert!(current_crypto_key(&crypto).unwrap().is_none());

    drop(db); // Windows: 파일 잠금 해제 후 삭제
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_open_project_clears_crypto_state() {
    let dir = unique_temp_dir("open_project");
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("existing.db");
    drop(crate::db::create_new(&path).unwrap());

    let db = DbState(Mutex::new(None));
    let db_path = DbPathState(Mutex::new(None));
    let crypto = crypto_state_with_key();

    open_project_impl(path.to_str().unwrap(), &db, &db_path, &crypto).unwrap();

    assert!(db.0.lock().unwrap().is_some());
    assert!(current_crypto_key(&crypto).unwrap().is_none());

    drop(db); // Windows: 파일 잠금 해제 후 삭제
    std::fs::remove_dir_all(&dir).unwrap();
}
