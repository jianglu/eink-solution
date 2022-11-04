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
mod mag_win;
mod magnify;
mod settings;
mod specialized;
mod topmost;
mod utils;
mod win_utils;
mod window;
mod tcon_api;
mod wmi_service;

use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, AtomicIsize, Ordering},
        Arc,
    },
};

use always_on_top::{AlwaysOnTop, ALWAYS_ON_TOP};
use anyhow::bail;
use log::info;
use mag_win::MagWindow;
use parking_lot::{Mutex, RwLock};
use settings::SETTINGS;
use structopt::StructOpt;
use tokio::runtime::Runtime;
use topmost::{set_window_topmost, TOPMOST_MANAGER};
use utils::get_current_exe_dir;
use win_utils::run_as_admin;
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Graphics::Gdi::ValidateRect,
    Win32::{
        System::{
            Console::GetConsoleWindow,
            Threading::{CreateThreadpoolTimer, SetThreadpoolTimer, TP_TIMER},
        },
        UI::{
            Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP},
            Magnification::{
                MagGetWindowFilterList, MagInitialize, MagSetColorEffect, MagSetWindowFilterList,
                MagUninitialize, MW_FILTERMODE, MW_FILTERMODE_EXCLUDE, MW_FILTERMODE_INCLUDE,
            },
            WindowsAndMessaging::*,
        },
    },
    Win32::{
        System::{LibraryLoader::GetModuleHandleA, Threading::TP_CALLBACK_INSTANCE},
        UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
    },
};
use windows_hotkeys::{
    keys::{winapi_keycodes::VK_LWIN, ModKey, VKey},
    HotkeyManager,
};
use wineventhook::{
    raw_event::{OBJECT_CREATE, SYSTEM_FOREGROUND},
    AccessibleObjectId, EventFilter, WindowEventHook,
};
use wmi_service::WMI_SERVICE;

use crate::{
    specialized::set_monitor_specialized,
    window::{enumerate_all_windows, enumerate_capturable_windows},
};

type AnyResult<T> = anyhow::Result<T>;

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

/// 设置窗口隐藏
pub fn set_window_minimize(hwnd: HWND) {
    if unsafe { ShowWindow(hwnd, SW_MINIMIZE).as_bool() } {
        // ignore
    } else {
        log::error!("Cannot hide launcher window");
    }
}

/// 设置窗口隐藏
pub fn set_window_maximize(hwnd: HWND) {
    if unsafe { ShowWindow(hwnd, SW_MAXIMIZE).as_bool() } {
        // ignore
    } else {
        log::error!("Cannot hide launcher window");
    }
}

// 
pub fn set_window_hidden(hwnd: HWND) {
    if unsafe { ShowWindow(hwnd, SW_HIDE).as_bool() } {
        // ignore
    } else {
        log::error!("Cannot hide launcher window");
    }
}

/// 设置窗口隐藏
pub fn set_window_shown(hwnd: HWND) {
    if unsafe { ShowWindow(hwnd, SW_SHOW).as_bool() } {
        // ignore
    } else {
        log::error!("Cannot hide launcher window");
    }
}

/// 切换到 EINK Launcher 模式
fn switch_to_eink_launcher_mode() {
    log::info!("switch_to_eink_launcher_mode");

    if let Ok(eink_monitor_id) = SETTINGS.read().get_string("eink_monitor_id") {
        if eink_monitor_id.len() > 8 {
            set_monitor_specialized(&eink_monitor_id, false).unwrap();
        }
        if let Ok(oled_monitor_id) = SETTINGS.read().get_string("oled_monitor_id") {
            if oled_monitor_id.len() > 8 {
                set_monitor_specialized(&oled_monitor_id, true).unwrap();

                // OLED 桌面模式采用 Hybrid Hybrid 模式

                // GI_MIPI_BROWSER = 0x02;
                // GI_MIPI_HYBRID = 0xF0;
                tcon_api::eink_set_mipi_mode(0xF0);

                // 置顶 Launcher
                find_launcher_and_set_topmost();

                // 置顶悬浮球
                find_floating_button_and_set_topmost();

                IS_OLED.store(false, Ordering::Relaxed);
            }
        }
    }
}

/// 查找窗口
fn find_window_by_title<P>(name: P) -> anyhow::Result<HWND>
where
    P: Into<PCSTR>,
{
    let hwnd = unsafe { FindWindowA(None, name) };
    if hwnd == HWND(0) {
        bail!("Cannot find window");
    } else {
        Ok(hwnd)
    }
}

/// 查找窗口
fn find_window_by_classname<P>(name: P) -> anyhow::Result<HWND>
where
    P: Into<PCSTR>,
{
    let hwnd = unsafe { FindWindowA(name, None) };
    if hwnd == HWND(0) {
        bail!("Cannot find window");
    } else {
        Ok(hwnd)
    }
}

/// 查找 Launcher 并且设置为置顶模式
fn find_launcher_and_set_topmost() {
    if let Ok(hwnd) =
        find_window_by_title(s!("ThinkbookEinkPlus2A7678FA-39DD-4C1D-8981-34A451919F59"))
    {
        set_window_shown(hwnd);
        set_window_maximize(hwnd);

        // 调用 Windows 的置顶方法
        unsafe {
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_SHOWWINDOW | SWP_NOSIZE);
            SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_SHOWWINDOW | SWP_NOSIZE);
        }

        // set_window_topmost(hwnd);
    } else {
        log::error!("Cannot find ThinkBook Eink Plus Launcher");
        start_launcher();
    }
}


