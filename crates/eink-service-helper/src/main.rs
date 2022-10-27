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

use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, AtomicIsize, Ordering},
        Arc,
    },
};

use anyhow::bail;
use log::info;
use mag_win::MagWindow;
use parking_lot::{Mutex, RwLock};
use settings::SETTINGS;
use structopt::StructOpt;
use tokio::runtime::Runtime;
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

use crate::{
    specialized::set_monitor_specialized,
    window::{enumerate_all_windows, enumerate_capturable_windows},
};

type AnyResult<T> = anyhow::Result<T>;

static mut g_timer_due_time: FILETIME = FILETIME {
    dwLowDateTime: 0,
    dwHighDateTime: 0,
};

#[repr(C)]
union FILETIME64 {
    pub ft: FILETIME,
    pub vl: i64,
}

fn create_relative_filetime_ms(milliseconds: u32) -> FILETIME {
    let ft = FILETIME64 {
        vl: -(milliseconds as i64 * 10000),
    };
    return unsafe { ft.ft };
}

unsafe extern "system" fn timer_tick_event(
    _: *mut TP_CALLBACK_INSTANCE,
    _context: *mut c_void,
    _: *mut TP_TIMER,
) {
    refresh_magnifier();

    // Reset timer to expire one time at next interval
    if g_enable.load(Ordering::Relaxed) {
        SetThreadpoolTimer(
            g_refresh_timer.load(Ordering::Relaxed) as *mut TP_TIMER,
            Some(&g_timer_due_time),
            0,
            0,
        );
    }
}

#[static_init::dynamic(lazy)]
static mut g_mag: RwLock<Option<MagWindow>> = RwLock::new(None);

/// Called in the timer tick event to refresh the magnification area drawn and lens (host window) position and size
unsafe fn refresh_magnifier() {
    let mut lock = g_mag.write();
    let a = lock.get_mut().as_mut().unwrap();

    let mut mag_pos = POINT { x: 0, y: 0 };
    let pan_offset = POINT { x: 0, y: 0 };
    let lens_size = SIZE { cx: 2560, cy: 1600 };
    a.update_magnifier(&mut mag_pos, pan_offset, lens_size);

    let topmost_hwnd = FindWindowA(None, s!("DebugView"));
    let mag_hwnd = a.get_handle();

    SetParent(topmost_hwnd, mag_hwnd);
    SetWindowPos(
        topmost_hwnd,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE,
    );
}

static mut g_enable: AtomicBool = AtomicBool::new(false);
static mut g_refresh_timer: AtomicIsize = AtomicIsize::new(0);

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
    config_file: Option<String>,
}

/// 切换到 EINK Launcher 模式
fn switch_to_eink_launcher_mode() {
    if let Ok(eink_monitor_id) = SETTINGS.read().get_string("eink_monitor_id") {
        if eink_monitor_id.len() > 8 {
            set_monitor_specialized(&eink_monitor_id, false).unwrap();
        }
        if let Ok(oled_monitor_id) = SETTINGS.read().get_string("oled_monitor_id") {
            if oled_monitor_id.len() > 8 {
                set_monitor_specialized(&oled_monitor_id, true).unwrap();

                // 置顶 Launcher
            }
        }
    }
}

// 切换搭配 OLED Windows 桌面模式
fn switch_to_oled_windows_desktop_mode() {
    if let Ok(oled_monitor_id) = SETTINGS.read().get_string("oled_monitor_id") {
        set_monitor_specialized(&oled_monitor_id, false).unwrap();

        if let Ok(eink_monitor_id) = SETTINGS.read().get_string("eink_monitor_id") {
            set_monitor_specialized(&eink_monitor_id, true).unwrap();

            // 最小化 Launcher
        }
    }
}

fn main() -> AnyResult<()> {
    let mut opt = Opt::from_args();

    // 监听目标进程关闭，绑定生命周期
    if let Some(pid) = opt.pid.take() {
        eink_winkits::process_waiter::on_process_terminate(pid, |err_code| {
            std::process::exit(err_code.0 as i32);
        });
    }

    // 设置当前的活动日志系统为 OutputDebugString 输出
    eink_logger::init_with_level(log::Level::Trace)?;

    // 开启热键响应线程
    std::thread::spawn(move || {
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

        hkm.event_loop();
    });

    Ok(())
}
