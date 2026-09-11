use crate::{
    error::{message, AppResult},
    models::{WorkspaceRecord, WorkspaceRoot},
    state::AppState,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;
use uuid::Uuid;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn canonical_directory(path: &str) -> AppResult<PathBuf> {
    let path = fs::canonicalize(path)?;
    if !path.is_dir() {
        return Err(message("工作区根路径必须是目录"));
    }
    Ok(path)
}

fn root_from_path(path: &Path) -> WorkspaceRoot {
    WorkspaceRoot {
        path: path.to_string_lossy().to_string(),
        display_name: path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
    }
}

pub fn workspace_by_id(state: &AppState, id: &str) -> AppResult<WorkspaceRecord> {
    state
        .config
        .lock()
        .map_err(|_| message("工作区配置锁已损坏"))?
        .workspaces
        .iter()
        .find(|workspace| workspace.id == id)
        .cloned()
        .ok_or_else(|| message("工作区不存在或已经被移除"))
}

pub fn validate_existing_path(
    state: &AppState,
    workspace_id: &str,
    path: &str,
) -> AppResult<PathBuf> {
    let workspace = workspace_by_id(state, workspace_id)?;
    let canonical = fs::canonicalize(path)?;
    let allowed = workspace.roots.iter().any(|root| {
        fs::canonicalize(&root.path)
            .map(|root_path| canonical.starts_with(root_path))
            .unwrap_or(false)
    });
    if !allowed {
        return Err(message("目标路径不在当前工作区的授权范围内"));
    }
    Ok(canonical)
}

#[tauri::command]
pub fn pick_folder() -> Option<String> {
    rfd::FileDialog::new()
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn list_workspaces(state: State<'_, AppState>) -> AppResult<Vec<WorkspaceRecord>> {
    Ok(state
        .config
        .lock()
        .map_err(|_| message("工作区配置锁已损坏"))?
        .workspaces
        .clone())
}

#[tauri::command(rename_all = "camelCase")]
pub fn add_workspace(path: String, state: State<'_, AppState>) -> AppResult<WorkspaceRecord> {
    let canonical = canonical_directory(&path)?;
    let canonical_string = canonical.to_string_lossy().to_string();
    let mut config = state
        .config
        .lock()
        .map_err(|_| message("工作区配置锁已损坏"))?;

    if let Some(existing) = config.workspaces.iter().find(|workspace| {
        workspace
            .roots
            .iter()
            .any(|root| root.path == canonical_string)
    }) {
        return Ok(existing.clone());
    }

    let root = root_from_path(&canonical);
    let workspace = WorkspaceRecord {
        id: Uuid::new_v4().to_string(),
        name: root.display_name.clone(),
        roots: vec![root],
        updated_at: now_ms(),
    };
    config.workspaces.push(workspace.clone());
    state.persist_config(&config)?;
    Ok(workspace)
}

#[tauri::command(rename_all = "camelCase")]
pub fn remove_workspace(workspace_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let mut config = state
        .config
        .lock()
        .map_err(|_| message("工作区配置锁已损坏"))?;
    config
        .workspaces
        .retain(|workspace| workspace.id != workspace_id);
    state.persist_config(&config)?;
    if let Ok(mut watchers) = state.watchers.lock() {
        watchers.remove(&workspace_id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::root_from_path;
    use std::path::Path;

    #[test]
    fn root_uses_last_path_component_as_name() {
        let root = root_from_path(Path::new("/tmp/example"));
        assert_eq!(root.display_name, "example");
    }
}
