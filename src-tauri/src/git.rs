use crate::{
    error::{message, AppResult},
    models::{GitHistoryEntry, GitHistoryPage, GitInfo},
    state::AppState,
    workspace::validate_existing_path,
};
use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use tauri::State;
use tempfile::NamedTempFile;
use wait_timeout::ChildExt;

const GIT_TIMEOUT: Duration = Duration::from_secs(15);
const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
const COMMIT_MARKER: &str = "__MARKDOWN_EDITOR_COMMIT__";

#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn hide_console(_: &mut Command) {}

fn run_git(cwd: Option<&Path>, args: &[OsString]) -> AppResult<String> {
    let mut stdout_file = NamedTempFile::new()?;
    let mut stderr_file = NamedTempFile::new()?;
    let mut command = Command::new("git");
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file.reopen()?))
        .stderr(Stdio::from(stderr_file.reopen()?));
    hide_console(&mut command);
    let mut child = command.spawn().map_err(|error| {
        message(format!(
            "无法启动 Git，请确认 Git 已安装并加入 PATH：{error}"
        ))
    })?;
    let status = match child.wait_timeout(GIT_TIMEOUT)? {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(message("Git 命令执行超时"));
        }
    };
    stdout_file.flush()?;
    stderr_file.flush()?;
    let stdout = fs::read(stdout_file.path())?;
    let stderr = fs::read(stderr_file.path())?;
    if stdout.len() > OUTPUT_LIMIT || stderr.len() > OUTPUT_LIMIT {
        return Err(message("Git 输出超过 8 MiB 安全上限"));
    }
    if !status.success() {
        let detail = String::from_utf8_lossy(&stderr).trim().to_string();
        return Err(message(if detail.is_empty() {
            "Git 命令执行失败".to_string()
        } else {
            detail
        }));
    }
    Ok(String::from_utf8_lossy(&stdout).to_string())
}

fn os_args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn discover_repository(file: &Path) -> AppResult<PathBuf> {
    let directory = file.parent().ok_or_else(|| message("文件缺少父目录"))?;
    let output = run_git(Some(directory), &os_args(&["rev-parse", "--show-toplevel"]))?;
    let root = fs::canonicalize(output.trim())?;
    Ok(root)
}

