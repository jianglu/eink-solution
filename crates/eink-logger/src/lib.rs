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

use std::path::PathBuf;

use anyhow;
use fast_log::appender::{Command, FastLogRecord, RecordFormat};
use fast_log::{FastLogFormat, FastLogFormatJson};
use windows::core::PCWSTR;
use windows::w;
use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;

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
            OutputDebugStringW(PCWSTR::from_raw(&s_utf16[0]));
        }
    }
}

pub struct DebugViewLog {}

impl fast_log::appender::LogAppender for DebugViewLog {
    fn do_logs(&self, record: &[fast_log::appender::FastLogRecord]) {
        for line in record.iter() {
            if line.command != fast_log::appender::Command::CommandRecord {
                continue;
            }

            // let now = fastdate::DateTime::from(line.now);
            let msg = format!(
                "[{}][{}][{}][{}] {}",
                // &now,
                line.target,
                line.file,
                line.line.unwrap_or_default(),
                line.level,
                &line.args
            );

            if let Ok(msg_u16) = widestring::U16CString::from_str(&msg) {
                unsafe {
                    OutputDebugStringW(PCWSTR::from_raw(msg_u16.as_ptr()));
                }
            } else {
                // ignore
            }
        }
    }
}

pub struct CustomLogFormat {}

impl RecordFormat for CustomLogFormat {
    fn do_format(&self, arg: &mut FastLogRecord) {
        match &arg.command {
            Command::CommandRecord => {
                let now = fastdate::DateTime::from(arg.now);
                arg.formated = format!(
                    "{} [{}][{}][{}][{}] {}\n",
                    &now,
                    arg.target,
                    arg.file,
                    arg.line.unwrap_or_default(),
                    arg.level,
                    arg.args,
                );
            }
            Command::CommandExit => {}
            Command::CommandFlush(_) => {}
        }
    }
}

pub fn init() -> anyhow::Result<()> {
    init_with_level(log::Level::Trace)
}

/// Initialise the logger with a specific log level.
///
/// Log messages below the given [`Level`] will be filtered.
/// The `RUST_LOG` environment variable is not used.
pub fn init_with_level(_level: log::Level) -> anyhow::Result<()> {
    unsafe {
        OutputDebugStringW(w!("eink-logger::init_with_level"));
    }

    // Get logging dir for different account
    // >> C:\Windows\system32\config\systemprofile\AppData\Local\Lenovo\ThinkBookEinkPlus\logging
    // >> %localappdata%\Lenovo\ThinkBookEinkPlus\logging
    // >> C:\Lenovo-Fallback-Log\
    let local_dir = match dirs::data_local_dir() {
        Some(mut dir) => {
            dir.push(&"Lenovo\\ThinkBookEinkPlus\\logging");
            dir
        }
        None => PathBuf::from("C:\\Lenovo-Fallback-Log"),
    };

    // Just unwrap
    let local_dir = local_dir.to_str().unwrap();

    // Get current exe filename without extersion name for logging
    let current_exe = std::env::current_exe().unwrap();
    let file_name = current_exe.file_name().unwrap();
    let file_name = file_name.to_str().unwrap().trim_end_matches(".exe");

    let file_path = format!("{local_dir}\\{file_name}.log").replace("\\", "/");

    output_debug_string(&file_path);

    if let Err(_) = fast_log::init(
        fast_log::Config::new()
            .custom(DebugViewLog {})
            .format(CustomLogFormat {})
            // .console()
            .file_split(
                &file_path,
                fast_log::consts::LogSize::MB(1),
                fast_log::plugin::file_split::RollingType::KeepNum(128),
                fast_log::plugin::packer::LogPacker {},
            ),
    ) {
        unsafe {
            OutputDebugStringW(w!("Cannot initialize fast_log"));
        }
    }

    Ok(())
}

#[test]
fn test_get_eink_logging_dir() {}
