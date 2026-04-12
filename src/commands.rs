use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{command, AppHandle, Runtime};
use tokio::sync::{mpsc, oneshot};

use crate::error::{Error, Result};
use crate::ffi;
use crate::session::{next_ctx_id, CompletionPayload, StreamSink, PENDING_COMPLETIONS, PENDING_STREAMS};

/// Tauri event name emitted when the model invokes a tool.
/// Must match the listener in guest-js/index.ts.
pub const TOOL_CALL_EVENT: &str = "foundation-models://tool-call";

// ── Input types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationOptions {
    pub temperature: Option<f64>,
    pub maximum_response_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    pub instructions: Option<String>,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityStatus {
    pub available: bool,
    pub reason: Option<String>,
}

// ── C-string helpers ─────────────────────────────────────────────────────

fn to_cstring(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| Error::InvalidInput("string contains NUL".into()))
}

/// Takes ownership of a Swift-allocated string and frees it.
unsafe fn take_cstring(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    ffi::ai_free_string(ptr);
    Some(s)
}

/// Borrows a C string without freeing it. Returns empty string if null.
unsafe fn read_cstr(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

/// Converts a native error string from the Swift bridge into a typed Error.
// Maps well-known error strings from Bridge.swift to typed variants.
// String literals here must stay in sync with errorMessage() in Bridge.swift.
fn map_native_error(msg: String) -> Error {
    match msg.as_str() {
        "exceededContextWindowSize" => Error::ContextWindowExceeded,
        "unsupportedLanguageOrLocale" => Error::UnsupportedLanguageOrLocale,
        _ => Error::Native(msg),
    }
}

// ── Commands ─────────────────────────────────────────────────────────────

#[command]
pub async fn availability() -> Result<AvailabilityStatus> {
    let raw = unsafe { ffi::ai_availability() };
    let json = unsafe { take_cstring(raw) }
        .ok_or_else(|| Error::Native("ai_availability returned null".into()))?;
    Ok(serde_json::from_str(&json)?)
}

#[command]
pub async fn create_session(config: SessionConfig) -> Result<u64> {
    let json = serde_json::to_string(&serde_json::json!({
        "instructions": config.instructions,
        "tools": config.tools.iter().map(|t| serde_json::json!({
            "name": t.name,
            "description": t.description,
            "parametersSchema": t.parameters_schema,
        })).collect::<Vec<_>>(),
    }))?;
    let c_json = to_cstring(&json)?;
    let mut session_id: u64 = 0;
    let mut err_ptr: *mut c_char = ptr::null_mut();
    let status = unsafe {
        ffi::ai_create_session(c_json.as_ptr(), &mut session_id, &mut err_ptr)
    };
    if status != 0 {
        let msg = unsafe { take_cstring(err_ptr) }.unwrap_or_else(|| "unknown".into());
        return Err(Error::Native(msg));
    }
    Ok(session_id)
}

#[command]
pub async fn close_session(session_id: u64) -> Result<()> {
    let status = unsafe { ffi::ai_close_session(session_id) };
    if status != 0 {
        return Err(Error::SessionNotFound(session_id));
    }
    Ok(())
}

/// Creates a throwaway session, runs `f`, then closes it regardless of outcome.
async fn with_ephemeral_session<F, Fut, T>(f: F) -> Result<T>
where
    F: FnOnce(u64) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let session_id = create_session(SessionConfig::default()).await?;
    let result = f(session_id).await;
    let _ = close_session(session_id).await;
    result
}

// ── One-shot respond ─────────────────────────────────────────────────────

extern "C" fn completion_trampoline(ctx: *mut c_void, status: c_int, payload: *const c_char) {
    let ctx_id = ctx as u64;
    let text = unsafe { read_cstr(payload) };
    if let Some(tx) = PENDING_COMPLETIONS.lock().unwrap().remove(&ctx_id) {
        let _ = tx.send(CompletionPayload { ok: status == 0, text });
    }
}

async fn respond_inner(
    session_id: u64,
    prompt: String,
    options: Option<GenerationOptions>,
) -> Result<String> {
    let opts_json = serde_json::to_string(&options.unwrap_or_default())?;
    let c_prompt = to_cstring(&prompt)?;
    let c_opts = to_cstring(&opts_json)?;
    let (tx, rx) = oneshot::channel::<CompletionPayload>();
    let ctx_id = next_ctx_id();
    PENDING_COMPLETIONS.lock().unwrap().insert(ctx_id, tx);
    let status = unsafe {
        ffi::ai_respond(
            session_id,
            c_prompt.as_ptr(),
            c_opts.as_ptr(),
            ctx_id as *mut c_void,
            completion_trampoline,
        )
    };
    if status != 0 {
        PENDING_COMPLETIONS.lock().unwrap().remove(&ctx_id);
        return Err(Error::Native(format!("ai_respond returned {status}")));
    }
    let payload = rx.await.map_err(|_| Error::Native("completion channel dropped".into()))?;
    if payload.ok { Ok(payload.text) } else { Err(map_native_error(payload.text)) }
}

