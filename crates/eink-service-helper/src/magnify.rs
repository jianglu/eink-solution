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

// 放大镜管理模块

use windows::{
    s,
    Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{FindWindowA, ShowWindowAsync, SHOW_WINDOW_CMD, SW_HIDE},
    },
};

struct Magnify {}

/// 查找放大镜窗口
pub fn find_magnify_window() -> HWND {
    unsafe { FindWindowA(s!("Screen Magnifier Window"), None) }
}

/// 查找放大镜 UI 窗口
pub fn find_magnify_ui_window() -> HWND {
    unsafe { FindWindowA(s!("MagUIClass"), None) }
}

/// 隐藏放大镜窗口
pub fn hide_magnify_window() -> bool {
    let hwnd = find_magnify_window();
    if hwnd != HWND(0) {
        unsafe { ShowWindowAsync(hwnd, SW_HIDE) };
        true
    } else {
        false
    }
}

/// 隐藏放大镜 UI 窗口
pub fn hide_magnify_ui_window() -> bool {
    let hwnd = find_magnify_ui_window();
    if hwnd != HWND(0) {
        unsafe { ShowWindowAsync(hwnd, SW_HIDE) };
        true
    } else {
        false
    }
}
