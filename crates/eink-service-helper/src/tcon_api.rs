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
use windows::Win32::Foundation::{GetLastError, SetLastError, HINSTANCE, NO_ERROR};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

const TCON_PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\tcon";

//
// 将 Native 库设置为 Lazy 全局变量
//
pub static TCON_CLIENT: Mutex<Option<BlockingClient>> = Mutex::new(None);

/// 检查链接状态
fn connect_tcon_client() {
    let mut guard = TCON_CLIENT.lock();

    unsafe { SetLastError(NO_ERROR) };

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

pub fn disconnect_tcon_client() {
    let mut guard = TCON_CLIENT.lock();
    let _client = guard.take();
}

/// 设置 Eink 刷新
pub fn eink_refresh() -> u32 {
    for _ in 0..2 {
        connect_tcon_client();
        let mut guard = TCON_CLIENT.lock();
        if let Some(client) = guard.as_mut() {
            match client.call_with_params("refresh", json!({})) {
                Ok(reply) => {
                    log::info!("eink_refresh: result: {:?}", reply.get_result());
                    break;
                }
                Err(err) => {
                    log::error!("Cannot invoke remote method to tcon service: err: {err:?}");

                    // 发生错误，断开链接, 再次尝试
                    disconnect_tcon_client();
                    continue;
                }
            }
        }
    }
    0
}

/// 设置 Eink MIPI Mode
pub fn eink_set_mipi_mode(mode: u32) -> u32 {
    for _ in 0..2 {
        connect_tcon_client();
        let mut guard = TCON_CLIENT.lock();
        if let Some(client) = guard.as_mut() {
            match client.call_with_params("set_mipi_mode", json!({ "mode": mode })) {
                Ok(reply) => {
                    log::info!("eink_set_mipi_mode: result: {:?}", reply.get_result());
                    break;
                }
                Err(err) => {
                    log::error!("Cannot invoke remote method to tcon service: err: {err:?}");

                    // 发生错误，断开链接, 再次尝试
                    disconnect_tcon_client();
                    continue;
                }
            }
        }
    }
    0
}

/// 软件启动 TCON
/// 重试 2 次
pub fn eink_software_reset_tcon() -> u32 {
    for _ in 0..2 {
        connect_tcon_client();
        let mut guard = TCON_CLIENT.lock();
        if let Some(client) = guard.as_mut() {
            match client.call_with_params("software_reset_api", json!({})) {
                Ok(reply) => {
                    log::info!("eink_software_reset_tcon: result: {:?}", reply.get_result());
                    break;
                }
                Err(err) => {
                    log::error!("Cannot invoke remote method to tcon service: err: {err:?}");

                    // 发生错误，断开链接, 再次尝试
                    disconnect_tcon_client();
                    continue;
                }
            }
        }
    }
    0
}

/// 设置 Eink 显示关机壁纸
pub fn eink_show_shutdown_cover() -> u32 {
    for _ in 0..2 {
        connect_tcon_client();
        let mut guard = TCON_CLIENT.lock();
        if let Some(client) = guard.as_mut() {
            match client.call_with_params("show_shutdown_cover", json!({})) {
                Ok(reply) => {
                    log::info!("eink_show_shutdown_cover: result: {:?}", reply.get_result());
                    break;
                }
                Err(err) => {
                    log::error!("Cannot invoke remote method to tcon service: err: {err:?}");

                    // 发生错误，断开链接, 再次尝试
                    disconnect_tcon_client();
                    continue;
                }
            }
        }
    }
    0
}

/// 设置 Eink 关机壁纸
pub fn eink_set_shutdown_cover(path: *const u16, disp_type: u32) -> u32 {
    let path = unsafe { widestring::U16CString::from_ptr_str(path) };
    let path = path.to_string_lossy();

    info!(
        "eink_set_shutdown_cover: path: {}, type: {}",
        &path, disp_type
    );

    for _ in 0..2 {
        connect_tcon_client();
        let mut guard = TCON_CLIENT.lock();
        if let Some(client) = guard.as_mut() {
            match client.call_with_params(
                "set_shutdown_cover",
                json!({
                    "path": path,
                    "type": disp_type
                }),
            ) {
                Ok(reply) => {
                    log::info!("eink_set_shutdown_cover: result: {:?}", reply.get_result());
                    break;
                }
                Err(err) => {
                    log::error!("Cannot invoke remote method to tcon service: err: {err:?}");

                    // 发生错误，断开链接, 再次尝试
                    disconnect_tcon_client();
                    continue;
                }
            }
        }
    }

    0
}

/// 设置 Eink 显示关机壁纸
pub fn eink_start_lockscreen_note() -> u32 {
    for _ in 0..2 {
        connect_tcon_client();
        let mut guard = TCON_CLIENT.lock();
        if let Some(client) = guard.as_mut() {
            match client.call_with_params("start_lockscreen_note", json!({})) {
                Ok(reply) => {
                    log::info!("start_lockscreen_note: result: {:?}", reply.get_result());
                    break;
                }
                Err(err) => {
                    log::error!("Cannot invoke remote method to tcon service: err: {err:?}");

                    // 发生错误，断开链接, 再次尝试
                    disconnect_tcon_client();
                    continue;
                }
            }
        }
    }
    0
}

/// TODO: 临时借用宝地
pub fn eink_start_launcher() -> u32 {
    for _ in 0..2 {
        connect_tcon_client();
        let mut guard = TCON_CLIENT.lock();
        if let Some(client) = guard.as_mut() {
            match client.call_with_params("start_launcher", json!({})) {
                Ok(reply) => {
                    log::info!("start_launcher: result: {:?}", reply.get_result());
                    break;
                }
                Err(err) => {
                    log::error!("Cannot invoke remote method to tcon service: err: {err:?}");

                    // 发生错误，断开链接, 再次尝试
                    disconnect_tcon_client();
                    continue;
                }
            }
        }
    }
    0
}

pub const TOUCH_EVENT_NO_REPORT: u32 = 0x70;
pub const TOUCH_EVENT_PEN_ONLY: u32 = 0x60;
pub const TOUCH_EVENT_TOUCH_ONLY: u32 = 0x50;
pub const TOUCH_EVENT_BOTH: u32 = 0x40;

/// 设置 Eink 触摸区域
pub fn eink_set_tp_mask_area(
    pen_style: u32,
    area_id: u32,
    x1: u32,
    x2: u32,
    y1: u32,
    y2: u32,
) -> u32 {
    for _ in 0..2 {
        connect_tcon_client();
        let mut guard = TCON_CLIENT.lock();
        if let Some(client) = guard.as_mut() {
            match client.call_with_params(
                "set_tp_mask_area",
                json!({
                    "pen_style": pen_style,
                    "area_id": area_id,
                    "x1": x1,
                    "x2": x2,
                    "y1": y1,
                    "y2": y2,
                }),
            ) {
                Ok(reply) => {
                    log::info!("eink_set_tp_mask_area: result: {:?}", reply.get_result());
                    break;
                }
                Err(err) => {
                    log::error!("Cannot invoke remote method to tcon service: err: {err:?}");

                    // 发生错误，断开链接, 再次尝试
                    disconnect_tcon_client();
                    continue;
                }
            }
        }
    }
    0
}
