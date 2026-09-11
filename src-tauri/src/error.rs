use std::io;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("文件操作失败：{0}")]
    Io(#[from] io::Error),
    #[error("配置数据无效：{0}")]
    Json(#[from] serde_json::Error),
    #[error("文件监听失败：{0}")]
    Notify(#[from] notify::Error),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn message(value: impl Into<String>) -> AppError {
    AppError::Message(value.into())
}
