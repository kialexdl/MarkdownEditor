use crate::{
    error::{message, AppResult},
    models::AppConfig,
};
use notify::RecommendedWatcher;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Manager};
use tempfile::NamedTempFile;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub config_path: PathBuf,
    pub recovery_dir: PathBuf,
    pub watchers: Mutex<HashMap<String, RecommendedWatcher>>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> AppResult<Self> {
        let base = app
            .path()
            .app_config_dir()
            .map_err(|error| message(format!("无法定位应用配置目录：{error}")))?;
        fs::create_dir_all(&base)?;
        let recovery_dir = base.join("recovery");
        fs::create_dir_all(&recovery_dir)?;
        let config_path = base.join("config.json");
        let config = if config_path.exists() {
            serde_json::from_slice(&fs::read(&config_path)?)?
        } else {
            AppConfig::default()
        };

        Ok(Self {
            config: Mutex::new(config),
            config_path,
            recovery_dir,
            watchers: Mutex::new(HashMap::new()),
        })
    }

    pub fn persist_config(&self, config: &AppConfig) -> AppResult<()> {
        atomic_write_json(&self.config_path, config)
    }
}

pub fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| message("配置文件缺少父目录"))?;
    fs::create_dir_all(parent)?;
    let mut temp = NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temp, value)?;
    temp.as_file_mut().sync_all()?;
    temp.persist(path).map_err(|error| error.error)?;
    Ok(())
}
