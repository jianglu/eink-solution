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

const WMI_PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\wmi";

//
// 将 Native 库设置为 Lazy 全局变量
//
pub static WMI_CLIENT: Mutex<Option<BlockingClient>> = Mutex::new(None);

/// 检查链接状态
fn ensure_wmi_client() {
    let mut guard = WMI_CLIENT.lock();

    if guard.is_none() {
        let client = eink_pipe_io::blocking::connect(WMI_PIPE_NAME);

        if let Ok(client) = client {
            guard.replace(client);
        } else {
            error!("Cannot connect to tcon service: last error: {:?}", unsafe {
                GetLastError()
            });
        }
    }
}

/// 设置 EINK 阅读灯
#[no_mangle]
pub extern "C" fn eink_set_reading_light_status(level: u32) -> u32 {
    ensure_wmi_client();
    let mut guard = WMI_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("set_reading_light_status", json!({ "level": level }))
            .expect("Cannot invoke remote method to tcon service");
        info!("set_reading_light_status: result: {:?}", reply.get_result());
    }
    0
}

/// 设置 EINK 阅读灯
#[no_mangle]
pub extern "C" fn eink_get_reading_light_status() -> u32 {
    ensure_wmi_client();
    let mut guard = WMI_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("get_reading_light_status", json!({}))
            .expect("Cannot invoke remote method to tcon service");
        info!("set_reading_light_status: result: {:?}", reply.get_result());
        if let Some(level) = reply.get_result() {
            if let Some(level) = level.as_u64() {
                return level as u32;
            }
        }
    }
    u32::max_value()
}
