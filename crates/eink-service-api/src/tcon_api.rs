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

const TCON_PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\tcon";

//
// 将 Native 库设置为 Lazy 全局变量
//
pub static TCON_CLIENT: Mutex<Option<BlockingClient>> = Mutex::new(None);

/// 检查链接状态
fn ensure_tcon_client() {
    let mut guard = TCON_CLIENT.lock();

    if guard.is_none() {
        let client = eink_pipe_io::blocking::connect(TCON_PIPE_NAME);

        if let Ok(client) = client {
            guard.replace(client);
        } else {
            error!("Cannot connect to tcon service: last error: {:?}", unsafe {
                GetLastError()
            });
        }
    }
}

/// 设置 Eink 刷新
#[no_mangle]
pub extern "C" fn eink_refresh() -> u32 {
    ensure_tcon_client();
    let mut guard = TCON_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("refresh", json!({}))
            .expect("Cannot invoke remote method to tcon service");
        info!("eink_refresh: result: {:?}", reply.get_result());
    }
    0
}

/// 设置 Eink MIPI Mode
#[no_mangle]
pub extern "C" fn eink_set_mipi_mode(mode: u32) -> u32 {
    ensure_tcon_client();
    let mut guard = TCON_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("set_mipi_mode", json!({ "mode": mode }))
            .expect("Cannot invoke remote method to tcon service");
        info!("eink_set_mipi_mode: result: {:?}", reply.get_result());
    }
    0
}

/// 获得当前 Eink MIPI Mode
/// 返回值为 i32 带符号类型，<1 表示错误，>=0 表示正确
#[no_mangle]
pub extern "C" fn eink_get_mipi_mode() -> i32 {
    ensure_tcon_client();
    let mut guard = TCON_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("get_mipi_mode", json!({}))
            .expect("Cannot invoke remote method to tcon service");
        let result = reply.get_result();
        info!("get_mipi_mode: result: {:?}", result);
        if let Some(result) = result {
            if result.is_i64() {
                return result.as_i64().unwrap_or(-1) as i32;
            }
        }
    }
    -1
}

/// 设置 Eink 显示关机壁纸
#[no_mangle]
pub extern "C" fn eink_show_shutdown_cover() -> u32 {
    ensure_tcon_client();
    let mut guard = TCON_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params("show_shutdown_cover", json!({}))
            .expect("Cannot invoke remote method to tcon service");
        info!("eink_show_shutdown_cover: result: {:?}", reply.get_result());
    }
    0
}

/// 设置 Eink 关机壁纸
#[no_mangle]
pub extern "C" fn eink_set_shutdown_cover(path: *const u16, disp_type: u32) -> u32 {
    let path = unsafe { widestring::U16CString::from_ptr_str(path) };
    let path = path.to_string_lossy();

    info!(
        "eink_set_shutdown_cover: path: {}, type: {}",
        &path, disp_type
    );

    ensure_tcon_client();
    let mut guard = TCON_CLIENT.lock();
    if let Some(client) = guard.as_mut() {
        let reply = client
            .call_with_params(
                "set_shutdown_cover",
                json!({
                    "path": path,
                    "type": disp_type
                }),
            )
            .expect("Cannot invoke remote method to tcon service");

        info!("eink_set_shutdown_cover: result: {:?}", reply.get_result());
    }

    0
}
