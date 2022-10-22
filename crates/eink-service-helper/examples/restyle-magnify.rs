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

use std::mem::zeroed;

use anyhow::Result;
use windows::{
    core::{IUnknown, PCWSTR},
    s, w,
    Win32::{
        Foundation::{COLORREF, HWND},
        System::Com::{
            self, CLSIDFromString, CoCreateInstance, CoCreateInstanceEx, CoInitializeEx,
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
        },
        UI::{
            Shell::{ITaskbarList, ITaskbarList2, ITaskbarList3, ITaskbarList4},
            WindowsAndMessaging::{
                FindWindowA, GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW,
                GWL_EXSTYLE, GWL_STYLE, LAYERED_WINDOW_ATTRIBUTES_FLAGS, LWA_ALPHA,
                WINDOW_EX_STYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
            },
        },
    },
};

pub fn make_window_invisable(hwnd: HWND) -> Result<()> {
    unsafe {
        let clsid_taskbar_list = CLSIDFromString(w!("{56FDF344-FD6D-11d0-958A-006097C9A090}"))?;

        let taskbar_list: ITaskbarList4 =
            CoCreateInstance(&clsid_taskbar_list, None, CLSCTX_INPROC_SERVER)?;

        // 设置窗口风格 WS_EX_LAYERED + WS_EX_TRANSPARENT
        let exstyle = WINDOW_EX_STYLE(GetWindowLongW(hwnd, GWL_EXSTYLE) as u32);
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            (exstyle.0 | WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0) as i32,
        );

        // 设置透明度w
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA);

        // 从任务栏上删除 Icon
        taskbar_list.DeleteTab(hwnd);
    }

    Ok(())
}

/// 服务助手程序
fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED);
        let magui_hwnd = FindWindowA(s!("MagUIClass"), None);
        make_window_invisable(magui_hwnd);
    }

    Ok(())
}

// CLSID_TaskbarList:TGUID='{56FDF344-FD6D-11d0-958A-006097C9A090}';

// IID_ITaskbarList:TGUID='{602D4995-B13A-429b-A66E-1935E44F4317}';
