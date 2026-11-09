use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub conversation_id: String,
    pub title: String,
    pub step_count: i64,
    pub last_modified_time: String,
    pub workspace_uris: String,
    pub project_id: String,
    pub status: String,
    pub last_user_input_time: String,
    pub last_user_input_step_index: String,
}

#[derive(Debug, Clone, Default)]
pub struct DiskInfo {
    pub db_bytes: u64,
    pub brain_bytes: u64,
    pub brain_files: usize,
    pub annot_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveManifest {
    pub format: String,
    pub version: u32,
    pub conversation_id: String,
    pub title: String,
    pub step_count: i64,
    pub workspace_uris: Vec<String>,
    pub project_id: String,
    pub source_home: String,
    pub source_user: String,
    pub last_modified_time: String,
    pub exported_at: String,
}

#[derive(Debug, Clone)]
pub struct LiveLsInfo {
    pub port: u16,
    pub csrf_token: String,
}
