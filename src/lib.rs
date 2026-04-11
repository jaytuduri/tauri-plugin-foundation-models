//! Tauri plugin exposing Apple Intelligence (FoundationModels) on macOS 26+.

mod commands;
mod error;
mod ffi;
mod session;

pub use error::{Error, Result};

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("apple-intelligence")
        .invoke_handler(tauri::generate_handler![
            commands::availability,
            commands::generate,
            commands::generate_stream,
            commands::create_session,
            commands::respond,
            commands::respond_stream,
            commands::close_session,
            commands::resolve_tool_call,
        ])
        .setup(|app, _api| {
            commands::install_tool_call_emitter(app.app_handle().clone());
            unsafe {
                ffi::ai_set_tool_dispatcher(
                    std::ptr::null_mut(),
                    commands::tool_dispatcher_trampoline,
                );
            }
            Ok(())
        })
        .build()
}
