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

use windows::{
    core::HSTRING,
    w,
    Win32::{
        Foundation::LPARAM,
        UI::{
            Shell::{SHAppBarMessage, ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE, APPBARDATA},
            WindowsAndMessaging::FindWindowW,
        },
    },
};

const TASKBAR_CLASSNAME: &HSTRING = w!("Shell_TrayWnd");

pub fn set_auto_hide(hide: bool) {
    let mut appbar: APPBARDATA = unsafe { std::mem::zeroed() };
    appbar.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    appbar.hWnd = unsafe { FindWindowW(TASKBAR_CLASSNAME, None) };
    appbar.lParam = LPARAM(if hide { ABS_AUTOHIDE } else { ABS_ALWAYSONTOP } as isize);
    unsafe { SHAppBarMessage(ABM_SETSTATE, &mut appbar) };
}
