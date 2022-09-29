//
// Copyright (C) Lenovo ThinkBook Gen4 Project.
//
// This program is protected under international and China copyright laws as
// an unpublished work. This program is confidential and proprietary to the
// copyright owners. Reproduction or disclosure, in whole or in part, or the
// production of derivative works therefrom without the express permission of
// the copyright owners is prohibited.
//
// All rights reserved.
//

use log::{Level, LevelFilter, Metadata, Record};

/// This implements `log::Log`, and so can be used as a logging provider.
/// It forwards log messages to the Windows `OutputDebugString` API.
pub struct DebuggerLogger;

/// This is a static instance of `DebuggerLogger`. Since `DebuggerLogger`
/// contains no state, this can be directly registered using `log::set_logger`.
///
/// Example:
///
/// ```
/// // During initialization:
/// log::set_logger(&win_dbg_logger::DEBUGGER_LOGGER).unwrap();
/// log::set_max_level(log::LevelFilter::Debug);
///
/// // Throughout your code:
/// use log::{info, debug};
///
/// info!("Hello, world!");
/// debug!("Hello, world, in detail!");
/// ```
pub static DEBUGGER_LOGGER: DebuggerLogger = DebuggerLogger;

impl log::Log for DebuggerLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // && is_debugger_present() {
            let s = format!(
                "{}({}): {} - {}\r\n",
                record.file().unwrap_or("<unknown>"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            );
            output_debug_string(&s);
        }
    }

    fn flush(&self) {}
}

/// Calls the `OutputDebugString` API to log a string.
///
/// On non-Windows platforms, this function does nothing.
///
/// See [`OutputDebugStringW`](https://docs.microsoft.com/en-us/windows/win32/api/debugapi/nf-debugapi-outputdebugstringw).
pub fn output_debug_string(s: &str) {
    #[cfg(windows)]
    {
        let len = s.encode_utf16().count() + 1;
        let mut s_utf16: Vec<u16> = Vec::with_capacity(len + 1);
        s_utf16.extend(s.encode_utf16());
        s_utf16.push(0);
        unsafe {
            OutputDebugStringW(&s_utf16[0]);
        }
    }
}

#[cfg(windows)]
extern "stdcall" {
    fn OutputDebugStringW(chars: *const u16);
    fn IsDebuggerPresent() -> i32;
}

/// Checks whether a debugger is attached to the current process.
///
/// On non-Windows platforms, this function always returns `false`.
///
/// See [`IsDebuggerPresent`](https://docs.microsoft.com/en-us/windows/win32/api/debugapi/nf-debugapi-isdebuggerpresent).
pub fn is_debugger_present() -> bool {
    #[cfg(windows)]
    {
        unsafe { IsDebuggerPresent() != 0 }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Sets the `DebuggerLogger` as the currently-active logger.
///
/// If an error occurs when registering `DebuggerLogger` as the current logger,
/// this function will output a warning and will return normally. It will not panic.
/// This behavior was chosen because `DebuggerLogger` is intended for use in debugging.
/// Panicking would disrupt debugging and introduce new failure modes. It would also
/// create problems for mixed-mode debugging, where Rust code is linked with C/C++ code.
pub fn init() {
    match log::set_logger(&DEBUGGER_LOGGER) {
        Ok(()) => {}
        Err(_) => {
            // There's really nothing we can do about it.
            output_debug_string(
                "Warning: Failed to register DebuggerLogger as the current Rust logger.\r\n",
            );
        }
    }
}

macro_rules! define_init_at_level {
    ($func:ident, $level:ident) => {
        /// This can be called from C/C++ code to register the debug logger.
        ///
        /// For Windows DLLs that have statically linked an instance of `win_dbg_logger` into them,
        /// `DllMain` should call `win_dbg_logger_init_<level>()` from the `DLL_PROCESS_ATTACH` handler.
        /// For example:
        ///
        /// ```ignore
        /// // Calls into Rust code.
        /// extern "C" void __cdecl rust_win_dbg_logger_init_debug();
        ///
        /// BOOL WINAPI DllMain(HINSTANCE hInstance, DWORD reason, LPVOID reserved) {
        ///     switch (reason) {
        ///         case DLL_PROCESS_ATTACH:
        ///             rust_win_dbg_logger_init_debug();
        ///             // ...
        ///     }
        ///     // ...
        /// }
        /// ```
        ///
        /// For Windows executables that have statically linked an instance of `win_dbg_logger` into
        /// them, call `win_dbg_logger_init_<level>()` during app startup.
        #[no_mangle]
        pub extern "C" fn $func() {
            init();
            log::set_max_level(LevelFilter::$level);
        }
    };
}

define_init_at_level!(rust_win_dbg_logger_init_info, Info);
define_init_at_level!(rust_win_dbg_logger_init_trace, Trace);
define_init_at_level!(rust_win_dbg_logger_init_debug, Debug);
define_init_at_level!(rust_win_dbg_logger_init_warn, Warn);
define_init_at_level!(rust_win_dbg_logger_init_error, Error);
