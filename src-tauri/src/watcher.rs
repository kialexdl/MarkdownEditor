use crate::{
    error::{message, AppResult},
    models::FsEventPayload,
    state::AppState,
    workspace::workspace_by_id,
};
use notify::{Event, RecursiveMode, Watcher};
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

fn ignored(path: &Path) -> bool {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    components.iter().any(|value| {
        matches!(
            value.as_str(),
            "node_modules" | "target" | ".next" | ".svelte-kit" | "dist" | "build"
        )
    }) || components
        .windows(2)
        .any(|window| window[0] == ".git" && window[1] == "objects")
}

fn emit_event(app: &AppHandle, workspace_id: &str, event: Event) {
    let paths = event
        .paths
        .into_iter()
        .filter(|path| !ignored(path))
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return;
    }
    let payload = FsEventPayload {
        workspace_id: workspace_id.to_string(),
        paths,
        kind: format!("{:?}", event.kind),
    };
    let _ = app.emit("workspace-fs-event", payload);
}

#[tauri::command(rename_all = "camelCase")]
pub fn watch_workspace(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let workspace = workspace_by_id(&state, &workspace_id)?;
    let callback_id = workspace_id.clone();
    let callback_app = app.clone();
    let mut watcher = notify::recommended_watcher(move |result| {
        if let Ok(event) = result {
            emit_event(&callback_app, &callback_id, event);
        }
    })?;
    for root in &workspace.roots {
        watcher.watch(Path::new(&root.path), RecursiveMode::Recursive)?;
    }
    state
        .watchers
        .lock()
        .map_err(|_| message("文件监听器状态锁已损坏"))?
        .insert(workspace_id, watcher);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub fn unwatch_workspace(workspace_id: String, state: State<'_, AppState>) -> AppResult<()> {
    state
        .watchers
        .lock()
        .map_err(|_| message("文件监听器状态锁已损坏"))?
        .remove(&workspace_id);
    Ok(())
}
