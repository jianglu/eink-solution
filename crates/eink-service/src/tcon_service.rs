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
    mem::zeroed,
    ops::DerefMut,
    sync::{Arc, Weak},
};

use anyhow::{bail, Result};
use eink_itetcon::{
    ITECleanUpEInkAPI, ITEDisplayAreaAPI, ITEGetBufferAddrInfoAPI, ITEGetDriveNo, ITEOpenDeviceAPI,
    ITESet8951KeepAlive, ITESetFA2, ITESetMIPIModeAPI, IteTconDevice, GI_MIPI_FAST_READER,
    GI_MIPI_READER,
};
use eink_pipe_io::server::Socket;
use jsonrpc_lite::{Id, JsonRpc, Params};
use log::{error, info};
use parking_lot::{Mutex, RwLock};
use serde_json::json;
use signals2::{connect::ConnectionImpl, Connect2, Emit2, Signal};
use tokio::runtime::Runtime;
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;

use crate::utils::{
    jsonrpc_error_internal_error, jsonrpc_error_invalid_params, jsonrpc_error_method_not_found,
    jsonrpc_success_string,
};

const PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\tcon";

pub struct TconService {
    /// IPC 接口使用 tokio 异步运行时
    rt: Runtime,

    tcon_device: Arc<RwLock<IteTconDevice>>,

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
            tcon_device: Arc::new(RwLock::new(IteTconDevice::new()?)),
            on_request: Default::default(),
        })
    }

    /// 启动服务
    pub fn start(&mut self) -> Result<()> {
        info!("TconService: start");

        let tcon_avail = match self.tcon_device.write().open() {
            Ok(_) => true,
            Err(_) => {
                error!("TconService: failed open tcon device");
                false
            }
        };

        // 每隔 30 秒进行 EINK 保活
        if tcon_avail {
            std::thread::spawn(|| loop {
                info!("Start Eink Live Keeper");
                tcon_keep_alive();
                std::thread::sleep(std::time::Duration::from_secs(30));
            });
        }

        let tcon_device = self.tcon_device.clone();

        self.on_request.write().connect(move |id, req| {
            info!("TconService: On request");
            match req.get_method() {
                Some("refresh") => {
                    if !tcon_avail {
                        return jsonrpc_error_internal_error(id);
                    }
                    tcon_refresh();
                    jsonrpc_success_string(id, "true")
                }
                Some("set_mipi_mode") => {
                    if !tcon_avail {
                        return jsonrpc_error_internal_error(id);
                    }
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
                Some("show_shutdown_cover") => {
                    tcon_device.write().show_cover_image();
                    jsonrpc_success_string(id, "true")
                }
                Some("set_shutdown_cover") => {
                    let path = {
                        if let Some(Params::Map(map)) = req.get_params() {
                            if let Some(path) = map.get("path") {
                                if let Some(path) = path.as_str() {
                                    path.to_owned()
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
                    tcon_device.write().set_cover_image(&path);
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

    // /// 打开 Tcon 设备
    // fn open_tcon_device(&self) -> Result<()> {
    //     info!("TconService: open_tcon_device");

    //     // 获得设备驱动号
    //     let mut drive_no: u8 = 0;
    //     let ret = unsafe { ITEGetDriveNo(&mut drive_no) };
    //     info!("ITEGetDriveNo: ret: {}, drive_no: {}", ret, drive_no);

    //     // 打开设备
    //     let dev_path = format!("\\\\.\\{}:", (0x41 + drive_no) as char);
    //     info!("Dev Path: {}", dev_path);

    //     let cstr = std::ffi::CString::new(dev_path).unwrap();
    //     info!("Dev Path C: {:?}", &cstr);

    //     if unsafe { ITEOpenDeviceAPI(&cstr) } == INVALID_HANDLE_VALUE {
    //         bail!("Open eink device fail, in thread");
    //     }

    //     // 设置 Tcon 为 KeepAlive 模式
    //     let ret = unsafe { ITESet8951KeepAlive(1) };
    //     info!("ITESet8951KeepAlive(1): {}", ret);

    //     // 设置 MIPI 模式
    //     let mut mode: u32 = 1;
    //     let ret = unsafe { ITESetMIPIModeAPI(&mut mode) };
    //     info!("ITESetMIPIModeAPI({}): {}", mode, ret);

    //     mode = 2;
    //     let ret = unsafe { ITESetMIPIModeAPI(&mut mode) };
    //     info!("ITESetMIPIModeAPI({}): {}", mode, ret);

    //     // 获得图片地址（支持 3 张图片），支持 3 张图片轮询
    //     // let mut addrs: [u32; 3] = unsafe { zeroed() };
    //     unsafe { ITEGetBufferAddrInfoAPI(&mut self.image_addrs) };
    //     println!(
    //         "EinkTcon ITEGetBufferAddrInfoAPI: addrs: {:?}",
    //         self.image_addrs
    //     );

    //     Ok(())
    // }
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
    unsafe { ITECleanUpEInkAPI() };
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

/// TCON 保活
pub fn tcon_keep_alive() {
    let ret = unsafe { ITESet8951KeepAlive(1) };
    info!("ITESet8951KeepAlive(1): {}", ret);

    // let mut mode: u32 = 1;
    // let ret = unsafe { ITESetMIPIModeAPI(&mut mode) };
    // info!("ITESetMIPIModeAPI({}): {}", mode, ret);

    // mode = 2;
    // let ret = unsafe { ITESetMIPIModeAPI(&mut mode) };
    // info!("ITESetMIPIModeAPI({}): {}", mode, ret);
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