fn relative_git_path(repository: &Path, file: &Path) -> AppResult<String> {
    let relative = file
        .strip_prefix(repository)
        .map_err(|_| message("文件不属于发现到的 Git 仓库"))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

#[tauri::command]
pub fn check_git() -> AppResult<String> {
    Ok(run_git(None, &os_args(&["--version"]))?.trim().to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_git_info(
    workspace_id: String,
    path: String,
    state: State<'_, AppState>,
) -> AppResult<GitInfo> {
    let file = validate_existing_path(&state, &workspace_id, &path)?;
    let repository = match discover_repository(&file) {
        Ok(repository) => repository,
        Err(error) => {
            return Ok(GitInfo {
                available: false,
                repository_root: None,
                branch: None,
                file_status: None,
                message: Some(error.to_string()),
            })
        }
    };
    let relative = relative_git_path(&repository, &file)?;
    let branch = run_git(
        Some(&repository),
        &os_args(&["symbolic-ref", "--short", "-q", "HEAD"]),
    )
    .or_else(|_| {
        run_git(
            Some(&repository),
            &os_args(&["rev-parse", "--short", "HEAD"]),
        )
    })
    .ok()
    .map(|value| value.trim().to_string());
    let mut status_args = os_args(&["status", "--porcelain=v1", "--untracked-files=all", "--"]);
    status_args.push(relative.clone().into());
    let status = run_git(Some(&repository), &status_args)
        .unwrap_or_default()
        .trim()
        .to_string();
    Ok(GitInfo {
        available: true,
        repository_root: Some(repository.to_string_lossy().to_string()),
        branch,
        file_status: if status.is_empty() {
            None
        } else {
            Some(status)
        },
        message: None,
    })
}

#[derive(Debug)]
struct PendingCommit {
    commit: String,
    author: String,
    authored_at: String,
    subject: String,
    older_path: Option<String>,
}

fn push_pending(
    pending: Option<PendingCommit>,
    current_path: &mut String,
    entries: &mut Vec<GitHistoryEntry>,
) {
    if let Some(pending) = pending {
        let short_commit = pending.commit.chars().take(8).collect();
        entries.push(GitHistoryEntry {
            commit: pending.commit,
            short_commit,
            author: pending.author,
            authored_at: pending.authored_at,
            subject: pending.subject,
            path_at_commit: current_path.clone(),
        });
        if let Some(older_path) = pending.older_path {
            *current_path = older_path;
        }
    }
}

fn parse_history(output: &str, starting_path: &str) -> Vec<GitHistoryEntry> {
    let mut entries = Vec::new();
    let mut pending: Option<PendingCommit> = None;
    let mut current_path = starting_path.to_string();

    for line in output.lines() {
        if let Some(data) = line.strip_prefix(COMMIT_MARKER) {
            push_pending(pending.take(), &mut current_path, &mut entries);
            let mut fields = data.trim_start_matches('\u{1f}').splitn(4, '\u{1f}');
            pending = Some(PendingCommit {
                commit: fields.next().unwrap_or_default().to_string(),
                author: fields.next().unwrap_or_default().to_string(),
                authored_at: fields.next().unwrap_or_default().to_string(),
                subject: fields.next().unwrap_or_default().to_string(),
                older_path: None,
            });
            continue;
        }

        if let Some(commit) = pending.as_mut() {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() >= 3 && fields[0].starts_with('R') && fields[2] == current_path {
                commit.older_path = Some(fields[1].to_string());
            }
        }
    }
    push_pending(pending, &mut current_path, &mut entries);
    entries
}

#[tauri::command(rename_all = "camelCase")]
pub fn list_git_history(
    workspace_id: String,
    path: String,
    offset: usize,
    limit: usize,
    state: State<'_, AppState>,
) -> AppResult<GitHistoryPage> {
    let file = validate_existing_path(&state, &workspace_id, &path)?;
    let repository = discover_repository(&file)?;
    let relative = relative_git_path(&repository, &file)?;
    let requested = offset.saturating_add(limit.min(200)).saturating_add(1);
    let format = format!("--format={COMMIT_MARKER}%x1f%H%x1f%an%x1f%aI%x1f%s");
    let args = vec![
        OsString::from("log"),
        OsString::from("--follow"),
        OsString::from("--name-status"),
        OsString::from("-M"),
        OsString::from(format!("-n{requested}")),
        OsString::from(format),
        OsString::from("--"),
        OsString::from(relative.clone()),
    ];
    let output = run_git(Some(&repository), &args)?;
    let parsed = parse_history(&output, &relative);
    let has_more = parsed.len() > offset + limit;
    let entries = parsed.into_iter().skip(offset).take(limit).collect();
    Ok(GitHistoryPage { entries, has_more })
}

fn validate_revision(value: &str) -> AppResult<()> {
    if !(7..=64).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(message("Git revision 格式无效"));
    }
    Ok(())
}

fn validate_relative_git_path(value: &str) -> AppResult<()> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(message("历史文件路径无效"));
    }
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub fn read_git_revision(
    workspace_id: String,
    path: String,
    revision: String,
    path_at_commit: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    validate_revision(&revision)?;
    validate_relative_git_path(&path_at_commit)?;
    let file = validate_existing_path(&state, &workspace_id, &path)?;
    let repository = discover_repository(&file)?;
    let object = format!("{revision}:{path_at_commit}");
    run_git(
        Some(&repository),
        &[OsString::from("show"), OsString::from(object)],
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_history, validate_relative_git_path, validate_revision};

    #[test]
    fn parses_rename_history_paths() {
        let input = "__MARKDOWN_EDITOR_COMMIT__\x1faabbccdd\x1fA\x1f2026-01-01T00:00:00Z\x1frename\nR100\told.md\tnew.md\n__MARKDOWN_EDITOR_COMMIT__\x1f11223344\x1fB\x1f2025-01-01T00:00:00Z\x1fold\nM\told.md";
        let history = parse_history(input, "new.md");
        assert_eq!(history[0].path_at_commit, "new.md");
        assert_eq!(history[1].path_at_commit, "old.md");
    }

    #[test]
    fn rejects_unsafe_revision_inputs() {
        assert!(validate_revision("HEAD;rm").is_err());
        assert!(validate_relative_git_path("../secret.md").is_err());
    }
}
