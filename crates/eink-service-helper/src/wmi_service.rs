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

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::bail;
use eink_pipe_io::server::Socket;
use jsonrpc_lite::{Id, JsonRpc, Params};
use log::{debug, info};
use parking_lot::Mutex;
use signals2::{Connect1, Connection, Emit1, Signal};
use tokio::runtime::Runtime;
use tokio::select;
use tokio_util::sync::CancellationToken;
use windows::core::HRESULT;
use windows::Win32::Foundation::RPC_E_TOO_LATE;
use wmi::{COMLibrary, Variant, WMIConnection, WMIError};

use crate::utils::{
    jsonrpc_error_internal_error, jsonrpc_error_method_not_found, jsonrpc_success_u32,
};

#[derive(Clone, Debug)]
pub enum LidEvent {
    Open,
    Close,
}

pub struct WmiService {
    /// IPC 接口使用 tokio 异步运行时
    rt: Runtime,

    token: Option<CancellationToken>,

    /// 盒盖翻盖事件
    on_lid_event: Signal<(LidEvent,)>,

    /// 模式切换事件
    on_move_switch_event: Signal<(u32,)>,
}

const PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\wmi";

impl WmiService {
    pub fn new() -> anyhow::Result<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Cannot create tokio runtime for TconService");

        let on_lid_event = Signal::default();
        let on_move_switch_event = Signal::default();

        Ok(Self {
            rt,
            token: None,
            on_lid_event,
            on_move_switch_event,
        })
    }

    pub fn on_lid_event<F>(&mut self, f: F) -> Connection
    where
        F: Fn(LidEvent) -> () + Send + Sync + 'static,
    {
        self.on_lid_event.connect(f)
    }

    pub fn on_mode_switch_event<F>(&mut self, f: F) -> Connection
    where
        F: Fn(u32) -> () + Send + Sync + 'static,
    {
        self.on_move_switch_event.connect(f)
    }

    /// WMI interface for set ALS function for light function.
    /// Below are the interface details:
    /// Name space L"root\\wmi"
    /// Class Name: L"LENOVO_TB_G4_CTRL"
    /// Method name: L"SetAlsForEinkLight"
    /// Input parameter:
    ///     uint32 Data: The control data for function
    ///         0x55 : Function disable
    ///         0xAA : Function enable
    /// Output parameter: uint32 ret:
    ///     return the value of execution status
    pub fn set_als_for_eink_light(&self) {}

    pub fn set_reading_light_status(&mut self, level: u32) -> u32 {
        let ret = cmd_lib_cf::run_cmd! {
            PowerShell.exe -Command "& {(Get-WmiObject -Class LENOVO_TB_G4_CTRL -Namespace ROOT/WMI).SetEinkLightLevel(${level})['ret'] }"
        };
        info!("set_reading_light_status: {ret:?}");
        0
    }

    pub fn get_reading_light_status(&self) -> u32 {
        let ret = cmd_lib_cf::run_fun! {
            PowerShell.exe -Command "& {(Get-WmiObject -Class LENOVO_TB_G4_CTRL -Namespace ROOT/WMI).GetEinkLightLevel()['ret']}"
        };
        info!("get_reading_light_status: {ret:?}");

        if_chain::if_chain! {
            if let Ok(ret) = ret;
            if let Ok(ret) = ret.parse::<u32>();
            then {
                ret
            } else {
                u32::max_value()
            }
        }
    }

    /// WMI interface for set display working status.
    /// In software Display Switch solution, application using this interface to notify the display
    /// working status to EC. That means, every time after application successfully switch the display,
    /// application must use this API to send the current display status to EC.
    /// Also, when application is cracked or re-install, application also must use this API to
    /// re-sync the current display status to EC.
    /// (Reserved for HW switch solution: If in HW solution, application use this API to trigger EC
    /// to switch display. And in HW solution, application uses another get API to get the current
    /// Display Working status)
    /// uint32 Data: The control data for function
    /// 0:  Initial status, Both OLED and E-ink are working
    /// 1 : OLED is Working
    /// 2 : E-ink is Working
    pub fn set_display_working_status(&mut self, status: u32) -> u32 {
        let ret = cmd_lib_cf::run_cmd! {
            PowerShell.exe -Command "& {(Get-WmiObject -Class LENOVO_TB_G4_CTRL -Namespace ROOT/WMI).SetDisplayWorkingStatus(${status})['ret'] }"
        };
        info!("set_display_working_status: {ret:?}");
        0
    }

    pub fn get_display_working_status(&self) -> u32 {
        let ret = cmd_lib_cf::run_fun! {
            PowerShell.exe -Command "& {(Get-WmiObject -Class LENOVO_TB_G4_CTRL -Namespace ROOT/WMI).GetDisplayWorkingStatus()['ret']}"
        };
        info!("get_display_working_status: {ret:?}");

        let ret = if_chain::if_chain! {
            if let Ok(ret) = ret;
            if let Ok(ret) = ret.trim().parse::<u32>();
            then {
                ret
            } else {
                u32::max_value()
            }
        };
        info!("get_display_working_status: {ret}");
        ret
    }

    pub fn send_mode_switch_event(&mut self, mode: u32) {
        // 取消上一次未触发的事件
        if let Some(token) = self.token.take() {
            token.cancel();
        }

        let token = CancellationToken::new();
        let cloned_token = token.clone();

        let on_move_switch_event2 = self.on_move_switch_event.clone();

        self.rt.spawn(async move {
            select! {
                _ = cloned_token.cancelled() => {
                    info!("Got mode switch event too fast, Ignore mode '{mode}'")
                }
                // 200ms 保护间隔
                _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                    // Tokio Runtime -> Normal Thread
                    std::thread::spawn(move || {
                        on_move_switch_event2.emit(mode);
                    });
                }
            }
        });

        // 保存 token
        self.token.replace(token);
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static WMI_SERVICE: Arc<Mutex<WmiService>> = {
    info!("Create WmiService");
    Arc::new(Mutex::new(
        WmiService::new().expect("Cannot instantiate WmiService"),
    ))
};

