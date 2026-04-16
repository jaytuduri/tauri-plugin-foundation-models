use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("session {0} not found")]
    SessionNotFound(u64),
    /// The session's context window was exceeded. Start a new session.
    #[error("context window exceeded")]
    ContextWindowExceeded,
    #[error("unsupported language or locale")]
    UnsupportedLanguageOrLocale,
    #[error("FoundationModels error: {0}")]
    Native(String),
    // ── ImagePlayground errors ───────────────────────────────────────────
    #[error("image generation is not supported on this device")]
    ImageNotSupported,
    #[error("image generation requires the app to be in the foreground")]
    ImageBackgroundCreationForbidden,
    #[error("image creation failed")]
    ImageCreationFailed,
    #[error("image creation was cancelled")]
    ImageCreationCancelled,
    #[error("face in source image is too small")]
    ImageFaceInImageTooSmall,
    #[error("image generation does not support this language")]
    ImageUnsupportedLanguage,
    #[error("image generation does not support this input image")]
    ImageUnsupportedInputImage,
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
