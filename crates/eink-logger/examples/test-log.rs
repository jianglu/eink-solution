use fast_log::appender::{FastLogRecord, LogAppender};
use fast_log::consts::LogSize;
use fast_log::plugin::file_split::RollingType;
use fast_log::plugin::packer::LogPacker;
use fast_log::Config;
use windows::core::PCWSTR;
use windows::w;
use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;

pub struct DebugViewLog {}

impl LogAppender for DebugViewLog {
    fn do_logs(&self, record: &[FastLogRecord]) {
        for line in record.iter() {
            if line.command == fast_log::appender::Command::CommandRecord {
                let msg = format!(
                    "[{}][{}][{}]: {}",
                    line.target,
                    line.file,
                    line.line.unwrap_or_default(),
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
}

fn main() {
    let current_exe = std::env::current_exe().unwrap();
    let file_name = current_exe.file_name().unwrap();
    let file_name = file_name.to_str().unwrap().trim_end_matches(".exe");

    fast_log::init(Config::new().custom(DebugViewLog {}).console().file_split(
        &format!("target/logs/{file_name}.log"),
        LogSize::KB(16),
        RollingType::KeepNum(128),
        LogPacker {},
    ))
    .unwrap();
    for _ in 0..20000 {
        log::info!("Commencing yak shaving");
    }
    log::logger().flush();
}
