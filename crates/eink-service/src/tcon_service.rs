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

use std::{
    ops::DerefMut,
    sync::{Arc, Weak},
};

use anyhow::{bail, Result};
use eink_itetcon::{
    ITEGetDriveNo, ITEOpenDeviceAPI, ITESet8951KeepAlive, ITESetFA2, ITESetMIPIModeAPI,
};
use eink_pipe_io::server::Socket;
use jsonrpc_lite::{Id, JsonRpc, Params};
use log::{error, info};
use parking_lot::{Mutex, RwLock};
use serde_json::json;
use signals2::{connect::ConnectionImpl, Connect2, Emit2, Signal};
use tokio::runtime::Runtime;
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;

pub struct SelfKeeper<T> {
    them: Mutex<Weak<T>>,
}

impl<T> SelfKeeper<T> {
    pub fn new() -> Self {
        Self {
            them: Mutex::new(Weak::new()),
        }
    }

    pub fn save(&self, arc: &Arc<T>) {
        *self.them.lock().deref_mut() = Arc::downgrade(arc);
    }

    pub fn get(&self) -> Arc<T> {
        match self.them.lock().upgrade() {
            Some(arc) => return arc,
            None => unreachable!(),
        }
    }
}

const PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\tcon";

pub struct TconService {
    /// IPC 接口使用 tokio 异步运行时
    rt: Runtime,

    /// IPC 请求响应函数
    on_request: Arc<RwLock<Signal<(Id, JsonRpc), JsonRpc>>>,
}

impl TconService {
    pub fn new() -> Result<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Cannot create tokio runtime for TconService");

        Ok(Self {
            rt,
            on_request: Default::default(),
        })
    }

    /// 启动服务
    pub fn start(&mut self) -> Result<()> {
        info!("TconService: start");

        match self.open_tcon_device() {
            Ok(_) => (),
            Err(_) => error!("TconService: failed open tcon device"),
        }

        self.on_request.write().connect(|id, req| {
            info!("TconService: On request");
            match req.get_method() {
                Some("refresh") => {
                    tcon_refresh();
                    jsonrpc_success_string(id, "true")
                }
                Some("set_mipi_mode") => {
                    let mode = {
                        if let Some(Params::Map(map)) = req.get_params() {
                            if let Some(mode) = map.get("mode") {
                                if let Some(mode) = mode.as_u64() {
                                    mode
                                } else {
                                    return jsonrpc_error_invalid_params(id);
                                }
                            } else {
                                return jsonrpc_error_invalid_params(id);
                            }
                        } else {
                            return jsonrpc_error_invalid_params(id);
                        }
                    };

                    let mode = match MipiMode::try_from(mode as u32) {
                        Ok(mode) => mode,
                        Err(_) => {
                            return jsonrpc_error_invalid_params(id);
                        }
                    };

                    tcon_set_mipi_mode(mode);
                    jsonrpc_success_string(id, "true")
                }
                Some(&_) => jsonrpc_error_method_not_found(id),
                None => jsonrpc_error_internal_error(id),
            }
        });
        self.start_ipc_server()?;
        Ok(())
    }

    /// 停止服务
    pub fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// 启动 IPC 服务器
    fn start_ipc_server(&mut self) -> Result<()> {
        info!("TconService: start_ipc_server");
        let mut server = eink_pipe_io::server::Server::new(PIPE_NAME);

        let on_request = self.on_request.clone();

        let _ = server.on_connection(move |socket, req| {
            info!("TconService: On connection");
            let on_request2 = on_request.clone();
            socket
                .lock()
                .on_request(move |_socket: Arc<Mutex<Socket>>, id: Id, req: JsonRpc| {
                    info!("TconService: On request: {req:?}");
                    match on_request2.read().emit(id.clone(), req) {
                        Some(reply) => reply,
                        None => JsonRpc::error(id, jsonrpc_lite::Error::internal_error()),
                    }
                });
            0
        });

        // 在异步运行时启动
        self.rt.spawn(async move {
            info!("TconService: start server listen");
            server.listen().await;
            info!("TconService: stop server listen");
        });

        Ok(())
    }

    /// 打开 Tcon 设备
    fn open_tcon_device(&self) -> Result<()> {
        info!("TconService: open_tcon_device");

        // 获得设备驱动号
        let mut drive_no: u8 = 0;
        let ret = unsafe { ITEGetDriveNo(&mut drive_no) };
        info!("ITEGetDriveNo: ret: {}, drive_no: {}", ret, drive_no);

        // 打开设备
        let dev_path = format!("\\\\.\\{}:", (0x41 + drive_no) as char);
        info!("Dev Path: {}", dev_path);

        let cstr = std::ffi::CString::new(dev_path).unwrap();
        info!("Dev Path C: {:?}", &cstr);

        if unsafe { ITEOpenDeviceAPI(&cstr) } == INVALID_HANDLE_VALUE {
            bail!("Open eink device fail, in thread");
        }

        // 设置 Tcon 为 KeepAlive 模式
        let ret = unsafe { ITESet8951KeepAlive(1) };
        info!("ITESet8951KeepAlive(1): {}", ret);

        // 设置 MIPI 模式
        let mut mode: u32 = 1;
        let ret = unsafe { ITESetMIPIModeAPI(&mut mode) };
        info!("ITESetMIPIModeAPI({}): {}", mode, ret);

        mode = 2;
        let ret = unsafe { ITESetMIPIModeAPI(&mut mode) };
        info!("ITESetMIPIModeAPI({}): {}", mode, ret);

        Ok(())
    }
}

