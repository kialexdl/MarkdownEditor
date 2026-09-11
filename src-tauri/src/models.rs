use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRoot {
    pub path: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub id: String,
    pub name: String,
    pub roots: Vec<WorkspaceRoot>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub workspaces: Vec<WorkspaceRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub kind: String,
    pub size: u64,
    pub modified_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskFingerprint {
    pub modified_ms: u64,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadDocumentResult {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub editable: bool,
    pub content: Option<String>,
    pub data_url: Option<String>,
    pub size: u64,
    pub large_file: bool,
    pub has_bom: bool,
    pub line_ending: String,
    pub fingerprint: DiskFingerprint,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub status: String,
    pub fingerprint: Option<DiskFingerprint>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub available: bool,
    pub repository_root: Option<String>,
    pub branch: Option<String>,
    pub file_status: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHistoryEntry {
    pub commit: String,
    pub short_commit: String,
    pub author: String,
    pub authored_at: String,
    pub subject: String,
    pub path_at_commit: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHistoryPage {
    pub entries: Vec<GitHistoryEntry>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryEntry {
    pub workspace_id: String,
    pub path: String,
    pub content: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsEventPayload {
    pub workspace_id: String,
    pub paths: Vec<String>,
    pub kind: String,
}
