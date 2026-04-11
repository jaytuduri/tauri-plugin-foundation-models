//! C ABI bindings to the Swift `AppleIntelligenceFFI` static library.
//!
//! All strings are UTF-8, NUL-terminated. Swift-allocated strings must be
//! released via [`ai_free_string`]. Status codes: 0 = success, nonzero = error
//! (the error message, if any, is returned via an out-pointer that must also
//! be freed).

use std::os::raw::{c_char, c_int, c_void};

pub type TokenCallback = extern "C" fn(ctx: *mut c_void, chunk: *const c_char);
pub type CompletionCallback =
    extern "C" fn(ctx: *mut c_void, status: c_int, payload: *const c_char);
pub type ToolCallCallback = extern "C" fn(
    ctx: *mut c_void,
    session_id: u64,
    call_id: u64,
    name: *const c_char,
    args_json: *const c_char,
);

#[link(name = "AppleIntelligenceFFI", kind = "static")]
extern "C" {
    pub fn ai_free_string(ptr: *mut c_char);

    /// Returns a JSON string describing availability, e.g.
    /// `{"available":true}` or `{"available":false,"reason":"..."}`.
    pub fn ai_availability() -> *mut c_char;

    /// Creates a session. `instructions_json` encodes `{instructions, tools}`.
    /// Returns session id via `out_session_id`. Status 0 = ok.
    pub fn ai_create_session(
        instructions_json: *const c_char,
        out_session_id: *mut u64,
        out_error: *mut *mut c_char,
    ) -> c_int;

    pub fn ai_close_session(session_id: u64) -> c_int;

    /// Sends a prompt, invokes `completion` with the full text on finish.
    pub fn ai_respond(
        session_id: u64,
        prompt: *const c_char,
        options_json: *const c_char,
        ctx: *mut c_void,
        completion: CompletionCallback,
    ) -> c_int;

    /// Sends a prompt, invokes `token` for each chunk then `completion` on end.
    pub fn ai_respond_stream(
        session_id: u64,
        prompt: *const c_char,
        options_json: *const c_char,
        ctx: *mut c_void,
        token: TokenCallback,
        completion: CompletionCallback,
    ) -> c_int;

    /// Installs the process-wide tool-call dispatcher. Must be called once at
    /// plugin init before any session uses tools.
    pub fn ai_set_tool_dispatcher(ctx: *mut c_void, cb: ToolCallCallback);

    /// Resolves a pending tool call with a JSON result string.
    pub fn ai_resolve_tool_call(
        call_id: u64,
        result_json: *const c_char,
        is_error: c_int,
    ) -> c_int;
}
