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

use std::mem::{zeroed, MaybeUninit};

use log::info;
use windows::{
    w,
    Win32::{
        Foundation::{LPARAM, RECT},
        UI::{
            Shell::{
                SHAppBarMessage, ABE_BOTTOM, ABM_QUERYPOS, ABM_SETAUTOHIDEBAR, ABM_SETPOS,
                ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE, APPBARDATA,
            },
            WindowsAndMessaging::{
                FindWindowW, GetWindowLongW, GetWindowRect, IsWindowVisible,
                SetLayeredWindowAttributes, ShowWindow, SystemParametersInfoW, GWL_EXSTYLE,
                GWL_STYLE, LWA_ALPHA, SHOW_WINDOW_CMD, SPIF_SENDCHANGE, SPI_GETWORKAREA,
                SPI_SETWORKAREA, SW_FORCEMINIMIZE, SW_HIDE, SW_SHOW,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_LONG_PTR_INDEX,
            },
        },
    },
};

pub fn hide_taskbar() {
    let hwnd = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };

    let mut data: APPBARDATA = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    data.hWnd = hwnd;
    data.lParam = LPARAM(ABS_AUTOHIDE as isize);
    unsafe { SHAppBarMessage(ABM_SETSTATE, &mut data) };

    // 重试 10 次左右是必须的，但是
    // 
    for _i in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(10));

        unsafe { ShowWindow(hwnd, SW_HIDE) };
        unsafe { SetLayeredWindowAttributes(hwnd, None, 0, LWA_ALPHA) };
    }
}

pub fn show_taskbar() {
    let mut data: APPBARDATA = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    data.hWnd = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };
    data.lParam = LPARAM(ABS_ALWAYSONTOP as isize);
    unsafe { SHAppBarMessage(ABM_SETSTATE, &mut data) };

    unsafe {
        ShowWindow(data.hWnd, SW_SHOW);
        SetLayeredWindowAttributes(data.hWnd, None, 2, LWA_ALPHA);
    }
}
