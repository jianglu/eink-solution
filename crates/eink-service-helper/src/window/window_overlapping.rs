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

use log::info;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    Graphics::Gdi::IntersectRect,
    System::{Console::GetConsoleWindow, StationsAndDesktops::EnumDesktopWindows},
    UI::WindowsAndMessaging::{GetParent, GetWindowRect, IsIconic, IsWindowVisible},
};

use super::WindowInfo;

struct WindowEnumerationState {
    windows: Vec<WindowInfo>,
    console_window: Option<HWND>,
    test_hwnd: HWND,
    test_rect: RECT,
}

/// 枚举所有和目标区域重合的窗口
pub fn enumerate_overlapping_windows(test_hwnd: HWND) -> Vec<WindowInfo> {
    unsafe {
        // TODO: This works for Command Prompt but not Terminal
        let console_window = {
            let window_handle = GetConsoleWindow();
            if window_handle.0 == 0 {
                None
            } else {
                Some(window_handle)
            }
        };

        // 获取被测试的区域
        let mut test_rect: RECT = Default::default();
        GetWindowRect(test_hwnd, &mut test_rect);

        let state = Box::into_raw(Box::new(WindowEnumerationState {
            windows: Vec::new(),
            console_window,
            test_hwnd,
            test_rect,
        }));

        // 遍历所有桌面窗口
        EnumDesktopWindows(None, Some(enum_overlapping_window), LPARAM(state as isize));

        let state = Box::from_raw(state);
        state.windows
    }
}

extern "system" fn enum_overlapping_window(window: HWND, state: LPARAM) -> BOOL {
    unsafe {
        let state = Box::leak(Box::from_raw(state.0 as *mut WindowEnumerationState));

        if let Some(console_window) = &state.console_window {
            if window == *console_window {
                return true.into();
            }
        }

        // 到达目标窗口，z-order 遍历结束
        if window == state.test_hwnd {
            return false.into();
        }

        // 必须要可见
        if !IsWindowVisible(window).as_bool() {
            return true.into();
        }

        // 必须没有父窗口
        if GetParent(window) != HWND(0) {
            return true.into();
        }

        // 获得窗口尺寸
        let mut rc: RECT = RECT::default();
        GetWindowRect(window, &mut rc);

        // 跳过窗口尺寸为空的目标
        if rc.bottom == 0 && rc.left == 0 && rc.right == 0 && rc.top == 0 {
            return true.into();
        }

        // 跳过最小化的窗口
        if IsIconic(window).as_bool() {
            return true.into();
        }

        // 选择和目标窗口矩形相交的
        let mut rc_tmp: RECT = RECT::default();
        if IntersectRect(&mut rc_tmp, &state.test_rect, &rc).as_bool() {
            //found an intersection - but we dont know
            //whether this window is on top of ours or not
            let window_info = WindowInfo::new(window);

            // Desktop
            if window_info.class_name == "SysListView32" && window_info.title == "FolderView" {
                info!("Found DESKTOP !!!!");
            }

            state.windows.push(window_info);
        }
    }
    true.into()
}
