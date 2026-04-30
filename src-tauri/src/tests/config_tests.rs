use crate::commands::config::{get_config_impl, set_config_impl};
use super::setup_test_db;

#[test]
fn test_get_config_missing_key_returns_none() {
    let conn = setup_test_db();
    let result = get_config_impl(&conn, "nonexistent_key").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_set_then_get_config() {
    let conn = setup_test_db();
    set_config_impl(&conn, "record_section_cell_text_size", "16").unwrap();
    let result = get_config_impl(&conn, "record_section_cell_text_size").unwrap();
    assert_eq!(result, Some("16".to_string()));
}

#[test]
fn test_set_config_overwrites_existing_value() {
    let conn = setup_test_db();
    set_config_impl(&conn, "record_section_cell_text_size", "14").unwrap();
    set_config_impl(&conn, "record_section_cell_text_size", "20").unwrap();
    let result = get_config_impl(&conn, "record_section_cell_text_size").unwrap();
    assert_eq!(result, Some("20".to_string()));
}

#[test]
fn test_multiple_keys_are_independent() {
    let conn = setup_test_db();
    set_config_impl(&conn, "key_a", "value_a").unwrap();
    set_config_impl(&conn, "key_b", "value_b").unwrap();
    assert_eq!(get_config_impl(&conn, "key_a").unwrap(), Some("value_a".to_string()));
    assert_eq!(get_config_impl(&conn, "key_b").unwrap(), Some("value_b".to_string()));
}

#[test]
fn test_set_config_empty_string_value() {
    let conn = setup_test_db();
    set_config_impl(&conn, "some_key", "").unwrap();
    let result = get_config_impl(&conn, "some_key").unwrap();
    assert_eq!(result, Some("".to_string()));
}

#[test]
fn test_get_config_returns_latest_after_multiple_sets() {
    let conn = setup_test_db();
    for val in ["10", "12", "14", "18", "22"] {
        set_config_impl(&conn, "record_section_cell_text_size", val).unwrap();
    }
    let result = get_config_impl(&conn, "record_section_cell_text_size").unwrap();
    assert_eq!(result, Some("22".to_string()));
}