/// 查找 Launcher 并且设置为置顶模式
fn find_floating_button_and_set_topmost() {
    if let Ok(hwnd) =
        find_window_by_title(s!("86948044-41D9-464B-B533-15FE92A0BA26"))
    {
        // 调用 Windows 的置顶方法
        unsafe {
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_SHOWWINDOW | SWP_NOSIZE);
        }

        // set_window_topmost(hwnd);
    } else {
        log::error!("Cannot find ThinkBook Eink Plus Floating Button");
    }
}

/// 启动 Launcher
fn start_launcher() {
    // topmost manager 可执行程序和 eink-service 在同一目录
    let exe_dir = get_current_exe_dir();
    let topmost_manager_exe = exe_dir.join("LenovoGen4.Launcher.exe");

    let _pid = run_as_admin(
        exe_dir.to_str().unwrap(),
        topmost_manager_exe.to_str().unwrap(),
    )
    .unwrap();
}

/// 查找 Launcher 并且设置为隐藏
fn find_launcher_and_set_hidden() {
    if let Ok(hwnd) =
        find_window_by_title(s!("ThinkbookEinkPlus2A7678FA-39DD-4C1D-8981-34A451919F59"))
    {
        set_window_hidden(hwnd);
    } else {
        log::error!("Cannot find ThinkBook Eink Plus Launcher");
    }
}

static IS_OLED: AtomicBool = AtomicBool::new(true);

// 切换搭配 OLED Windows 桌面模式
fn switch_to_oled_windows_desktop_mode() {
    log::info!("switch_to_oled_windows_desktop_mode");

    if let Ok(oled_monitor_id) = SETTINGS.read().get_string("oled_monitor_id") {
        set_monitor_specialized(&oled_monitor_id, false).unwrap();

        if let Ok(eink_monitor_id) = SETTINGS.read().get_string("eink_monitor_id") {
            set_monitor_specialized(&eink_monitor_id, true).unwrap();

            // OLED 桌面模式采用 Hybrid Browser 模式

            // GI_MIPI_BROWSER = 0x02;
            // GI_MIPI_HYBRID = 0xF0;
            //tcon_api::eink_set_mipi_mode(0x02);
            tcon_api::eink_show_shutdown_cover();

            // 最小化 Launcher
            find_launcher_and_set_hidden();

            // 清除当前置顶的窗口
            TOPMOST_MANAGER.lock().clear_current_topmost_window();

            IS_OLED.store(true, Ordering::Relaxed);
        }
    }
}

/// 在 EINK/OLED 模式之间切换
pub fn switch_eink_oled_display() {
    log::info!("switch_eink_oled_display");

    // 防止通过 SendMessage 形成进程间死锁
    std::thread::spawn(|| {
        if IS_OLED.load(Ordering::Relaxed) {
            switch_to_eink_launcher_mode();
        } else {
            switch_to_oled_windows_desktop_mode();
        }
    });
}

/// 初始化 Panic 的输出为 OutputDebugString
fn init_panic_output() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {:?}", info);
    }));
}

fn main() -> AnyResult<()> {
    // 设置当前的活动日志系统为 OutputDebugString 输出
    eink_logger::init_with_level(log::Level::Trace)?;

    init_panic_output();

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
        info!("Received LidEvent: {:?}", evt);
    });
    WMI_SERVICE.lock().on_mode_switch_event(|mode| {
        info!("Received OnModeSwitchEvent: {:?}", mode);

        match mode {
            // OLED
            1 | 2 | 3 | 7 | 9 => {
                switch_to_oled_windows_desktop_mode();
            },
            // EINK
            4 | 5 | 6 | 8 | 10 => {
                switch_to_eink_launcher_mode();
            },
            _ => {
                info!("Unused mode : {mode}")
            }
        }
    });
    wmi_service::start_service(&WMI_SERVICE).expect("Error start WMI_SERVICE");

    // 线程开启热键响应
    let mut hkm = HotkeyManager::new();

    // CTRL-ALT-Q 退出
    hkm.register(VKey::Q, &[ModKey::Ctrl, ModKey::Alt], move || {
        std::process::exit(0);
    })
    .expect("Cannot register hot-key CTRL-ALT-Q");

    // CTRL-SHIFT-M 进入 EINK
    hkm.register(VKey::M, &[ModKey::Ctrl, ModKey::Shift], move || {
        switch_to_eink_launcher_mode();
    })
    .expect("Cannot register hot-key CTRL-SHIFT-M");

    // CTRL-SHIFT-N 进入 OLED
    hkm.register(VKey::N, &[ModKey::Ctrl, ModKey::Shift], move || {
        switch_to_oled_windows_desktop_mode();
    })
    .expect("Cannot register hot-key CTRL-SHIFT-N");

     // CTRL-WIN-F13 进入 EINK
     match hkm.register(VKey::F13, &[ModKey::Ctrl, ModKey::Win], move || {
        switch_eink_oled_display();
     }) {
        Ok(_) => (), // ignore
        Err(err) => {
            log::error!("Cannot register hot-key CTRL-WIN-F13: err:{err:?}");
        },
    }
    
     // CTRL-WIN-F14 进入 OLED
     match hkm.register(VKey::F14, &[ModKey::Ctrl, ModKey::Win], move || {
         switch_to_oled_windows_desktop_mode();
     }) {
        Ok(_) => (), // ignore
        Err(err) => {
            log::error!("Cannot register hot-key CTRL-WIN-F14: err:{err:?}");
        },
    }

    // 进入 OLED 桌面模式
    switch_to_oled_windows_desktop_mode();

    hkm.event_loop();

    ALWAYS_ON_TOP.lock().start().unwrap();
    Ok(())
}
