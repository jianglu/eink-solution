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

use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    sync::{Arc, Mutex},
};

use anyhow::{bail, Ok};
use eink_pipe_io::blocking::BlockingIpcConnection;
use jsonrpc_lite::{Error, JsonRpc};
use mag_win::MagWindow;
use static_init::dynamic;
use windows::{
    core::{HSTRING, PCWSTR, PWSTR},
    w,
    Win32::{
        Foundation::{
            BOOL, COLORREF, FILETIME, HINSTANCE, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM,
        },
        Graphics::Gdi::{UpdateWindow, COLOR_BTNFACE, HBRUSH},
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{
                CreateThreadpoolTimer, SetThreadpoolTimer, TP_CALLBACK_INSTANCE, TP_TIMER,
            },
        },
        UI::{
            HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
            Magnification::MagInitialize,
            Shell::{
                SHAppBarMessage, Shell_NotifyIconW, StrCpyW, ABM_SETSTATE, ABS_ALWAYSONTOP,
                ABS_AUTOHIDE, APPBARDATA, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_SETVERSION,
                NOTIFYICONDATAW, NOTIFYICON_VERSION,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowA,
                FindWindowW, GetCursorPos, GetMessageW, GetSystemMetrics, LoadIconW, MoveWindow,
                RegisterClassExW, SetLayeredWindowAttributes, SetWindowPos, SetWindowsHookExW,
                ShowWindow, TranslateMessage, UnhookWindowsHookEx, CS_HREDRAW, CS_VREDRAW, HCURSOR,
                HHOOK, HICON, HWND_TOPMOST, LWA_ALPHA, MSG, SM_CXSCREEN, SM_CYSCREEN,
                SWP_NOACTIVATE, SWP_NOREDRAW, SWP_NOSIZE, SW_HIDE, SW_RESTORE, WH_KEYBOARD_LL,
                WM_DESTROY, WM_QUERYENDSESSION, WM_USER, WNDCLASSEXW, WS_CLIPCHILDREN,
                WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
            },
        },
    },
};

mod helper;
mod mag_win;
mod magnify;

// Show magnifier or not
static mut ENABLED: bool = false;

static mut HKB: HHOOK = HHOOK(0);
// KBDLLHOOKSTRUCT* key;
// BOOL                wkDown = FALSE;

// lens pan offset x|y
static mut PAN_OFFSET: POINT = POINT { x: 0, y: 0 };

/// 服务助手程序
/// 在 admin 权限下运行，负责 system 权限无法进行的操作
fn main() -> anyhow::Result<()> {
    // 设置当前的活动日志系统为 OutputDebugString 输出
    eink_logger::init_with_level(log::Level::Trace)?;

    // 启动和 eink-service 的通讯进程
    helper::start_helper_thread();

    // 设置进程 DPI
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };

    unsafe { init_screen_dimensions() };

    if unsafe { !MagInitialize().as_bool() } {
        bail!("MagInitialize() failed");
    }

    let instance = unsafe { GetModuleHandleW(None).unwrap() };

    if unsafe { setup_host_window(instance.clone()).is_err() } {
        bail!("setup_host_window() failed");
    }

    if unsafe { setup_magnifier_window(instance.clone()).is_err() } {
        bail!("SetupMagnifierWindow() failed");
    }

    if unsafe { !UpdateWindow(HWND_HOST).as_bool() } {
        bail!("UpdateWindow(HWND_HOST) failed");
    }

    // Start as disabled
    unsafe { ShowWindow(HWND_HOST, SW_HIDE) };
    unsafe { ENABLED = false };

    // Create notification object for the task tray icon
    unsafe {
        let mut nid: NOTIFYICONDATAW = zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.Anonymous.uVersion = NOTIFYICON_VERSION;
        nid.hWnd = HWND_HOST;
        nid.uID = 0;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_USER;
        nid.hIcon = HICON(0);

        StrCpyW(
            PWSTR::from_raw(nid.szTip.as_mut_ptr()),
            PCWSTR::from_raw(w!("Magnify10 (Click to Close)").as_ptr()),
        );

        // Add icon to the task tray
        Shell_NotifyIconW(NIM_ADD, &nid);
        Shell_NotifyIconW(NIM_SETVERSION, &nid);
    }

    // Setup the keyboard hook to capture global hotkeys
    unsafe {
        HKB = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(LowLevelKeyboardProc),
            instance.clone(),
            0,
        )?;
    }

    // Create and start a timer to refresh the window.
    unsafe {
        refreshTimer = CreateThreadpoolTimer(Some(TimerTickEvent), None, None) as isize;
    }

    // TODO: this only needs to be started if enabled at start
    unsafe { SetThreadpoolTimer(refreshTimer as *mut TP_TIMER, Some(&timerDueTime), 0, 0) };

    // Main message loop.
    unsafe {
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            TranslateMessage(&mut msg);
            DispatchMessageW(&mut msg);
        }
    }

    // Shut down.
    unsafe { ENABLED = false };

    unsafe { UnhookWindowsHookEx(HKB) };
    // hkb = HHOOK(0);

    // delete hkb;
    // key = 0;
    // delete key;

    // SetThreadpoolTimer(refreshTimer, nullptr, 0, 0);
    // Shell_NotifyIcon(NIM_DELETE, &nid);
    // MagUninitialize();
    // DestroyWindow(mag1.GetHandle());
    // DestroyWindow(mag2.GetHandle());
    // DestroyWindow(hwndHost);

    // return (int)msg.wParam;
    Ok(())
}

