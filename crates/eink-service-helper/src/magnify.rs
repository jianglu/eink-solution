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

use cmd_lib::run_cmd;
use windows::{
    s,
    Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            CloseWindow, FindWindowA, ShowWindowAsync, SHOW_WINDOW_CMD, SW_HIDE,
        },
    },
};

struct Magnify {}

// 启动放大镜程序
pub fn start_magnify() {
    // let mut magnify_cmd = Command::new();
    // let magnify_proc = magnify_cmd.spawn().unwrap();

    run_cmd!(powershell -Command "magnify.exe").expect("Cannot start magnify");

    for _i in 0..10 {
        let magnify_ui = find_magnify_ui_window();
        log::info!("magnify_ui: {magnify_ui:?}");

        if magnify_ui != HWND(0) {
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

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

/// 隐藏放大镜窗口
pub fn close_magnify_window() -> bool {
    let hwnd = find_magnify_ui_window();
    if hwnd != HWND(0) {
        unsafe { CloseWindow(hwnd) };
        true
    } else {
        false
    }
}