#[command]
pub async fn generate(prompt: String, options: Option<GenerationOptions>) -> Result<String> {
    with_ephemeral_session(|id| respond_inner(id, prompt, options)).await
}

#[command]
pub async fn respond(
    session_id: u64,
    prompt: String,
    options: Option<GenerationOptions>,
) -> Result<String> {
    respond_inner(session_id, prompt, options).await
}

// ── Streaming ────────────────────────────────────────────────────────────

extern "C" fn token_trampoline(ctx: *mut c_void, chunk: *const c_char) {
    let ctx_id = ctx as u64;
    let text = unsafe { read_cstr(chunk) };
    if text.is_empty() { return; }
    if let Some(sink) = PENDING_STREAMS.lock().unwrap().get(&ctx_id) {
        let _ = sink.tokens.send(text);
    }
}

extern "C" fn stream_completion_trampoline(ctx: *mut c_void, status: c_int, payload: *const c_char) {
    let ctx_id = ctx as u64;
    let text = unsafe { read_cstr(payload) };
    if let Some(sink) = PENDING_STREAMS.lock().unwrap().remove(&ctx_id) {
        let _ = sink.done.send(CompletionPayload { ok: status == 0, text });
    }
}

async fn respond_stream_inner(
    session_id: u64,
    prompt: String,
    options: Option<GenerationOptions>,
    channel: Channel<String>,
) -> Result<String> {
    let opts_json = serde_json::to_string(&options.unwrap_or_default())?;
    let c_prompt = to_cstring(&prompt)?;
    let c_opts = to_cstring(&opts_json)?;

    let (tok_tx, mut tok_rx) = mpsc::unbounded_channel::<String>();
    let (done_tx, done_rx) = oneshot::channel::<CompletionPayload>();
    let ctx_id = next_ctx_id();
    PENDING_STREAMS.lock().unwrap().insert(ctx_id, StreamSink { tokens: tok_tx, done: done_tx });

    let status = unsafe {
        ffi::ai_respond_stream(
            session_id,
            c_prompt.as_ptr(),
            c_opts.as_ptr(),
            ctx_id as *mut c_void,
            token_trampoline,
            stream_completion_trampoline,
        )
    };
    if status != 0 {
        PENDING_STREAMS.lock().unwrap().remove(&ctx_id);
        return Err(Error::Native(format!("ai_respond_stream returned {status}")));
    }

    tokio::spawn(async move {
        while let Some(chunk) = tok_rx.recv().await {
            let _ = channel.send(chunk);
        }
    });

    let payload = done_rx.await.map_err(|_| Error::Native("stream completion channel dropped".into()))?;
    if payload.ok { Ok(payload.text) } else { Err(map_native_error(payload.text)) }
}

#[command]
pub async fn generate_stream(
    prompt: String,
    options: Option<GenerationOptions>,
    on_token: Channel<String>,
) -> Result<String> {
    with_ephemeral_session(|id| respond_stream_inner(id, prompt, options, on_token)).await
}

#[command]
pub async fn respond_stream(
    session_id: u64,
    prompt: String,
    options: Option<GenerationOptions>,
    on_token: Channel<String>,
) -> Result<String> {
    respond_stream_inner(session_id, prompt, options, on_token).await
}

// ── Tool calling ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallEvent {
    pub session_id: u64,
    pub call_id: u64,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    pub call_id: u64,
    pub result: serde_json::Value,
    #[serde(default)]
    pub is_error: bool,
}

#[command]
pub async fn resolve_tool_call(payload: ToolCallResult) -> Result<()> {
    let result_json = serde_json::to_string(&payload.result)?;
    let c_result = to_cstring(&result_json)?;
    let status = unsafe {
        ffi::ai_resolve_tool_call(
            payload.call_id,
            c_result.as_ptr(),
            if payload.is_error { 1 } else { 0 },
        )
    };
    if status != 0 {
        return Err(Error::Native(format!("ai_resolve_tool_call returned {status}")));
    }
    Ok(())
}

type ToolCallEmitter = Box<dyn Fn(ToolCallEvent) + Send + Sync + 'static>;
static TOOL_CALL_EMITTER: OnceCell<ToolCallEmitter> = OnceCell::new();

pub(crate) fn install_tool_call_emitter<R: Runtime>(app: AppHandle<R>) {
    use tauri::Emitter;
    let _ = TOOL_CALL_EMITTER.set(Box::new(move |event: ToolCallEvent| {
        let _ = app.emit(TOOL_CALL_EVENT, event);
    }));
}

pub(crate) extern "C" fn tool_dispatcher_trampoline(
    _ctx: *mut c_void,
    session_id: u64,
    call_id: u64,
    name: *const c_char,
    args_json: *const c_char,
) {
    let name = unsafe { read_cstr(name) };
    let args_str = unsafe { read_cstr(args_json) };
    let arguments = serde_json::from_str(&args_str).unwrap_or(serde_json::Value::Null);
    if let Some(emit) = TOOL_CALL_EMITTER.get() {
        emit(ToolCallEvent { session_id, call_id, name, arguments });
    }
}
