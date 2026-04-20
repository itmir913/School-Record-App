// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::State;

struct DbState(Mutex<Option<Connection>>);

/// 새 DB 파일 생성 (프론트에서 save 다이얼로그로 받은 경로를 넘겨줌)
#[tauri::command]
fn new_project(path: String, state: State<DbState>) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.exists() {
        return Err(format!("이미 파일이 존재합니다: {path}"));
    }
    let conn = db::create_new(p).map_err(|e| e.to_string())?;
    *state.0.lock().unwrap() = Some(conn);
    Ok(())
}

/// 기존 DB 파일 열기 (프론트에서 open 다이얼로그로 받은 경로를 넘겨줌)
#[tauri::command]
fn open_project(path: String, state: State<DbState>) -> Result<(), String> {
    let conn = db::open_existing(std::path::Path::new(&path))
        .map_err(|e| e.to_string())?;
    *state.0.lock().unwrap() = Some(conn);
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DbState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![new_project, open_project])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
