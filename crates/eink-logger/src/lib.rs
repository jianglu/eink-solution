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
                // utc -> localtime
                let now = now.set_offset(fastdate::offset_sec());
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

/// 初始化 Panic 的输出为 OutputDebugString
pub fn init_panic_output() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {:?}", info);
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("BACKTRACE:\n{}", backtrace.to_string());
    }));
}

#[test]
fn test_backtrace_split() {
    let re = regex::Regex::new(r"(\}, \{)|(\[\{)").unwrap();
    let text = r#"Backtrace [{ fn: "std::backtrace_rs::backtrace::dbghelp::trace", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\std\src\..\..\backtrace\src\backtrace\dbghelp.rs", line: 98 }, { fn: "std::backtrace_rs::backtrace::trace_unsynchronized", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\std\src\..\..\backtrace\src\backtrace\mod.rs", line: 66 }, { fn: "std::backtrace::Backtrace::create", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\std\src\backtrace.rs", line: 332 }, { fn: "std::backtrace::Backtrace::force_capture", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\std\src\backtrace.rs", line: 314 }, { fn: "eink_service::init_panic_output::closure$0", file: "C:\Users\JiangLu\lenovo-thinkbook-gen4\eink-solution\crates\eink-service\src\main.rs", line: 53 }, { fn: "alloc::boxed::impl$47::call", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\alloc\src\boxed.rs", line: 2032 }, { fn: "std::panicking::rust_panic_with_hook", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\std\src\panicking.rs", line: 692 }, { fn: "std::panicking::begin_panic_handler::closure$0", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\std\src\panicking.rs", line: 579 }, { fn: "std::sys_common::backtrace::__rust_end_short_backtrace<std::panicking::begin_panic_handler::closure_env$0,never$>", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\std\src\sys_common\backtrace.rs", line: 137 }, { fn: "std::panicking::begin_panic_handler", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\std\src\panicking.rs", line: 575 }, { fn: "core::panicking::panic_fmt", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b/library\core\src\panicking.rs", line: 65 }, { fn: "core::panicking::panic_display<windows_dll::Error<enum2$<eink_itetcon::itetcon::ITEGetDriveNo> > >", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b\library\core\src\panicking.rs", line: 138 }, { fn: "eink_itetcon::itetcon::ITEGetDriveNo::closure$0", file: "C:\Users\JiangLu\lenovo-thinkbook-gen4\eink-solution\crates\eink-itetcon\src\itetcon.rs", line: 60 }, { fn: "enum2$<core::result::Result<u32 (*)(ref_mut$<u8>),windows_dll::Error<enum2$<eink_itetcon::itetcon::ITEGetDriveNo> > > >::unwrap_or_else<u32 (*)(ref_mut$<u8>),windows_dll::Error<enum2$<eink_itetcon::itetcon::ITEGetDriveNo> >,eink_itetcon::itetcon::ITEGetDr", file: "/rustc/c5d82ed7a4ad94a538bb87e5016e7d5ce0bd434b\library\core\src\result.rs", line: 1504 }, { fn: "eink_itetcon::itetcon::ITEGetDriveNo", file: "C:\Users\JiangLu\lenovo-thinkbook-gen4\eink-solution\crates\eink-itetcon\src\itetcon.rs", line: 60 }, { fn: "eink_itetcon::itetcon_device::IteTconDevice::open", file: "C:\Users\JiangLu\lenovo-thinkbook-gen4\eink-solution\crates\eink-itetcon\src\itetcon_device.rs", line: 60 }, { fn: "eink_service::tcon_service::TconService::start", file: "C:\Users\JiangLu\lenovo-thinkbook-gen4\eink-solution\crates\eink-service\src\tcon_service.rs", line: 72 }, { fn: "eink_service::service_main::run_service", file: "C:\Users\JiangLu\lenovo-thinkbook-gen4\eink-solution\crates\eink-service\src\service_main.rs", line: 139 }, { fn: "eink_service::service_main", file: "C:\Users\JiangLu\lenovo-thinkbook-gen4\eink-solution\crates\eink-service\src\main.rs", line: 93 }, { fn: "eink_service::ffi_service_main", file: "C:\Users\JiangLu\.cargo\registry\src\mirrors.tuna.tsinghua.edu.cn-df7c3c540f42cdbd\windows-service-0.5.0\src\service_dispatcher.rs", line: 53 }, { fn: "QueryServiceConfig2W" }, { fn: "BaseThreadInitThunk" }, { fn: "RtlUserThreadStart" }]"#;
    for line in re.split(text) {
        println!("{}", line.trim());
    }
}
