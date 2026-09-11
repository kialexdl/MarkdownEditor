mod error;
mod filesystem;
mod git;
mod models;
mod recovery;
mod state;
mod watcher;
mod workspace;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::new(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            workspace::pick_folder,
            workspace::list_workspaces,
            workspace::add_workspace,
            workspace::remove_workspace,
            filesystem::list_directory,
            filesystem::read_document,
            filesystem::create_markdown_file,
            filesystem::save_document,
            filesystem::read_asset,
            filesystem::resolve_document_link,
            filesystem::open_external_url,
            filesystem::open_external_path,
            watcher::watch_workspace,
            watcher::unwatch_workspace,
            git::check_git,
            git::get_git_info,
            git::list_git_history,
            git::read_git_revision,
            recovery::save_recovery,
            recovery::list_recovery,
            recovery::discard_recovery,
        ])
        .run(tauri::generate_context!())
        .expect("MarkdownEditor failed to start");
}
