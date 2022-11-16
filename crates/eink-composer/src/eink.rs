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

use std::ffi::CStr;

use ntapi::winapi::{
    shared::{
        minwindef::{BYTE, DWORD},
        ntdef::HANDLE,
    },
    um::handleapi::INVALID_HANDLE_VALUE,
};

#[windows_dll::dll(EInkTcon)]
extern "system" {
    #[allow(non_snake_case)]
    pub fn ITEGetDriveNo(drive_no: &mut BYTE) -> DWORD;

    #[allow(non_snake_case)]
    pub fn ITEOpenDeviceAPI(dev_path: &CStr) -> HANDLE;

    #[allow(non_snake_case)]
    pub fn ITESet8951KeepAlive(enable: DWORD) -> DWORD;

    #[allow(non_snake_case)]
    pub fn ITESetMIPIModeAPI(mode: &mut DWORD) -> DWORD;

    #[allow(non_snake_case)]
    pub fn ITEResetTcon() -> DWORD;
}

pub unsafe fn eink_enable() {
    let mut drive_no: BYTE = 0;
    let ret = ITEGetDriveNo(&mut drive_no);
    println!("ret: {}, drive_no: {}", ret, drive_no);

    let dev_path = format!("\\\\.\\{}:", (0x41 + drive_no) as char);
    println!("Dev Path: {}", dev_path);

    let cstr = std::ffi::CString::new(dev_path).unwrap();
    println!("Dev Path C: {:?}", &cstr);

    if ITEOpenDeviceAPI(&cstr) == INVALID_HANDLE_VALUE {
        println!("open eink device fail, in thread");
        return;
    }

    let ret = ITESet8951KeepAlive(1);
    println!("ITESet8951KeepAlive(1): {}", ret);

    let mut mode: DWORD = 1;
    let ret = ITESetMIPIModeAPI(&mut mode);
    println!("ITESetMIPIModeAPI({}): {}", mode, ret);

    mode = 2;
    let ret = ITESetMIPIModeAPI(&mut mode);
    println!("ITESetMIPIModeAPI({}): {}", mode, ret);
}

#[test]
fn test_eink() {
    unsafe { eink_enable() };
}
