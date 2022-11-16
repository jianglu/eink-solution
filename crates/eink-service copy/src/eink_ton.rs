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

use log::{debug, info};

use crate::winrt::HANDLE;
use crate::winrt::INVALID_HANDLE_VALUE;

#[windows_dll::dll(EInkTcon)]
extern "system" {
    #[allow(non_snake_case)]
    pub fn ITEGetDriveNo(drive_no: &mut u8) -> u32;

    #[allow(non_snake_case)]
    pub fn ITEOpenDeviceAPI(dev_path: &CStr) -> HANDLE;

    #[allow(non_snake_case)]
    pub fn ITESet8951KeepAlive(enable: u32) -> u32;

    #[allow(non_snake_case)]
    pub fn ITESetMIPIModeAPI(mode: &mut u32) -> u32;

    #[allow(non_snake_case)]
    pub fn ITEResetTcon() -> u32;
}

pub fn eink_enable() {
    unsafe { eink_enable_unsafe() }
}

unsafe fn eink_enable_unsafe() {
    let mut drive_no: u8 = 0;
    let ret = ITEGetDriveNo(&mut drive_no);
    info!("ITEGetDriveNo: ret: {}, drive_no: {}", ret, drive_no);

    let dev_path = format!("\\\\.\\{}:", (0x41 + drive_no) as char);
    info!("Dev Path: {}", dev_path);

    let cstr = std::ffi::CString::new(dev_path).unwrap();
    info!("Dev Path C: {:?}", &cstr);

    if ITEOpenDeviceAPI(&cstr) == INVALID_HANDLE_VALUE {
        debug!("Open eink device fail, in thread");
        return;
    }

    let ret = ITESet8951KeepAlive(1);
    info!("ITESet8951KeepAlive(1): {}", ret);

    let mut mode: u32 = 1;
    let ret = ITESetMIPIModeAPI(&mut mode);
    info!("ITESetMIPIModeAPI({}): {}", mode, ret);

    mode = 2;
    let ret = ITESetMIPIModeAPI(&mut mode);
    info!("ITESetMIPIModeAPI({}): {}", mode, ret);
}

#[test]
fn test_eink() {
    eink_enable();
}
