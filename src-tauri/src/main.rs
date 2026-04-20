// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;

fn main() {
    let db_path = std::path::PathBuf::from("school_record.db");
    let _conn = db::open(&db_path).expect("DB 초기화 실패");

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    school_record_app_lib::run()
}
