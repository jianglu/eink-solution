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

use windows::Win32::Foundation::HANDLE;

#[allow(non_snake_case)]

/// 和 EInkTcon.dll 的最底层对接

#[windows_dll::dll(EInkTcon)]
extern "system" {
    pub fn ITEGetDriveNo(drive_no: &mut u8) -> u32;

    pub fn ITEOpenDeviceAPI(dev_path: &CStr) -> HANDLE;

    pub fn ITESet8951KeepAlive(enable: u32) -> u32;

    pub fn ITESetMIPIModeAPI(mode: &mut u32) -> u32;

    pub fn ITEGetBufferAddrInfoAPI(addrs: *mut u32) -> u32;
}

#[test]
fn test_get_buffer_addr_info() {
    let mut addrs: u32 = 0;
    unsafe { ITEGetBufferAddrInfoAPI(&mut addrs) };
    println!("ITEGetBufferAddrInfoAPI: addrs: {addrs}");
}
