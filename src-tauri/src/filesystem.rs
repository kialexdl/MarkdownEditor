use crate::{
    error::{message, AppResult},
    models::{DiskFingerprint, FileEntry, ReadDocumentResult, SaveResult},
    state::AppState,
    workspace::{validate_existing_path, workspace_by_id},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use percent_encoding::percent_decode_str;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;
use tempfile::NamedTempFile;
use url::Url;

const LIVE_PREVIEW_LIMIT: u64 = 2 * 1024 * 1024;
const EDIT_LIMIT: u64 = 10 * 1024 * 1024;
const TEXT_READ_LIMIT: u64 = 32 * 1024 * 1024;
const ASSET_READ_LIMIT: u64 = 20 * 1024 * 1024;

fn modified_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn fingerprint(path: &Path) -> AppResult<DiskFingerprint> {
    let metadata = fs::metadata(path)?;
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(DiskFingerprint {
        modified_ms: modified_ms(&metadata),
        size: metadata.len(),
        sha256: hex_digest(&hasher.finalize()),
    })
}

pub fn document_kind(path: &Path) -> &'static str {
    if path.is_dir() {
        return "unsupported";
    }
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "md" | "markdown" | "mdown" | "mkd" => "markdown",
        "txt" | "json" | "jsonc" | "yaml" | "yml" | "toml" | "xml" | "html" | "svg" | "css"
        | "scss" | "js" | "jsx" | "ts" | "tsx" | "py" | "rs" | "go" | "java" | "kt" | "kts"
        | "c" | "h" | "cpp" | "hpp" | "sh" | "ps1" | "sql" | "ini" | "conf" | "properties"
        | "log" | "csv" | "tsv" => "text",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" => "image",
        "pdf" => "pdf",
        _ => "unsupported",
    }
}

fn excluded_directory(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | ".next" | ".svelte-kit" | "dist" | "build"
    )
}