/// 初始化 COM 基础设施，如果 COM 已经被初始化，将会返回 RPC_E_TOO_LATE 错误，忽略即可
pub fn wmi_create_com_library() -> anyhow::Result<COMLibrary> {
    match COMLibrary::new() {
        Ok(com) => Ok(com),
        Err(WMIError::HResultError { hres }) => {
            if HRESULT(hres) == RPC_E_TOO_LATE {
                match COMLibrary::without_security() {
                    Ok(com) => Ok(com),
                    Err(_) => bail!("Cannot create COMLibrary without security"),
                }
            } else {
                bail!("Cannot create COMLibrary, Call failed with: {:#X}", hres)
            }
        }
        Err(err) => {
            bail!("Cannot create COMLibrary, err: {}", &err.to_string())
        }
    }
}

/// 启动服务
pub fn start_service(this: &Arc<Mutex<WmiService>>) -> anyhow::Result<()> {
    info!("WmiService: start_service");

    // 服务内部初始化
    let this_cloned = this.clone();

    // 接受 Lenovo_LidEvent 事件 不需要处理该事件，全部由mode switch响应 Niuxj 2022-11-16
    // std::thread::spawn(move || {
    //     //
    //     // 初始化 COM 基础设施，如果 COM 已经被初始化，将会返回 RPC_E_TOO_LATE 错误，忽略即可
    //     let com = match wmi_create_com_library() {
    //         Ok(com) => com,
    //         Err(err) => {
    //             log::error!("{:?}", err);
    //             return;
    //         }
    //     };

    //     // 创建 WMI 链接
    //     let wmi_con = match WMIConnection::with_namespace_path("root/wmi", com.into()) {
    //         Ok(conn) => conn,
    //         Err(err) => {
    //             log::error!("Cannot create WMIConnection, err: {}", &err.to_string());
    //             return;
    //         }
    //     };

    //     let iterator =
    //         wmi_con.raw_notification::<HashMap<String, Variant>>("SELECT * FROM LENOVO_LID_CHANGE_EVENT");

    //     let iterator = match iterator {
    //         Ok(v) => v,
    //         Err(err) => {
    //             debug!("Cannot listen LENOVO_LID_CHANGE_EVENT: {err:?}");
    //             return;
    //         }
    //     };

    //     // WBEM_E_UNPARSABLE_QUERY 0x80041058
    //     for result in iterator {
    //         let result = result.unwrap();
    //         let status = result.get("ULong").unwrap();
    //         if let Variant::UI4(status) = status {
    //             info!("Lenovo_LidEvent: status: {:?}", status);

    //             let on_lid_event_cloned = this_cloned.lock().on_lid_event.clone();
    //             if status == &0 {
    //                 on_lid_event_cloned.emit(LidEvent::Open);
    //             } else if status == &1 {
    //                 on_lid_event_cloned.emit(LidEvent::Close);
    //             }
    //         }
    //     }
    // });

    // 接受 LENOVO_BASE_MODE_SWITCH_EVENT 事件
    // uint32 ret: return current base mode
    //  1 : Mode 1: NoteBook – OLED (0~70°)
    //  2 : Mode 2: NoteBook – OLED (70~110°)
    //  3 : Mode 3: NoteBook – OLED (110~180°)
    //  4 : Mode 4: NoteBook - E-ink(110~180°)
    //  5 : Mode 5: NoteBook - E-ink(70~110°)
    //  6 : Mode 6: NoteBook - E-ink(0~70°)
    //  7 : Mode 7: Tablet - OLED
    //  8 : Mode 8: Tablet - E-ink
    //  9 : Mode 9: Twisting – OLED (180°  +/- 10°)
    //  10 : Mode 10: Twisting – E-ink (0° +/- 10°)
    //  11 : Mode 11: Twisting – Midway(10~170°)
    // Other：reserve
    let this_cloned = this.clone();
    std::thread::spawn(move || {
        //
        // 初始化 COM 基础设施，如果 COM 已经被初始化，将会返回 RPC_E_TOO_LATE 错误，忽略即可
        let com = match wmi_create_com_library() {
            Ok(com) => com,
            Err(err) => {
                log::error!("{:?}", err);
                return;
            }
        };

        // 创建 WMI 链接
        let wmi_con = match WMIConnection::with_namespace_path("root/wmi", com.into()) {
            Ok(conn) => conn,
            Err(err) => {
                log::error!("Cannot create WMIConnection, err: {}", &err.to_string());
                return;
            }
        };

        let iterator = wmi_con.raw_notification::<HashMap<String, Variant>>(
            "SELECT * FROM LENOVO_BASE_MODE_SWITCH_EVENT",
        );

        let iterator = match iterator {
            Ok(v) => v,
            Err(err) => {
                debug!("Cannot listen LENOVO_BASE_MODE_SWITCH_EVENT: {err:?}");
                return;
            }
        };

        for result in iterator {
            let result = result.unwrap();
            info!("LENOVO_BASE_MODE_SWITCH_EVENT: result: {:?}", result);
            let mode = result.get("ret").unwrap();
            if let Variant::UI4(mode) = mode {
                info!("LENOVO_BASE_MODE_SWITCH_EVENT: mode: {:?}", *mode);
                this_cloned.lock().send_mode_switch_event(*mode);
            }
        }
    });

    // 启动 IPC 线程
    let mut server = eink_pipe_io::server::Server::new(PIPE_NAME);

    let this_cloned = this.clone();
    let _ = server.on_connection(move |socket, _req| {
        info!("WmiService: On connection");
        let this_cloned = this_cloned.clone();
        socket
            .lock()
            .on_request(move |_socket: Arc<Mutex<Socket>>, id: Id, req: JsonRpc| {
                info!("TconService: On request: {req:?}");
                match req.get_method() {
                    Some("set_reading_light_status") => {
                        if_chain::if_chain! {
                            if let Some(Params::Map(map)) = req.get_params();
                            if let Some(level) = map.get("level");
                            if let Some(level) = level.as_u64();
                            then {
                                this_cloned.lock().set_reading_light_status(level as u32);
                                jsonrpc_success_u32(id, 0)
                            } else {
                                jsonrpc_error_internal_error(id)
                            }
                        }
                    }
                    Some("get_reading_light_status") => {
                        let level = this_cloned.lock().get_reading_light_status();
                        jsonrpc_success_u32(id, level)
                    }
                    Some(&_) => jsonrpc_error_method_not_found(id),
                    None => jsonrpc_error_method_not_found(id),
                }
            });
        0
    });

    // 在异步运行时启动
    this.lock().rt.spawn(async move {
        info!("TconService: start server listen");
        server.listen().await;
        info!("TconService: stop server listen");
    });
    Ok(())
}
