use crate::{
    error::AppResult,
    models::RecoveryEntry,
    state::{atomic_write_json, AppState},
    workspace::validate_existing_path,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn recovery_path(state: &AppState, workspace_id: &str, path: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(workspace_id.as_bytes());
    hasher.update([0]);
    hasher.update(path.as_bytes());
    let name = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    state.recovery_dir.join(format!("{name}.json"))
}

#[tauri::command(rename_all = "camelCase")]
pub fn save_recovery(
    workspace_id: String,
    path: String,
    content: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let canonical = validate_existing_path(&state, &workspace_id, &path)?;
    let entry = RecoveryEntry {
        workspace_id: workspace_id.clone(),
        path: canonical.to_string_lossy().to_string(),
        content,
        saved_at: now_ms(),
    };
    atomic_write_json(&recovery_path(&state, &workspace_id, &entry.path), &entry)
}

#[tauri::command]
pub fn list_recovery(state: State<'_, AppState>) -> AppResult<Vec<RecoveryEntry>> {
    let mut entries = Vec::new();
    for item in fs::read_dir(&state.recovery_dir)? {
        let item = item?;
        if item.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        if let Ok(bytes) = fs::read(item.path()) {
            if let Ok(entry) = serde_json::from_slice::<RecoveryEntry>(&bytes) {
                entries.push(entry);
            }
        }
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.saved_at));
    Ok(entries)
}

#[tauri::command(rename_all = "camelCase")]
pub fn discard_recovery(
    workspace_id: String,
    path: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let target = recovery_path(&state, &workspace_id, &path);
    if target.exists() {
        fs::remove_file(target)?;
    }
    Ok(())
}