#[tauri::command(rename_all = "camelCase")]
pub fn list_directory(
    workspace_id: String,
    path: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<FileEntry>> {
    let directory = validate_existing_path(&state, &workspace_id, &path)?;
    if !directory.is_dir() {
        return Err(message("目标不是目录"));
    }
    let mut entries = Vec::new();
    for item in fs::read_dir(directory)? {
        let item = item?;
        let path = item.path();
        let metadata = match item.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let name = item.file_name().to_string_lossy().to_string();
        if metadata.is_dir() && excluded_directory(&name) {
            continue;
        }
        entries.push(FileEntry {
            name,
            path: path.to_string_lossy().to_string(),
            is_directory: metadata.is_dir(),
            kind: document_kind(&path).to_string(),
            size: metadata.len(),
            modified_ms: modified_ms(&metadata),
        });
    }
    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command(rename_all = "camelCase")]
pub fn read_document(
    workspace_id: String,
    path: String,
    state: State<'_, AppState>,
) -> AppResult<ReadDocumentResult> {
    let path = validate_existing_path(&state, &workspace_id, &path)?;
    if !path.is_file() {
        return Err(message("目标不是普通文件"));
    }
    let metadata = fs::metadata(&path)?;
    let size = metadata.len();
    let kind = document_kind(&path).to_string();
    let file_fingerprint = fingerprint(&path)?;
    let mut content = None;
    let mut data_url = None;
    let mut has_bom = false;
    let mut line_ending = "\n".to_string();
    let mut utf8_valid = true;

    if (kind == "markdown" || kind == "text") && size <= TEXT_READ_LIMIT {
        let bytes = fs::read(&path)?;
        let body = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            has_bom = true;
            &bytes[3..]
        } else {
            &bytes[..]
        };
        let text = match String::from_utf8(body.to_vec()) {
            Ok(text) => text,
            Err(error) => {
                utf8_valid = false;
                String::from_utf8_lossy(error.as_bytes()).to_string()
            }
        };
        if text.contains("\r\n") {
            line_ending = "\r\n".to_string();
        }
        content = Some(text);
    } else if kind == "image" && size <= ASSET_READ_LIMIT {
        let bytes = fs::read(&path)?;
        let mime = mime_guess::from_path(&path).first_or_octet_stream();
        data_url = Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)));
    }

    Ok(ReadDocumentResult {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default(),
        kind: kind.clone(),
        editable: kind == "markdown" && utf8_valid && size <= EDIT_LIMIT,
        content,
        data_url,
        size,
        large_file: size > LIVE_PREVIEW_LIMIT,
        has_bom,
        line_ending,
        fingerprint: file_fingerprint,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn create_markdown_file(
    workspace_id: String,
    parent_path: String,
    name: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let parent = validate_existing_path(&state, &workspace_id, &parent_path)?;
    if !parent.is_dir() {
        return Err(message("新文件的父路径不是目录"));
    }
    let candidate = Path::new(&name);
    let extension = candidate
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if name.trim().is_empty()
        || candidate.components().count() != 1
        || !matches!(
            extension.as_deref(),
            Some("md" | "markdown" | "mdown" | "mkd")
        )
    {
        return Err(message("文件名必须是单层路径并使用 Markdown 扩展名"));
    }
    let target = parent.join(candidate);
    if target.exists() {
        return Err(message("同名文件已经存在"));
    }
    File::create(&target)?.sync_all()?;
    Ok(target.to_string_lossy().to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn save_document(
    workspace_id: String,
    path: String,
    content: String,
    expected: DiskFingerprint,
    has_bom: bool,
    force: bool,
    state: State<'_, AppState>,
) -> AppResult<SaveResult> {
    let path = validate_existing_path(&state, &workspace_id, &path)?;
    if document_kind(&path) != "markdown" {
        return Err(message("只有 Markdown 文件允许写入"));
    }
    let current = fingerprint(&path)?;
    if !force && current != expected {
        return Ok(SaveResult {
            status: "conflict".to_string(),
            fingerprint: Some(current),
            message: Some("磁盘文件已被其他程序修改，请比较后再决定".to_string()),
        });
    }

    let parent = path.parent().ok_or_else(|| message("文件缺少父目录"))?;
    let permissions = fs::metadata(&path)?.permissions();
    let mut temp = NamedTempFile::new_in(parent)?;
    if has_bom {
        temp.write_all(&[0xEF, 0xBB, 0xBF])?;
    }
    temp.write_all(content.as_bytes())?;
    temp.as_file_mut().sync_all()?;
    fs::set_permissions(temp.path(), permissions)?;
    temp.persist(&path).map_err(|error| error.error)?;
    let updated = fingerprint(&path)?;
    Ok(SaveResult {
        status: "saved".to_string(),
        fingerprint: Some(updated),
        message: None,
    })
}

fn decode_asset_path(value: &str) -> AppResult<String> {
    let decoded = percent_decode_str(value)
        .decode_utf8()
        .map_err(|_| message("资源路径包含无效编码"))?;
    let path = Path::new(decoded.as_ref());
    if path
        .components()
        .any(|component| matches!(component, Component::Prefix(_)))
    {
        return Err(message("不允许使用带盘符的资源路径"));
    }
    Ok(decoded.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn read_asset(
    workspace_id: String,
    document_path: String,
    asset_path: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    if Url::parse(&asset_path).is_ok() || asset_path.starts_with('#') {
        return Err(message("只允许读取工作区内的相对图片"));
    }
    let document = validate_existing_path(&state, &workspace_id, &document_path)?;
    let decoded = decode_asset_path(&asset_path)?;
    let candidate = if decoded.starts_with('/') || decoded.starts_with('\\') {
        let workspace = workspace_by_id(&state, &workspace_id)?;
        let root = workspace
            .roots
            .first()
            .ok_or_else(|| message("工作区没有可用的目录根"))?;
        PathBuf::from(&root.path).join(decoded.trim_start_matches(['/', '\\']))
    } else {
        document
            .parent()
            .ok_or_else(|| message("文档缺少父目录"))?
            .join(decoded)
    };
    let candidate = validate_existing_path(&state, &workspace_id, &candidate.to_string_lossy())?;
    if document_kind(&candidate) != "image" {
        return Err(message("目标不是受支持的图片"));
    }
    let metadata = fs::metadata(&candidate)?;
    if metadata.len() > ASSET_READ_LIMIT {
        return Err(message("图片超过 20 MiB，未在预览中加载"));
    }
    let mime = mime_guess::from_path(&candidate).first_or_octet_stream();
    Ok(format!(
        "data:{mime};base64,{}",
        STANDARD.encode(fs::read(candidate)?)
    ))
}

#[tauri::command(rename_all = "camelCase")]
pub fn resolve_document_link(
    workspace_id: String,
    document_path: String,
    href: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let document = validate_existing_path(&state, &workspace_id, &document_path)?;
    let path_part = href.split('#').next().unwrap_or_default();
    if path_part.is_empty() {
        return Ok(document.to_string_lossy().to_string());
    }
    if Url::parse(path_part).is_ok() {
        return Err(message("外部链接不能解析为工作区文件"));
    }
    let decoded = decode_asset_path(path_part)?;
    let candidate = if decoded.starts_with('/') || decoded.starts_with('\\') {
        let workspace = workspace_by_id(&state, &workspace_id)?;
        let root = workspace
            .roots
            .first()
            .ok_or_else(|| message("工作区没有可用的目录根"))?;
        PathBuf::from(&root.path).join(decoded.trim_start_matches(['/', '\\']))
    } else {
        document
            .parent()
            .ok_or_else(|| message("文档缺少父目录"))?
            .join(decoded)
    };
    let canonical = validate_existing_path(&state, &workspace_id, &candidate.to_string_lossy())?;
    Ok(canonical.to_string_lossy().to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_external_url(url: String) -> AppResult<()> {
    let parsed = Url::parse(&url).map_err(|_| message("链接格式无效"))?;
    if !matches!(parsed.scheme(), "http" | "https" | "mailto") {
        return Err(message("该链接协议不允许交给系统打开"));
    }
    open::that_detached(url).map_err(|error| message(format!("无法打开链接：{error}")))?;
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_external_path(
    workspace_id: String,
    path: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let path = validate_existing_path(&state, &workspace_id, &path)?;
    open::that_detached(path).map_err(|error| message(format!("无法打开文件：{error}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::document_kind;
    use std::path::Path;

    #[test]
    fn markdown_extensions_are_editable_kind() {
        for name in ["README.md", "a.markdown", "a.mdown", "a.mkd"] {
            assert_eq!(document_kind(Path::new(name)), "markdown");
        }
    }

    #[test]
    fn source_files_are_text_kind() {
        assert_eq!(document_kind(Path::new("main.rs")), "text");
        assert_eq!(document_kind(Path::new("app.ts")), "text");
    }
}
