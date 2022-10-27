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

mod process_waiter {
    use windows::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_SUCCESS, WAIT_OBJECT_0, WIN32_ERROR},
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    };

    pub fn on_process_terminate<F>(pid: u32, cb: F)
    where
        F: FnOnce(WIN32_ERROR) + Sync + Send + 'static,
    {
        std::thread::spawn(move || unsafe {
            let process =
                OpenProcess(PROCESS_SYNCHRONIZE, false, pid).expect("Cannot open parent process");
            if !process.is_invalid() {
                const INFINITE: u32 = 0xFFFFFFFFu32;
                if WaitForSingleObject(process, INFINITE) == WAIT_OBJECT_0 {
                    CloseHandle(process);
                    cb(ERROR_SUCCESS);
                } else {
                    CloseHandle(process);
                    cb(GetLastError());
                }
            } else {
                cb(GetLastError());
            }
        });
    }
}

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

/// 沉浸模式
/// 使用 Magnify 的特殊窗口特性

fn immersion_main() -> Result<()> {
    let mut opt = Opt::from_args();

    if let Some(pid) = opt.pid.take() {
        process_waiter::on_process_terminate(pid, |err_code| {
            std::process::exit(err_code.0 as i32);
        });
    }

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

    // Create the runtime
    let rt = Runtime::new()?;

    let mut hwnds = Vec::<HWND>::with_capacity(32);

    // Window Filter
    unsafe {
        // keybd_event(VK_LWIN as u8, 0x45, KEYBD_EVENT_FLAGS::default(), 0); //show start menu
        // keybd_event(VK_LWIN as u8, 0x45, KEYEVENTF_KEYUP, 0);

        std::thread::sleep(std::time::Duration::from_millis(100));

        let wins = enumerate_all_windows();

        // hwnds.push(GetDesktopWindow());

        // let mut hwnds_guard = hwnds;
        for win in wins.into_iter() {
            if win.title.contains("DebugView") {
                watcher.set_topmost_app_hwnd(win.handle);
                // continue;
            }
            info!(">> Hide {:?}", win.title);
            if win
                .title
                .contains("ThinkbookEinkPlus2A7678FA-39DD-4C1D-8981-34A451919F59")
            {
                continue;
            }
            hwnds.push(win.handle);
        }

        hwnds.push(FindWindowA(s!("Shell_TrayWnd"), None));
        hwnds.push(FindWindowA(s!("TaskListOverlayWnd"), None));

        info!("MagSetWindowFilterList: count: {:?}", hwnds.len() as i32);

        MagSetWindowFilterList(
            mag.get_handle(),
            MW_FILTERMODE_EXCLUDE,
            hwnds.len() as i32,
            hwnds.as_mut_ptr(),
        );
    }

    let mag_hwnd = mag.get_handle();

    // 保存到全局变量
    (*g_mag.write().get_mut()) = Some(mag);

    rt.spawn(async move {
        // Create a new hook
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
        let hook = WindowEventHook::hook(EventFilter::default().events(SYSTEM_FOREGROUND .. OBJECT_CREATE), event_tx)
            .await
            .unwrap();

        // Wait and print events
        while let Some(event) = event_rx.recv().await {
            use wineventhook::MaybeKnown::Known;

            //
            // 窗口被置为前台
            // 1. 如果不是 Launcher 和被选择的应用，移动到后台
            // 2. 
            //
            if event.raw.event_id as i32 == SYSTEM_FOREGROUND {
                info!("WinEvent: SYSTEM_FOREGROUND");
                let hwnd = HWND(event.raw.window_handle as isize);
                if unsafe { IsWindowVisible(hwnd).as_bool() } {
                    let win_classname = eink_winkits::get_window_class(hwnd).unwrap();
                    let win_real_classname = eink_winkits::get_window_real_class(hwnd).unwrap();
                    let win_title = eink_winkits::get_window_text(hwnd).unwrap();
                    info!("WinEvent: SYSTEM_FOREGROUND {win_classname}, {win_real_classname}, {win_title}");

                    // [22504] INFO  [eink_service_helper] WinEvent: SYSTEM_FOREGROUND XamlExplorerHostIslandWindow, XamlExplorerHostIslandWindow, 任务切换

                    if win_classname == "Windows.UI.Core.CoreWindow" || win_classname == "XamlExplorerHostIslandWindow" {
                        // let hwnd = unsafe { GetAncestor(hwnd, GA_ROOT) };
                        // hwnds.push(hwnd);
                        info!("MagSetWindowFilterList: count: {:?}", hwnds.len() as i32);
                        unsafe {
                            MagSetWindowFilterList(
                                mag_hwnd,
                                MW_FILTERMODE_EXCLUDE,
                                hwnds.len() as i32,
                                hwnds.as_mut_ptr(),
                            );
                            refresh_magnifier();
                        }
                    }
                    // [6236] INFO  [eink_service_helper] WinEvent: SYSTEM_FOREGROUND Windows.UI.Core.CoreWindow, Windows.UI.Core.CoreWindow, 搜索
                }
            }
            else if let Known(object_type) = event.object_type() {
                if object_type == AccessibleObjectId::Window {
                    let hwnd = HWND(event.raw.window_handle as isize);
                    let win_classname = eink_winkits::get_window_class(hwnd).unwrap();
                    let win_real_classname = eink_winkits::get_window_real_class(hwnd).unwrap();
                    let win_title = eink_winkits::get_window_text(hwnd).unwrap();
                    info!("WinEvent: Create Window {win_classname}, {win_real_classname}, {win_title}");

                    // [15052] INFO  [eink_service_helper] WinEvent: Create Window OleMainThreadWndClass, OleMainThreadWndClass, OLEChannelWnd

                    // [20172] INFO  [eink_service_helper] WinEvent: Create Window OleMainThreadWndName

                    // [22488] INFO  [eink_service_helper] WinEvent: Create Window TaskListOverlayWnd, TaskListOverlayWnd, 

                    if win_classname == "Windows.UI.Core.CoreWindow" || win_classname == "TaskListOverlayWnd" || win_classname == "OleMainThreadWndName" {
                        hwnds.push(hwnd);
                        info!("MagSetWindowFilterList: count: {:?}", hwnds.len() as i32);
                        unsafe {
                            MagSetWindowFilterList(
                                mag_hwnd,
                                MW_FILTERMODE_EXCLUDE,
                                hwnds.len() as i32,
                                hwnds.as_mut_ptr(),
                            )
                        };
                    }
                }
            }
        }

        // Unhook the hook
        hook.unhook().await.unwrap();
    });

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

    // 开启热键响应线程
    std::thread::spawn(move || {
        let mut hkm = HotkeyManager::new();

        // ALT-A 退出
        hkm.register(VKey::A, &[ModKey::Alt], move || {
            unsafe { PostMessageA(host_hwnd, WM_QUIT, WPARAM(0), LPARAM(0)) };
        })
        .expect("Cannot register hot-key ALT-A");

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

    // 进入窗口循环
    unsafe {
        ShowWindow(host_hwnd, SW_SHOWNOACTIVATE);

        let mut message = MSG::default();

        while GetMessageA(&mut message, None, 0, 0).as_bool() {
            TranslateMessage(&message);
            DispatchMessageA(&message);
        }
    }

    magnify::close_magnify_window();

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
    let mut magnifier_hwnd = magnify::find_magnify_window();

    if magnifier_hwnd == HWND(0) {
        magnify::start_magnify();
        magnifier_hwnd = magnify::find_magnify_window();
    }

    magnify::hide_magnify_ui_window();
    magnify::hide_magnify_window();

    let style = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW;
    let hwnd = CreateWindowExA(
        style,
        window_class,
        s!("Screen Magnifier"),
        WS_POPUP | WS_VISIBLE,
        100,
        -50,
        2560,
        1600,
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
