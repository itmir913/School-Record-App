// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod crypto;
mod db;
mod engine;
mod state;
mod types;
#[cfg(test)]
mod tests;

use commands::*;
use state::{CryptoState, CryptoStateHandle, DbPathState, DbState, ReplaceCache};
use std::collections::HashMap;
use std::sync::Mutex;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(DbState(Mutex::new(None)))
        .manage(DbPathState(Mutex::new(None)))
        .manage(Mutex::new(ReplaceCache {
            ruleset_version: 0,
            entries: HashMap::new(),
        }))
        .manage(CryptoStateHandle::new(CryptoState { key: None, salt: None }))
        .invoke_handler(tauri::generate_handler![
            new_project,
            open_project,
            get_areas,
            create_area,
            update_area,
            delete_area,
            get_activities,
            set_area_activities,
            create_activity,
            update_activity,
            delete_activity,
            set_activity_areas,
            get_students,
            create_student,
            update_student,
            delete_student,
            bulk_upsert_students,
            get_area_students,
            set_area_students,
            get_area_grid,
            upsert_record,
            get_record_history,
            save_history_snapshot,
            bulk_import_records,
            preview_import_records,
            write_bytes_file,
            create_snapshot,
            get_snapshots,
            restore_snapshot,
            get_replace_rules,
            create_replace_rule,
            update_replace_rule,
            delete_replace_rule,
            seed_default_replace_rules,
            preview_replace,
            apply_replace,
            get_synonym_groups,
            create_synonym_group,
            delete_synonym_group,
            add_synonym_word,
            delete_synonym_word,
            seed_default_synonyms,
            get_all_records_for_inspect,
            get_config,
            set_config,
            get_encryption_status,
            unlock_encryption,
            enable_encryption,
            disable_encryption,
            change_encryption_password,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
