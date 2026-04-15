use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use once_cell::sync::Lazy;
use tokio::sync::{mpsc, oneshot};

/// Completion slot: passes the final text (or error) back to an awaiting task.
pub static PENDING_COMPLETIONS: Lazy<Mutex<HashMap<u64, oneshot::Sender<CompletionPayload>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Streaming slot: token sink plus a final completion sink.
pub static PENDING_STREAMS: Lazy<Mutex<HashMap<u64, StreamSink>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Image generation slot: one JSON string per image, plus a final completion sink.
pub static PENDING_IMG_GEN: Lazy<Mutex<HashMap<u64, StreamSink>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct StreamSink {
    pub tokens: mpsc::UnboundedSender<String>,
    pub done: oneshot::Sender<CompletionPayload>,
}

#[derive(Debug, Clone)]
pub struct CompletionPayload {
    pub ok: bool,
    pub text: String,
}

static NEXT_CTX: AtomicU64 = AtomicU64::new(1);

pub fn next_ctx_id() -> u64 {
    NEXT_CTX.fetch_add(1, Ordering::Relaxed)
}