#[derive(Default, num_enum::IntoPrimitive, num_enum::FromPrimitive)]
#[repr(u32)]
enum MipiMode {
    #[default]
    Reader = 0x00,
    Mixed = 0x01,
    Browser = 0x02,
    FastReader = 0x03,
    FastUI = 0x04,
    Sleep = 0x0F,
    No = 0x10,
    Refresh = 0x11,
    Standby = 0x12,
    HandWriting = 0x13,
    Hybrid = 0xF0,
}

// fn get_param(params: &Option<Params>, key: &str) -> Result<&Value> {
//     if let Some(Params::Map(map)) = req.get_params() {
//         if let Some(mode) = map.get(key) {
//             mode
//         }
//     }
//     bail!("Cannot find param {key}")
// }

fn tcon_refresh() {
    info!("tcon_refresh");
}

/// 设置 MIPI 模式
fn tcon_set_mipi_mode(mipi_mode: MipiMode) {
    let mut mode: u32 = 1;
    let ret = unsafe {
        ITESetFA2(1);
        ITESetMIPIModeAPI(&mut mode)
    };
    info!("ITESetMIPIModeAPI({}): {}", mode, ret);

    mode = mipi_mode.into();
    let ret = unsafe {
        ITESetFA2(1);
        ITESetMIPIModeAPI(&mut mode)
    };
    info!("ITESetMIPIModeAPI({}): {}", mode, ret);
}

/// 返回成功（字符串值）
fn jsonrpc_success_string(id: Id, result: &str) -> JsonRpc {
    JsonRpc::success(id, &serde_json::Value::String(result.to_owned()))
}

/// 返回错误（无效参数）
fn jsonrpc_error_invalid_params(id: Id) -> JsonRpc {
    JsonRpc::error(id, jsonrpc_lite::Error::invalid_params())
}

/// 返回错误（找不到方法）
fn jsonrpc_error_method_not_found(id: Id) -> JsonRpc {
    JsonRpc::error(id, jsonrpc_lite::Error::method_not_found())
}

/// 返回错误（内部错误）
fn jsonrpc_error_internal_error(id: Id) -> JsonRpc {
    JsonRpc::error(id, jsonrpc_lite::Error::internal_error())
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static TCON_SERVICE: Arc<Mutex<TconService>> = {
    info!("Create TconService");
    Arc::new(Mutex::new(
        TconService::new().expect("Cannot instantiate TconService"),
    ))
};
