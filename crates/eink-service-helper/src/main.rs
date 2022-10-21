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

mod mag_win;
mod magnify;
mod window;

use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, AtomicIsize, Ordering},
        Arc, Mutex,
    },
};

use anyhow::bail;
use log::info;
use mag_win::MagWindow;
use parking_lot::RwLock;
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

use crate::window::enumerate_capturable_windows;

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
}

static mut g_enable: AtomicBool = AtomicBool::new(false);
static mut g_refresh_timer: AtomicIsize = AtomicIsize::new(0);

fn main() -> AnyResult<()> {
    // 设置当前的活动日志系统为 OutputDebugString 输出
    eink_logger::init_with_level(log::Level::Trace)?;

    let instance = unsafe { GetModuleHandleA(None)? };
    debug_assert!(instance.0 != 0);

    unsafe {
        g_timer_due_time = create_relative_filetime_ms(10);

        // 设置进程 DPI
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // init_screen_dimensions();

        if !MagInitialize().as_bool() {
            bail!("MagInitialize() failed")
        }
    }

    // 创建 HOST 窗口
    let host_hwnd = register_and_create_window()?;

    // 创建 MagWindow
    let mut mag_pos = POINT { x: 0, y: 0 };
    let mag_size = SIZE { cx: 2560, cy: 1600 };
    let mut mag = MagWindow::new(1.0f32, mag_pos.clone(), mag_size.clone())?;

    if !mag.create(instance.clone(), host_hwnd, true) {
        bail!("Cannot create magnify window")
    }

    let mut watcher = window::Watcher::new()?;

    // 排除自身的窗口
    watcher.add_exclude_win(host_hwnd);

    // Window Filter
    unsafe {
        let mut hwnds = Vec::<HWND>::with_capacity(32);

        let wins = enumerate_capturable_windows();
        for win in wins.into_iter() {
            if win.title.contains("DebugView") {
                watcher.set_topmost_app_hwnd(win.handle);
                continue;
            }
            hwnds.push(win.handle);
        }

        MagSetWindowFilterList(
            mag.get_handle(),
            MW_FILTERMODE_EXCLUDE,
            hwnds.len() as i32,
            hwnds.as_mut_ptr(),
        );
    }

    // 保存到全局变量
    (*g_mag.write().get_mut()) = Some(mag);

    // Create and start a timer to refresh the window.
    unsafe {
        g_enable.store(true, Ordering::Relaxed);

        g_refresh_timer.store(
            CreateThreadpoolTimer(Some(timer_tick_event), None, None) as isize,
            Ordering::Relaxed,
        );

        // TODO: this only needs to be started if enabled at start
        SetThreadpoolTimer(
            g_refresh_timer.load(Ordering::Relaxed) as *mut TP_TIMER,
            Some(&g_timer_due_time),
            0,
            0,
        );
    }

    // 进入窗口循环
    unsafe {
        ShowWindow(host_hwnd, SW_SHOWNOACTIVATE);

        let mut message = MSG::default();

        while GetMessageA(&mut message, None, 0, 0).as_bool() {
            TranslateMessage(&message);
            DispatchMessageA(&message);
        }
    }

    unsafe {
        if !MagUninitialize().as_bool() {
            bail!("MagUninitialize() failed")
        }
    }

    Ok(())
}

fn register_and_create_window() -> AnyResult<HWND> {
    unsafe { register_and_create_window_unsafe() }
}

unsafe fn register_and_create_window_unsafe() -> AnyResult<HWND> {
    let instance = GetModuleHandleA(None)?;
    debug_assert!(instance.0 != 0);

    let window_class = s!("MagnifierWindow");

    let wc = WNDCLASSA {
        hCursor: LoadCursorW(None, IDC_ARROW)?,
        hInstance: instance,
        lpszClassName: window_class,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        ..Default::default()
    };

    let atom = RegisterClassA(&wc);
    debug_assert!(atom != 0);

    // 查找系统放大镜窗口
    let magnifier_hwnd = magnify::find_magnify_window();

    magnify::hide_magnify_ui_window();
    magnify::hide_magnify_window();

    // let style = WINDOW_EX_STYLE::default();
    let style = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW;
    let hwnd = CreateWindowExA(
        style,
        window_class,
        s!("Screen Magnifier"),
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        Some(magnifier_hwnd),
        None,
        instance,
        None,
    );
    debug_assert!(hwnd.0 != 0);

    Ok(hwnd)
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_PAINT => {
                println!("WM_PAINT");
                ValidateRect(window, None);
                LRESULT(0)
            }
            WM_DESTROY => {
                println!("WM_DESTROY");
                let console_hwnd = GetConsoleWindow();
                if console_hwnd != HWND(0) {
                    CloseWindow(console_hwnd);
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}
