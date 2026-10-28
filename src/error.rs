use std::fmt;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    NotFound,
    Conflict,
    InvalidState,
    InvalidInput,
    Git,
    Io,
    Internal,
}

/// Error type shared by the Git layer and the Tauri commands; it reaches the
/// frontend as `{ kind, message }` when a command rejects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidState, message)
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidInput, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl From<git2::Error> for AppError {
    fn from(error: git2::Error) -> Self {
        let kind = match error.code() {
            git2::ErrorCode::NotFound => ErrorKind::NotFound,
            git2::ErrorCode::Conflict => ErrorKind::Conflict,
            _ => ErrorKind::Git,
        };
        Self::new(kind, error.message())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorKind::Io, error.to_string())
    }
}
