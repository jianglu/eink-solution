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

const KEYBOARD_PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\keyboard";

//
// 将 Native 库设置为 Lazy 全局变量
//
pub static KEYBOARD_CLIENT: Mutex<Option<BlockingClient>> = Mutex::new(None);

/// 检查链接状态
fn ensure_keyboard_client() {
    let mut guard = KEYBOARD_CLIENT.lock();

    if guard.is_none() {
        let client = eink_pipe_io::blocking::connect(KEYBOARD_PIPE_NAME);

        if let Ok(client) = client {
            guard.replace(client);
        } else {
            error!(
                "Cannot connect to keyboard service: last error: {:?}",
                unsafe { GetLastError() }
            );
        }
    }
}

/// 设置窗口为置顶
#[no_mangle]
pub extern "C" fn disable_win_key() -> u32 {
    ensure_keyboard_client();
    let mut guard = KEYBOARD_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("disable_win_key", json!({}))
            .expect("Cannot invoke remote method to keyboard service");
        info!("disable_win_key: result: {:?}", reply.get_result());
    }
    0
}

/// 清除所有置顶窗口
#[no_mangle]
pub extern "C" fn enable_win_key() -> u32 {
    ensure_keyboard_client();
    let mut guard = KEYBOARD_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("enable_win_key", json!({}))
            .expect("Cannot invoke remote method to keyboard service");
        info!("enable_win_key: result: {:?}", reply.get_result());
    }
    0
}
