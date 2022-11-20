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

// 使用 windows subsystem 子系统
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod always_on_top;
mod hotkey;
mod keyboard_manager;
mod ls_note_starter;
mod mag_win;
mod magnify;
mod mode_manager;
mod monitor;
mod settings;
mod specialized;
mod tcon_api;
mod topmost;
mod utils;
mod win_utils;
mod window;
mod wmi_service;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use always_on_top::ALWAYS_ON_TOP;
use ls_note_starter::LockScreenNoteManager;
use ntapi::winapi::um::winnt;
use parking_lot::Mutex;
use structopt::StructOpt;
use topmost::TOPMOST_MANAGER;
use windows::Win32::Foundation::*;
use windows_hotkeys::keys::{ModKey, VKey};
use windows_hotkeys::HotkeyManager;
use winnt::KEY_ALL_ACCESS;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;
use wmi_service::WMI_SERVICE;

use crate::mode_manager::MODE_MANAGER;

type AnyResult<T> = anyhow::Result<T>;

shadow_rs::shadow!(build);

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Eink Service Helper",
    about = "Bottom-half of eink service, running in admin account"
)]
struct Opt {
    /// verbosity level
    #[structopt(short = "p", long = "pid")]
    pid: Option<u32>,
    #[structopt(short = "c", long = "config-file")]
    _config_file: Option<String>,
}

/// 将当前显示模式保存到注册表
pub fn save_display_mode_to_registry(mode: &str) {
    let key_path = r#"SOFTWARE\Lenovo\ThinkBookEinkPlus"#;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let mut key = hklm.open_subkey_with_flags(key_path, KEY_ALL_ACCESS);

    if key.is_err() {
        // Maybe notfound, ignore any error
        let _ = hklm.create_subkey(key_path);

        key = hklm.open_subkey(key_path);
        if key.is_err() {
            // 多次错误，只能退出，输出到日志
            log::error!("Cannot open '{}' registry subkey", key_path);
            return;
        }
    }

    let key = key.unwrap();
    key.set_value("DisplayMode", &mode.to_owned())
        .expect("Cannot save 'DisplayMode' to registry");
}

/// 在 EINK/OLED 模式之间切换
pub fn switch_eink_oled_display() {
    log::info!("switch_eink_oled_display");

    MODE_MANAGER.lock().request_switch_eink_oled_display();

    // 防止通过 SendMessage 形成进程间死锁
    // std::thread::spawn(|| {
    //     if IS_OLED.load(Ordering::Relaxed) {
    //         switch_to_eink_launcher_mode();
    //     } else {
    //         switch_to_oled_windows_desktop_mode();
    //     }
    // });
}

static LAST_MODE: AtomicU32 = AtomicU32::new(u32::MAX);