// Calculates an X or Y value where the lens (host window) should be relative to mouse position. i.e. top left corner of a window centered on mouse
fn lens_position_value(mousepoint_value: i32, lenssize_value: i32) -> i32 {
    mousepoint_value - (lenssize_value / 2) - 1
}

// Calculates a lens size value that is slightly larger than (lens + increment) to give an extra buffer area on the edges
fn lens_size_buffer_value(lens_size_value: i32, resize_increment_value: i32) -> i32 {
    lens_size_value + (2 * resize_increment_value)
}

static mut mag1: Option<MagWindow> = None;
// static mag2: Option<MagWindow> = None;

unsafe fn setup_magnifier_window(hInst: HINSTANCE) -> anyhow::Result<()> {
    let mut magSize: SIZE = SIZE { cx: 0, cy: 0 };

    magSize.cx = lens_size_buffer_value(LENS_SIZE.cx, RESIZE_INCREMENT.cx);
    magSize.cy = lens_size_buffer_value(LENS_SIZE.cy, RESIZE_INCREMENT.cy);

    // position in the host window coordinates - top left corner
    let mut magPosition: POINT = POINT { x: 0, y: 0 };
    magPosition.x = 0;
    magPosition.y = 0;

    mag1 = Some(MagWindow::new(magnificationFactor, magPosition, magSize).unwrap());
    if !mag1
        .as_mut()
        .unwrap()
        .create(hInst.clone(), HWND_HOST, true)
    {
        bail!("Cannot create magnify window")
    }

    // mag2 = Some(MagWindow::new(magnificationFactor, magPosition, magSize).unwrap());
    // if !mag2.as_mut().unwrap().Create(hInst.clone(), HWND_HOST, false) {
    //     return false;
    // }

    // magActive = &mag1;

    Ok(())
}

unsafe extern "system" fn LowLevelKeyboardProc(
    nCode: i32,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    LRESULT(0)
}

// // Timer interval structures
// union FILETIME64
// {
// 	INT64 quad;
// 	FILETIME ft;
// };

// FILETIME CreateRelativeFiletimeMS(DWORD milliseconds)
// {
// 	FILETIME64 ft = { -static_cast<INT64>(milliseconds) * 10000 };
// 	return ft.ft;
// }

static timerDueTime: FILETIME = FILETIME {
    dwLowDateTime: 0,
    dwHighDateTime: 0,
};

// = CreateRelativeFiletimeMS(TIMER_INTERVAL_MS);

static mut refreshTimer: isize = 0;

unsafe extern "system" fn TimerTickEvent(
    _: *mut TP_CALLBACK_INSTANCE,
    context: *mut c_void,
    _: *mut TP_TIMER,
) {
    refresh_magnifier();

    // Reset timer to expire one time at next interval
    if ENABLED {
        SetThreadpoolTimer(refreshTimer as *mut TP_TIMER, Some(&timerDueTime), 0, 0);
    }
}

static mut magnificationFactor: f32 = 2.0f32;
static mut newMagnificationFactor: f32 = 2.0f32; // Temp mag factor to store change during update

// MagWindow           mag1;
// MagWindow           mag2;
// MagWindow* magActive;

