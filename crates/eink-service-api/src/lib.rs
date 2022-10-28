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

use std::ffi::c_void;

use eink_pipe_io::blocking::BlockingClient;
use log::{error, info};
use parking_lot::Mutex;
use serde_json::json;
use windows::Win32::{
    Foundation::{GetLastError, HINSTANCE},
    System::SystemServices::DLL_PROCESS_ATTACH,
};

mod keyboard_api;
mod tcon_api;
mod topmost_api;

#[no_mangle]
extern "stdcall" fn DllMain(_hInstDll: HINSTANCE, fdwReason: u32, _lpvReserved: *mut c_void) {
    if fdwReason == DLL_PROCESS_ATTACH {
        // 设置当前的活动日志系统为 OutputDebugString 输出
        eink_logger::init_with_level(log::Level::Trace).unwrap();
        info!("Eink Service API Init Logging");
    }
}
