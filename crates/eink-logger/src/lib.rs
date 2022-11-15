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
use flexi_logger::writers::LogWriter;
use flexi_logger::{Cleanup, Criterion, Level, Naming};
use windows::core::PCWSTR;
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

struct DebugViewLogWriter;

impl LogWriter for DebugViewLogWriter {
    fn write(
        &self,
        _now: &mut flexi_logger::DeferredNow,
        record: &flexi_logger::Record,
    ) -> std::io::Result<()> {
        output_debug_string(&format!("{}", &record.args()));
        Ok(())
    }

    fn flush(&self) -> std::io::Result<()> {
        // ignore
        Ok(())
    }
}

/// Initialise the logger with a specific log level.
///
/// Log messages below the given [`Level`] will be filtered.
/// The `RUST_LOG` environment variable is not used.
pub fn init_with_level(level: Level) -> anyhow::Result<()> {
    // Get logging dir for different account
    // >> C:\Windows\system32\config\systemprofile\AppData\Local\Lenovo\ThinkBookEinkPlus\logging
    // >> %localappdata%\Lenovo\ThinkBookEinkPlus\logging
    // >> C:\Lenovo-Fallback-Log\
    let local_dir = match dirs::data_local_dir() {
        Some(mut dir) => {
            dir.push(&"Lenovo\\ThinkBookEinkPlus\\logging");
            dir
        }
        None => PathBuf::from("C:\\Lenovo-Fallback-Log\\"),
    };

    // Just unwrap
    let local_dir = local_dir.to_str().unwrap();

    // Write all error, warn, and info messages
    flexi_logger::Logger::try_with_str(level.as_str())?
        .log_to_file(
            flexi_logger::FileSpec::default().directory(local_dir),
        )
        // do not truncate the log file when the program is restarted
        .append()
        .rotate(
            Criterion::Size(1024 * 1024 * 2),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(16),
        )
        .format_for_files(|w, now, record| {
            write!(
                w,
                "{} [{}] {}",
                now.now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                &record.args()
            )
        })
        .start()?;

    Ok(())
}

#[test]
fn test_get_eink_logging_dir() {}