/// Called in the timer tick event to refresh the magnification area drawn and lens (host window) position and size
unsafe fn refresh_magnifier() {
    GetCursorPos(&mut MOUSE_POINT);

    if LENS_SIZE.cx != NEW_LENS_SIZE.cx || LENS_SIZE.cy != NEW_LENS_SIZE.cy
    // lens size has changed - do update
    {
        LENS_SIZE = NEW_LENS_SIZE;
        rotate_mag_window(LENS_SIZE, magnificationFactor);

        update_lens_position(&mut MOUSE_POINT);

        SetWindowPos(
            HWND_HOST,
            HWND_TOPMOST,
            LENS_POSITION.x,
            LENS_POSITION.y, // x|y coordinate of top left corner
            LENS_SIZE.cx,
            LENS_SIZE.cy, // width|height of window
            SWP_NOACTIVATE | SWP_NOREDRAW,
        );

        // magActive->UpdateMagnifier(&mousePoint, panOffset, lensSize);
        // Exit early to avoid updating once more below
        return;
    } else if magnificationFactor != newMagnificationFactor {
        unsafe { magnificationFactor = newMagnificationFactor };
        rotate_mag_window(LENS_SIZE, magnificationFactor);
    }

    // TODO why does this else cause problems?
    {
        // magActive->UpdateMagnifier(&mousePoint, panOffset, lensSize);
        mag1.as_mut()
            .unwrap()
            .update_magnifier(&mut MOUSE_POINT, PAN_OFFSET, LENS_SIZE);
    }

    if update_lens_position(&mut MOUSE_POINT) {
        SetWindowPos(
            HWND_HOST,
            HWND_TOPMOST,
            LENS_POSITION.x,
            LENS_POSITION.y, // x|y coordinate of top left corner
            LENS_SIZE.cx,
            LENS_SIZE.cy, // width|height of window
            SWP_NOACTIVATE | SWP_NOREDRAW | SWP_NOSIZE,
        );
    }
}

unsafe fn rotate_mag_window(newSize: SIZE, newMagFactor: f32) {
    // if (magActive == &mag1)
    // {
    // UpdateMagWindow(&mag2, newSize, newMagFactor);
    // ReassignActiveMag(&mag2, &mag1);
    // }
    // else
    // {
    update_mag_window(mag1.as_mut().unwrap(), newSize, newMagFactor);
    // ReassignActiveMag(&mag1, &mag2);
    // }
}

fn update_mag_window(mag: &mut MagWindow, new_size: SIZE, new_mag_factor: f32) {
    unsafe {
        mag.set_magnification_factor(new_mag_factor);
        mag.update_magnifier(&mut MOUSE_POINT, PAN_OFFSET, new_size);
        mag.set_size(
            lens_size_buffer_value(new_size.cx, RESIZE_INCREMENT.cx),
            lens_size_buffer_value(new_size.cy, RESIZE_INCREMENT.cy),
        );
    }
}

// fn ReassignActiveMag(active: &mut MagWindow, MagWindow* backup)
// {
//     SetWindowPos(active->GetHandle(), HWND_TOP, 0, 0, 0, 0,
//         SWP_SHOWWINDOW | SWP_NOSIZE | SWP_NOMOVE | SWP_NOREDRAW);

//     magActive = active;
//     ShowWindow(backup->GetHandle(), SW_HIDE);
// }

pub static mut SCREEN_SIZE: SIZE = SIZE { cx: 0, cy: 0 };
pub static mut LENS_SIZE: SIZE = SIZE { cx: 0, cy: 0 };
pub static mut NEW_LENS_SIZE: SIZE = SIZE { cx: 0, cy: 0 };
pub static mut LENS_POSITION: POINT = POINT { x: 0, y: 0 };

pub static mut RESIZE_INCREMENT: SIZE = SIZE { cx: 0, cy: 0 };
pub static mut RESIZE_LIMIT: SIZE = SIZE { cx: 0, cy: 0 };

// lens sizing factors as a percent of screen resolution
const INIT_LENS_WIDTH_FACTOR: f32 = 0.5f32;
const INIT_LENS_HEIGHT_FACTOR: f32 = 0.5f32;
const INIT_LENS_RESIZE_HEIGHT_FACTOR: f32 = 0.0625f32;
const INIT_LENS_RESIZE_WIDTH_FACTOR: f32 = 0.0625f32;
const LENS_MAX_WIDTH_FACTOR: f32 = 1.2f32;
const LENS_MAX_HEIGHT_FACTOR: f32 = 1.2f32;

// Set initial values for screen, lens, and resizing dimensions.
unsafe fn init_screen_dimensions() {
    SCREEN_SIZE.cx = GetSystemMetrics(SM_CXSCREEN);
    SCREEN_SIZE.cy = GetSystemMetrics(SM_CYSCREEN);

    LENS_SIZE.cx = (SCREEN_SIZE.cx as f32 * INIT_LENS_WIDTH_FACTOR) as i32;
    LENS_SIZE.cy = (SCREEN_SIZE.cy as f32 * INIT_LENS_HEIGHT_FACTOR) as i32;
    NEW_LENS_SIZE = LENS_SIZE; // match initial value

    RESIZE_INCREMENT.cx = (SCREEN_SIZE.cx as f32 * INIT_LENS_RESIZE_WIDTH_FACTOR) as i32;
    RESIZE_INCREMENT.cy = (SCREEN_SIZE.cy as f32 * INIT_LENS_RESIZE_HEIGHT_FACTOR) as i32;

    RESIZE_LIMIT.cx = (SCREEN_SIZE.cx as f32 * LENS_MAX_WIDTH_FACTOR) as i32;
    RESIZE_LIMIT.cy = (SCREEN_SIZE.cy as f32 * LENS_MAX_HEIGHT_FACTOR) as i32;
}

