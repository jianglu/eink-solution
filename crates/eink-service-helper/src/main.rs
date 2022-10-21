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
    mem::zeroed,
    sync::{Arc, Mutex},
};

use anyhow::{bail, Ok};
use eink_pipe_io::blocking::BlockingIpcConnection;
use jsonrpc_lite::{Error, JsonRpc};
use static_init::dynamic;
use windows::{
    core::{PCWSTR, PWSTR},
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
mod magnify;

// Show magnifier or not
static mut ENABLED: bool = false;

static mut hkb: HHOOK = HHOOK(0);
// KBDLLHOOKSTRUCT* key;
// BOOL                wkDown = FALSE;

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
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
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
        hkb = SetWindowsHookExW(
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

    unsafe { UnhookWindowsHookEx(hkb) };
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

fn setup_magnifier_window(clone: HINSTANCE) -> anyhow::Result<()> {

    SIZE magSize;
	magSize.cx = LENS_SIZE_BUFFER_VALUE(lensSize.cx, resizeIncrement.cx);
	magSize.cy = LENS_SIZE_BUFFER_VALUE(lensSize.cy, resizeIncrement.cy);

    POINT magPosition; // position in the host window coordinates - top left corner
	magPosition.x = 0;
	magPosition.y = 0;

	mag1 = MagWindow(magnificationFactor, magPosition, magSize);
	if (!mag1.Create(hInst, hwndHost, TRUE))
	{
		return FALSE;
	}

	mag2 = MagWindow(magnificationFactor, magPosition, magSize);
	if (!mag2.Create(hInst, hwndHost, FALSE))
	{
		return FALSE;
	}

	magActive = &mag1;

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
    RefreshMagnifier();

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
unsafe fn refresh_magnifier() -> anyhow::Result<()> {
    GetCursorPos(&mut mousePoint);

    if lensSize.cx != newLensSize.cx || lensSize.cy != newLensSize.cy
    // lens size has changed - do update
    {
        lensSize = newLensSize;
        RotateMagWindow(lensSize, magnificationFactor);

        UpdateLensPosition(&mut mousePoint);

        SetWindowPos(
            HWND_HOST,
            HWND_TOPMOST,
            lensPosition.x,
            lensPosition.y, // x|y coordinate of top left corner
            lensSize.cx,
            lensSize.cy, // width|height of window
            SWP_NOACTIVATE | SWP_NOREDRAW,
        );

        // magActive->UpdateMagnifier(&mousePoint, panOffset, lensSize);
        // Exit early to avoid updating once more below
        return Ok(());
    } else if magnificationFactor != newMagnificationFactor {
        unsafe { magnificationFactor = newMagnificationFactor };
        RotateMagWindow(lensSize, magnificationFactor);
    }

    // TODO why does this else cause problems?
    {
        // magActive->UpdateMagnifier(&mousePoint, panOffset, lensSize);
    }

    if UpdateLensPosition(&mut mousePoint).is_ok() {
        SetWindowPos(
            HWND_HOST,
            HWND_TOPMOST,
            lensPosition.x,
            lensPosition.y, // x|y coordinate of top left corner
            lensSize.cx,
            lensSize.cy, // width|height of window
            SWP_NOACTIVATE | SWP_NOREDRAW | SWP_NOSIZE,
        );
    }

    Ok(())
}

unsafe fn RotateMagWindow(newSize: SIZE, newMagFactor: f32) {
    // if (magActive == &mag1)
    // {
    // 	UpdateMagWindow(&mag2, newSize, newMagFactor);
    // 	ReassignActiveMag(&mag2, &mag1);
    // }
    // else
    // {
    // 	UpdateMagWindow(&mag1, newSize, newMagFactor);
    // 	ReassignActiveMag(&mag1, &mag2);
    // }
}

pub static mut SCREEN_SIZE: SIZE = SIZE { cx: 0, cy: 0 };
pub static mut lensSize: SIZE = SIZE { cx: 0, cy: 0 };
pub static mut newLensSize: SIZE = SIZE { cx: 0, cy: 0 };
pub static mut lensPosition: POINT = POINT { x: 0, y: 0 };

pub static mut resizeIncrement: SIZE = SIZE { cx: 0, cy: 0 };
pub static mut resizeLimit: SIZE = SIZE { cx: 0, cy: 0 };

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

    lensSize.cx = (SCREEN_SIZE.cx as f32 * INIT_LENS_WIDTH_FACTOR) as i32;
    lensSize.cy = (SCREEN_SIZE.cy as f32 * INIT_LENS_HEIGHT_FACTOR) as i32;
    newLensSize = lensSize; // match initial value

    resizeIncrement.cx = (SCREEN_SIZE.cx as f32 * INIT_LENS_RESIZE_WIDTH_FACTOR) as i32;
    resizeIncrement.cy = (SCREEN_SIZE.cy as f32 * INIT_LENS_RESIZE_HEIGHT_FACTOR) as i32;

    resizeLimit.cx = (SCREEN_SIZE.cx as f32 * LENS_MAX_WIDTH_FACTOR) as i32;
    resizeLimit.cy = (SCREEN_SIZE.cy as f32 * LENS_MAX_HEIGHT_FACTOR) as i32;
}

// Current mouse location
static mut mousePoint: POINT = POINT { x: 0, y: 0 };

unsafe fn register_host_window_class(hInstance: HINSTANCE) -> u16 /*ATOM*/ {
    let window_class_name = widestring::U16CString::from_str("MagnifierWindow").unwrap();

    let mut wcex: WNDCLASSEXW = zeroed();
    wcex.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
    wcex.style = CS_HREDRAW | CS_VREDRAW;
    wcex.lpfnWndProc = Some(HostWndProc);
    wcex.hInstance = hInstance;
    wcex.hCursor = HCURSOR(0); // LoadCursor(nullptr, IDC_ARROW);
    wcex.hbrBackground = HBRUSH((1i32 + COLOR_BTNFACE.0) as isize);
    wcex.lpszClassName = PCWSTR::from_raw(window_class_name.as_ptr());

    return RegisterClassExW(&wcex);
}

// Window handles
static mut HWND_HOST: HWND = HWND(0);

unsafe fn setup_host_window(hInst: HINSTANCE) -> anyhow::Result<()> {
    GetCursorPos(&mut mousePoint);
    UpdateLensPosition(&mut mousePoint);

    // Create the host window.
    register_host_window_class(hInst);

    // 查找系统放大镜窗口
    let magni_hwnd = FindWindowW(w!("Screen Magnifier Window"), None);

    // WS_EX_LAYERED: Required style to render the magnification correctly
    // WS_EX_TOPMOST: Always-on-top
    // WS_EX_TRANSPARENT: Click-through
    // WS_EX_TOOLWINDOW: Do not show program on taskbar

    let window_class_name = widestring::U16CString::from_str("MagnifierWindow").unwrap();
    let window_name = widestring::U16CString::from_str("Screen Magnifier").unwrap();

    // 寄生在系统 Magnify 窗口中

    // WS_POPUP: Removes titlebar and borders - simply a bare window
    // WS_BORDER: Adds a 1-pixel border for tracking the edges - aesthetic
    unsafe {
        HWND_HOST = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            PCWSTR::from_raw(window_class_name.as_ptr()),
            PCWSTR::from_raw(window_name.as_ptr()),
            WS_CLIPCHILDREN | WS_POPUP,
            lensPosition.x,
            lensPosition.y,
            lensSize.cx,
            lensSize.cy,
            magni_hwnd,
            None,
            hInst,
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

fn UpdateLensPosition(mousePosition: &mut POINT) -> anyhow::Result<()> {
    // if (lensPosition.x == LENS_POSITION_VALUE(mousePosition->x, lensSize.cx) &&
    // 	lensPosition.y == LENS_POSITION_VALUE(mousePosition->y, lensSize.cy))
    // {
    // 	return FALSE; // No change needed
    // }

    // lensPosition.x = LENS_POSITION_VALUE(mousePosition->x, lensSize.cx);
    // lensPosition.y = LENS_POSITION_VALUE(mousePosition->y, lensSize.cy);
    // Values were changed
    Ok(())
}

unsafe extern "system" fn HostWndProc(
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

        // default:
        // 	return DefWindowProc(hWnd, message, wParam, lParam);
        // }
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