fn main() -> AnyResult<()> {
    // 设置当前的活动日志系统为 OutputDebugString 输出
    eink_logger::init_with_level(log::Level::Trace)?;

    eink_logger::init_panic_output();

    let mut opt = Opt::from_args();

    // 监听目标进程关闭，绑定生命周期
    if let Some(pid) = opt.pid.take() {
        eink_winkits::process_waiter::on_process_terminate(pid, |err_code| {
            std::process::exit(err_code.0 as i32);
        });
    }

    //
    // 启动各种服务
    //
    ALWAYS_ON_TOP.lock().start().unwrap();

    TOPMOST_MANAGER.lock().start().unwrap();

    WMI_SERVICE.lock().on_lid_event(|evt| {
        log::info!("Received LidEvent: {:?}", evt);
    });

    // Cases: 4 -> 11 -> 3
    WMI_SERVICE.lock().on_mode_switch_event(|mode| {
        log::info!("Received OnModeSwitchEvent: {:?}", mode);

        let last_mode = LAST_MODE.load(Ordering::Relaxed);

        match mode {
            // OLED
            1 | 2 => {
                if last_mode == 10 || last_mode == 11 {
                    MODE_MANAGER.lock().request_to_oled_windows_desktop_mode();
                }
            }
            9 => {
                // ignore
            }
            3 | 7 => {
                MODE_MANAGER.lock().request_to_oled_windows_desktop_mode();
            }
            // EINK
            5 | 6 => {
                if last_mode == 10 || last_mode == 11 {
                    MODE_MANAGER.lock().request_to_eink_launcher_mode();
                }
            }
            10 => {
                // ignore
            }
            4 | 8 => {
                MODE_MANAGER.lock().request_to_eink_launcher_mode();
            }
            _ => {
                log::info!("Unused mode : {mode}")
            }
        }

        // 存储当前模式
        LAST_MODE.store(mode, Ordering::Relaxed);
    });
    wmi_service::start_service(&WMI_SERVICE).expect("Error start WMI_SERVICE");

    // Give BIOS a trigger，disable default Lid Event processing
    WMI_SERVICE.lock().get_display_working_status();

    // Start LockScreenNoteManager in detached thread
    let _deteched = std::thread::spawn(move || {
        let lsn_starter = Arc::new(Mutex::new(
            LockScreenNoteManager::new().expect("Cannot instantiate LOCKSCREEN_NOTE_STARTER"),
        ));

        match lsn_starter.lock().start() {
            Ok(_) => (),
            Err(err) => {
                log::error!(
                    "Cannot register LockScreen Detecter: err:{err:?}, last_win_error:{:?}",
                    unsafe { GetLastError() }
                );
            }
        }

        lsn_starter.lock().event_loop();
    });

    // Start HotkeyManager
    // TODO: it should be in another detached thread
    let mut hkm = HotkeyManager::new();

    // CTRL-ALT-Q 退出
    match hkm.register(VKey::Q, &[ModKey::Ctrl, ModKey::Alt], move || {
        std::process::exit(0);
    }) {
        Ok(_) => (), // ignore
        Err(err) => {
            log::error!(
                "Cannot register hot-key CTRL-ALT-Q: err:{err:?}, last_win_error:{:?}",
                unsafe { GetLastError() }
            );
        }
    }

    // CTRL-SHIFT-M 进入 EINK
    match hkm.register(VKey::M, &[ModKey::Ctrl, ModKey::Shift], move || {
        MODE_MANAGER.lock().request_to_eink_launcher_mode();
    }) {
        Ok(_) => (), // ignore
        Err(err) => {
            log::error!(
                "Cannot register hot-key CTRL-SHIFT-M: err:{err:?}, last_win_error:{:?}",
                unsafe { GetLastError() }
            );
        }
    }

    // CTRL-SHIFT-N 进入 OLED
    match hkm.register(VKey::N, &[ModKey::Ctrl, ModKey::Shift], move || {
        MODE_MANAGER.lock().request_to_oled_windows_desktop_mode();
    }) {
        Ok(_) => (), // ignore
        Err(err) => {
            log::error!(
                "Cannot register hot-key CTRL-SHIFT-N: err:{err:?}, last_win_error:{:?}",
                unsafe { GetLastError() }
            );
        }
    }

    // CTRL-Shift-F13 进入 EINK
    match hkm.register(VKey::F13, &[ModKey::Ctrl, ModKey::Shift], move || {
        log::info!("Clicked: CTRL-Shift-F13");
        MODE_MANAGER.lock().request_switch_eink_oled_display();
    }) {
        Ok(_) => (), // ignore
        Err(err) => {
            log::error!("Cannot register hot-key CTRL-WIN-F13: err:{err:?}");
        }
    }

    // CTRL-Shift-F14
    match hkm.register(VKey::F14, &[ModKey::Ctrl, ModKey::Shift], move || {
        log::info!("Clicked: CTRL-Shift-F14")
    }) {
        Ok(_) => (), // ignore
        Err(err) => {
            log::error!("Cannot register hot-key CTRL-WIN-F14: err:{err:?}");
        }
    }

    // CTRL-Shift-F15
    match hkm.register(VKey::F15, &[ModKey::Ctrl, ModKey::Shift], move || {
        log::info!("Clicked: CTRL-Shift-F15")
    }) {
        Ok(_) => (), // ignore
        Err(err) => {
            log::error!("Cannot register hot-key CTRL-WIN-F15: err:{err:?}");
        }
    }

    // 进入 OLED 桌面模式
    log::info!("After system-up, switch to oled desktop mode");
    MODE_MANAGER.lock().request_to_oled_windows_desktop_mode();

    hkm.event_loop();

    ALWAYS_ON_TOP.lock().start().unwrap();
    Ok(())
}
