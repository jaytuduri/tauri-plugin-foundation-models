//! End-to-end smoke test hitting the Swift FFI directly (no Tauri).
//!
//!     cargo run --example smoke
//!
//! Exercises: availability → create_session → respond → close_session,
//! then a streaming respond.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::mpsc;
use std::time::Duration;

use tauri_plugin_apple_intelligence as _; // force crate build

#[link(name = "AppleIntelligenceFFI", kind = "static")]
extern "C" {
    fn ai_free_string(ptr: *mut c_char);
    fn ai_availability() -> *mut c_char;
    fn ai_create_session(
        cfg: *const c_char,
        out_id: *mut u64,
        out_err: *mut *mut c_char,
    ) -> c_int;
    fn ai_close_session(id: u64) -> c_int;
    fn ai_respond(
        id: u64,
        prompt: *const c_char,
        opts: *const c_char,
        ctx: *mut c_void,
        cb: extern "C" fn(*mut c_void, c_int, *const c_char),
    ) -> c_int;
    fn ai_respond_stream(
        id: u64,
        prompt: *const c_char,
        opts: *const c_char,
        ctx: *mut c_void,
        token: extern "C" fn(*mut c_void, *const c_char),
        done: extern "C" fn(*mut c_void, c_int, *const c_char),
    ) -> c_int;
}

type CompletionMsg = (c_int, String);

extern "C" fn on_complete(ctx: *mut c_void, status: c_int, payload: *const c_char) {
    let tx = unsafe { &*(ctx as *const mpsc::Sender<CompletionMsg>) };
    let s = if payload.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(payload).to_string_lossy().into_owned() }
    };
    let _ = tx.send((status, s));
}

struct StreamCtx {
    tx: mpsc::Sender<StreamMsg>,
}
enum StreamMsg {
    Token(String),
    Done(c_int, String),
}

extern "C" fn on_token(ctx: *mut c_void, chunk: *const c_char) {
    let c = unsafe { &*(ctx as *const StreamCtx) };
    if chunk.is_null() {
        return;
    }
    let s = unsafe { CStr::from_ptr(chunk).to_string_lossy().into_owned() };
    let _ = c.tx.send(StreamMsg::Token(s));
}

extern "C" fn on_stream_done(ctx: *mut c_void, status: c_int, payload: *const c_char) {
    let c = unsafe { &*(ctx as *const StreamCtx) };
    let s = if payload.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(payload).to_string_lossy().into_owned() }
    };
    let _ = c.tx.send(StreamMsg::Done(status, s));
}

fn main() {
    // 1. Availability
    let raw = unsafe { ai_availability() };
    let json = unsafe { CStr::from_ptr(raw).to_string_lossy().into_owned() };
    unsafe { ai_free_string(raw) };
    println!("availability: {json}");
    if !json.contains("\"available\":true") {
        println!("Apple Intelligence not available — stopping.");
        return;
    }

    // 2. Create session
    let cfg = CString::new(r#"{"instructions":"You are a terse helper.","tools":[]}"#).unwrap();
    let mut id: u64 = 0;
    let mut err: *mut c_char = ptr::null_mut();
    let status = unsafe { ai_create_session(cfg.as_ptr(), &mut id, &mut err) };
    if status != 0 {
        let msg = if err.is_null() {
            "unknown".into()
        } else {
            let s = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
            unsafe { ai_free_string(err) };
            s
        };
        panic!("create_session failed: {msg}");
    }
    println!("session id: {id}");

    // 3. One-shot respond
    let prompt = CString::new("Say hello in five words.").unwrap();
    let opts = CString::new(r#"{"temperature":0.7,"maximumResponseTokens":64}"#).unwrap();
    let (tx, rx) = mpsc::channel::<CompletionMsg>();
    let tx_box = Box::new(tx);
    let rc = unsafe {
        ai_respond(
            id,
            prompt.as_ptr(),
            opts.as_ptr(),
            Box::into_raw(tx_box) as *mut c_void,
            on_complete,
        )
    };
    assert_eq!(rc, 0, "ai_respond dispatch failed");
    let (status, text) = rx
        .recv_timeout(Duration::from_secs(60))
        .expect("respond timed out");
    println!("respond status={status}");
    println!("respond text: {text}");

    // 4. Streaming respond
    println!("\n--- streaming ---");
    let (stx, srx) = mpsc::channel::<StreamMsg>();
    let ctx = Box::new(StreamCtx { tx: stx });
    let prompt2 = CString::new("Count from one to five.").unwrap();
    let rc = unsafe {
        ai_respond_stream(
            id,
            prompt2.as_ptr(),
            opts.as_ptr(),
            Box::into_raw(ctx) as *mut c_void,
            on_token,
            on_stream_done,
        )
    };
    assert_eq!(rc, 0, "ai_respond_stream dispatch failed");
    loop {
        match srx.recv_timeout(Duration::from_secs(60)) {
            Ok(StreamMsg::Token(t)) => {
                print!("{t}");
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
            Ok(StreamMsg::Done(st, _)) => {
                println!("\n[stream done status={st}]");
                break;
            }
            Err(_) => {
                println!("\nstream timed out");
                break;
            }
        }
    }

    // 5. Close
    let rc = unsafe { ai_close_session(id) };
    println!("close status: {rc}");
}