// Current mouse location
static mut MOUSE_POINT: POINT = POINT { x: 0, y: 0 };

fn register_host_window_class(inst: HINSTANCE) -> u16 /*ATOM*/ {
    let class_name = &HSTRING::from("MagnifierWindow");

    let mut wc: WNDCLASSEXW = unsafe { zeroed() };
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.style = CS_HREDRAW | CS_VREDRAW;
    wc.lpfnWndProc = Some(host_wnd_proc);
    wc.hInstance = inst;
    wc.hCursor = HCURSOR(0); // LoadCursor(nullptr, IDC_ARROW);
    wc.hbrBackground = HBRUSH((1i32 + COLOR_BTNFACE.0) as isize);
    wc.lpszClassName = PCWSTR::from(class_name);

    unsafe { RegisterClassExW(&wc) }
}

// Window handles
static mut HWND_HOST: HWND = HWND(0);

unsafe fn setup_host_window(inst: HINSTANCE) -> anyhow::Result<()> {
    GetCursorPos(&mut MOUSE_POINT);
    update_lens_position(&mut MOUSE_POINT);

    // Create the host window.
    register_host_window_class(inst);

    // 查找系统放大镜窗口
    let magni_hwnd = FindWindowW(w!("Screen Magnifier Window"), None);

    // WS_EX_LAYERED: Required style to render the magnification correctly
    // WS_EX_TOPMOST: Always-on-top
    // WS_EX_TRANSPARENT: Click-through
    // WS_EX_TOOLWINDOW: Do not show program on taskbar

    // 寄生在系统 Magnify 窗口中

    // WS_POPUP: Removes titlebar and borders - simply a bare window
    // WS_BORDER: Adds a 1-pixel border for tracking the edges - aesthetic
    unsafe {
        HWND_HOST = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            PCWSTR::from(w!("MagnifierWindow")),
            PCWSTR::from(w!("Screen Magnifier")),
            WS_CLIPCHILDREN | WS_POPUP,
            LENS_POSITION.x,
            LENS_POSITION.y,
            LENS_SIZE.cx,
            LENS_SIZE.cy,
            magni_hwnd,
            None,
            inst,
            None,
        );
    }

    // 隐藏系统 Magnify 窗口
    MoveWindow(magni_hwnd, -1000, -1000, 1, 1, true);
    // SetLayeredWindowAttributes(magni_hwnd, 0, 0, LWA_ALPHA);

    if HWND_HOST == HWND(0) {
        bail!("Cannot create HWND_HOST")
    }

    let mut myWindow: HWND = HWND(0); //Handle to my application window
    let mut externalWindow: HWND = HWND(0); //Handle to external application window

    // Make the window fully opaque.
    SetLayeredWindowAttributes(HWND_HOST, COLORREF(0), 255, LWA_ALPHA);

    Ok(())
}

fn update_lens_position(mousePosition: &mut POINT) -> bool {
    unsafe {
        if LENS_POSITION.x == lens_position_value(mousePosition.x, LENS_SIZE.cx)
            && LENS_POSITION.y == lens_position_value(mousePosition.y, LENS_SIZE.cy)
        {
            return false; // No change needed
        }

        LENS_POSITION.x = lens_position_value(mousePosition.x, LENS_SIZE.cx);
        LENS_POSITION.y = lens_position_value(mousePosition.y, LENS_SIZE.cy);
        // Values were changed
        true
    }
}

unsafe extern "system" fn host_wnd_proc(
    hWnd: HWND,
    message: u32,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    match message {
        WM_USER => (), // Exit on task tray icon click - very simple exit functionality
        // switch (lParam)
        // {
        // case WM_LBUTTONUP:
        // 	PostMessage(hwndHost, WM_CLOSE, 0, 0);
        // 	break;
        // case WM_RBUTTONUP:
        // 	PostMessage(hwndHost, WM_CLOSE, 0, 0);
        // 	break;
        WM_QUERYENDSESSION => (),
        // PostMessage(hwndHost, WM_DESTROY, 0, 0);
        // break;
        WM_CLOSE => (),
        // PostMessage(hwndHost, WM_DESTROY, 0, 0);
        // break;
        WM_DESTROY => {
            ENABLED = false;
            ()
            // PostQuitMessage(0);
            // break;
        }
        _ => {
            return DefWindowProcW(hWnd, message, wParam, lParam);
        }
    }
    return LRESULT(0);
}
