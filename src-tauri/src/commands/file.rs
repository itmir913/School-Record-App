use crate::engine::validate_parent_dir_path;

#[tauri::command]
pub fn write_bytes_file(path: String, data: String) -> Result<(), String> {
    use base64::Engine;
    validate_parent_dir_path(&path, "저장 위치의 디렉토리가 존재하지 않습니다.")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())
}
