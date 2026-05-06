use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub struct DbState(pub Mutex<Option<Connection>>);
pub struct DbPathState(pub Mutex<Option<PathBuf>>);

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct CryptoState {
    pub key: Option<[u8; 32]>,
    pub salt: Option<Vec<u8>>,
}

pub type CryptoStateHandle = Mutex<CryptoState>;

pub fn current_crypto_key(crypto: &CryptoStateHandle) -> Result<Option<[u8; 32]>, String> {
    let guard = crypto.lock().map_err(|e| e.to_string())?;
    Ok(guard.key)
}

pub fn set_crypto_state(
    crypto: &CryptoStateHandle,
    key: [u8; 32],
    salt: Vec<u8>,
) -> Result<(), String> {
    let mut guard = crypto.lock().map_err(|e| e.to_string())?;
    guard.key = Some(key);
    guard.salt = Some(salt);
    Ok(())
}

pub fn clear_crypto_state(crypto: &CryptoStateHandle) -> Result<(), String> {
    let mut guard = crypto.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut k) = guard.key { k.zeroize(); }
    if let Some(ref mut s) = guard.salt { s.zeroize(); }
    guard.key = None;
    guard.salt = None;
    Ok(())
}

pub struct ReplaceCache {
    pub ruleset_version: u64,
    pub entries: HashMap<u64, (String, u64)>,
}

pub type ReplaceCacheState = Mutex<ReplaceCache>;

pub fn unique_err(e: &rusqlite::Error, conflict_msg: &str) -> String {
    if e.to_string().contains("UNIQUE constraint failed") {
        conflict_msg.to_string()
    } else {
        e.to_string()
    }
}
