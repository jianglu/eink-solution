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

const TOPMOST_PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\topmost";

//
// 将 Native 库设置为 Lazy 全局变量
//
pub static TOPMOST_CLIENT: Mutex<Option<BlockingClient>> = Mutex::new(None);

/// 检查链接状态
fn ensure_topmost_client() {
    let mut guard = TOPMOST_CLIENT.lock();

    if guard.is_none() {
        let client = eink_pipe_io::blocking::connect(TOPMOST_PIPE_NAME);

        if let Ok(client) = client {
            guard.replace(client);
        } else {
            error!(
                "Cannot connect to topmost service: last error: {:?}",
                unsafe { GetLastError() }
            );
        }
    }
}

/// 设置窗口为置顶
#[no_mangle]
pub extern "C" fn set_window_topmost(hwnd: u32) -> u32 {
    ensure_topmost_client();
    let mut guard = TOPMOST_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("set_window_topmost", json!({ "hwnd": hwnd }))
            .expect("Cannot invoke remote method to topmost service");
        info!("set_window_topmost: result: {:?}", reply.get_result());
    }
    0
}

/// 设置窗口为置顶
#[no_mangle]
pub extern "C" fn unset_window_topmost(hwnd: u32) -> u32 {
    ensure_topmost_client();
    let mut guard = TOPMOST_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("unset_window_topmost", json!({ "hwnd": hwnd }))
            .expect("Cannot invoke remote method to topmost service");
        info!("unset_window_topmost: result: {:?}", reply.get_result());
    }
    0
}

/// 清除所有置顶窗口
#[no_mangle]
pub extern "C" fn clear_all_windows_topmost() -> u32 {
    ensure_topmost_client();
    let mut guard = TOPMOST_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("clear_all_windows_topmost", json!({}))
            .expect("Cannot invoke remote method to topmost service");
        info!(
            "clear_all_windows_topmost: result: {:?}",
            reply.get_result()
        );
    }
    0
}

/// 设置窗口为置顶
#[no_mangle]
pub extern "C" fn adjust_topmost_on_app_launched(pid: isize) -> u32 {
    ensure_topmost_client();
    let mut guard = TOPMOST_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("adjust_topmost_on_app_launched", json!({ "pid": pid }))
            .expect("Cannot invoke remote method to topmost service");
        info!(
            "adjust_topmost_on_app_launched: result: {:?}",
            reply.get_result()
        );
    }
    0
}
