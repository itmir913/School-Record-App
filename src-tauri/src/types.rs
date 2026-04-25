use serde::{Deserialize, Serialize};

// ── Area / Activity 관련 ──────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ActivityItem {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct AreaItem {
    pub id: i64,
    pub name: String,
    pub byte_limit: i64,
    pub activities: Vec<ActivityItem>,
}

#[derive(Serialize, Clone)]
pub struct AreaRef {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct ActivityDetail {
    pub id: i64,
    pub name: String,
    pub areas: Vec<AreaRef>,
    pub record_count: i64,
}

// ── Student 관련 ─────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct StudentItem {
    pub id: i64,
    pub grade: i64,
    pub class_num: i64,
    pub number: i64,
    pub name: String,
}

#[derive(Deserialize)]
pub struct StudentInput {
    pub grade: i64,
    pub class_num: i64,
    pub number: i64,
    pub name: String,
}

#[derive(Serialize)]
pub struct BulkUpsertResult {
    pub inserted: i64,
    pub updated: i64,
}

// ── 치환 규칙 관련 ───────────────────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct ReplaceRule {
    pub id: i64,
    pub old_text: String,
    pub new_text: String,
    pub is_regex: bool,
    pub enabled: bool,
    pub priority: i64,
    pub created_at: String,
    pub updated_at: String,
    pub conflicts: Vec<i64>,
}

#[derive(Serialize)]
pub struct ReplacePreviewItem {
    pub activity_id: i64,
    pub student_id: i64,
    pub activity_name: String,
    pub student_name: String,
    pub original: String,
    pub result: String,
}

#[derive(Serialize)]
pub struct ReplaceApplyResult {
    pub changed_count: i64,
    pub total_count: i64,
}

// ── 기록 그리드 / 히스토리 관련 ─────────────────────────────

#[derive(Serialize, Clone)]
pub struct RecordCell {
    pub activity_id: i64,
    pub student_id: i64,
    pub content: String,
}

#[derive(Serialize, Clone)]
pub struct AreaGridData {
    pub activities: Vec<ActivityItem>,
    pub students: Vec<StudentItem>,
    pub records: Vec<RecordCell>,
}

#[derive(Serialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub content: String,
    pub changed_at: String,
    pub note: Option<String>,
}

// ── 가져오기 관련 ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ImportRecordInput {
    pub grade: i64,
    pub class_num: i64,
    pub number: i64,
    pub name: Option<String>,
    pub activity_id: i64,
    pub content: String,
}

#[derive(Serialize)]
pub struct BulkImportResult {
    pub students_created: i64,
    pub students_updated: i64,
    pub records_saved: i64,
}

#[derive(Serialize)]
pub struct PreviewImportItem {
    pub grade: i64,
    pub class_num: i64,
    pub number: i64,
    pub student_name: String,
    pub activity_id: i64,
    pub activity_name: String,
    pub new_content: String,
    pub existing_content: String,
}

// ── 스냅샷 관련 ──────────────────────────────────────────────

#[derive(Serialize)]
pub struct SnapshotItem {
    pub id: i64,
    pub memo: Option<String>,
    pub created_at: String,
}

// ── 유의어 점검 관련 ─────────────────────────────────────────

#[derive(Serialize)]
pub struct SynonymWordItem {
    pub id: i64,
    pub group_id: i64,
    pub word: String,
}

#[derive(Serialize)]
pub struct SynonymGroupFull {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub items: Vec<SynonymWordItem>,
}

#[derive(Deserialize)]
pub struct SeedGroupInput {
    pub name: String,
    pub words: Vec<String>,
}

#[derive(Serialize)]
pub struct InspectRecord {
    pub id: i64,
    pub activity_name: String,
    pub student_name: String,
    pub area_name: String,
    pub grade: i64,
    pub class_num: i64,
    pub number: i64,
    pub content: String,
}
