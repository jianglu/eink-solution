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

use std::{collections::HashSet, ffi::CStr};

use widestring::{U16CStr, U16CString};
use windows::{
    core::{PCWSTR, PWSTR},
    w,
    Graphics::Capture::{GraphicsCaptureItem, IGraphicsCaptureItem},
    Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        Storage::FileSystem::STANDARD_RIGHTS_REQUIRED,
        System::{
            StationsAndDesktops::{
                CloseDesktop, CreateDesktopW, EnumDesktopWindows, EnumDesktopsW, GetThreadDesktop,
                OpenDesktopW, SwitchDesktop, DF_ALLOWOTHERACCOUNTHOOK,
            },
            SystemServices::{
                DESKTOP_ACCESS_FLAGS, DESKTOP_CREATEMENU, DESKTOP_CREATEWINDOW, DESKTOP_ENUMERATE,
                DESKTOP_HOOKCONTROL, DESKTOP_JOURNALPLAYBACK, DESKTOP_JOURNALRECORD,
                DESKTOP_READOBJECTS, DESKTOP_SWITCHDESKTOP, DESKTOP_WRITEOBJECTS,
            },
            Threading::{
                CreateProcessW, GetCurrentThreadId, CREATE_NEW_CONSOLE, NORMAL_PRIORITY_CLASS,
                PROCESS_INFORMATION, STARTUPINFOW,
            },
            WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
        },
        UI::WindowsAndMessaging::{
            CloseWindow, EnumWindows, GetAncestor, GetClassNameA, GetDesktopWindow, GetWindowLongA,
            GetWindowLongW, GetWindowTextA, GetWindowTextW, GetWindowThreadProcessId,
            PostQuitMessage, PostThreadMessageA, RealGetWindowClassA, GA_ROOT, GET_ANCESTOR_FLAGS,
            GWL_STYLE, HCF_DEFAULTDESKTOP, WM_QUIT, WNDENUMPROC, WS_VISIBLE,
        },
    },
};

pub mod process;
pub mod process_waiter;
pub mod taskbar;

pub fn get_window_ancestor(hwnd: HWND) -> anyhow::Result<HWND> {
    unsafe {
        return Ok(GetAncestor(hwnd, GA_ROOT));
    }
}

pub fn get_window_class(hwnd: HWND) -> anyhow::Result<String> {
    unsafe {
        let mut buf: [u8; 256] = std::mem::zeroed();
        GetClassNameA(hwnd, &mut buf);
        let class_name = CStr::from_bytes_with_nul_unchecked(&buf);
        return Ok(class_name
            .to_str()?
            .trim_end_matches(|c: char| c == '\0')
            .to_string());
    }
}

pub fn get_window_real_class(hwnd: HWND) -> anyhow::Result<String> {
    unsafe {
        let mut buf: [u8; 256] = std::mem::zeroed();
        RealGetWindowClassA(hwnd, &mut buf);
        let class_name = CStr::from_bytes_with_nul_unchecked(&buf);
        return Ok(class_name
            .to_str()?
            .trim_end_matches(|c: char| c == '\0')
            .to_string());
    }
}

pub fn get_window_text(hwnd: HWND) -> anyhow::Result<String> {
    unsafe {
        let mut utf16 = vec![0x0u16; 1024];
        GetWindowTextW(hwnd, &mut utf16);
        let title = U16CString::from_vec_unchecked(utf16);
        Ok(title.to_string_lossy())
    }
}

pub fn find_all_windows() -> HashSet<isize> {
    unsafe extern "system" fn enum_hwnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let mut hwnds = Box::from_raw(lparam.0 as *mut HashSet<isize>);

        let hwnd_ancestor = GetAncestor(hwnd, GA_ROOT);
        if hwnd_ancestor == hwnd {
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let visible = (style & WS_VISIBLE.0) == WS_VISIBLE.0;

            if visible {
                hwnds.insert(hwnd.0);
            }
        }

        std::mem::forget(hwnds);
        BOOL(1)
    }

    let boxed_hwnds = Box::new(HashSet::<isize>::new());
    let boxed_hwnds_ptr = Box::into_raw(boxed_hwnds) as isize;

    unsafe {
        EnumWindows(Some(enum_hwnd), LPARAM(boxed_hwnds_ptr));
        let hwnds = Box::from_raw(boxed_hwnds_ptr as *mut HashSet<isize>);
        return *hwnds;
    }
}
