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

use std::sync::Arc;

use eink_pipe_io::server::Socket;
use jsonrpc_lite::{Id, JsonRpc, Params};
use log::info;
use parking_lot::{Mutex, RwLock};
use signals2::{Connect2, Emit2, Signal};
use tokio::runtime::Runtime;
use windows::{
    s,
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            GetForegroundWindow, SendMessageA, ShowWindow, SW_HIDE, SW_SHOWMINIMIZED, WM_USER, SW_SHOW,
        },
    },
};

use crate::{
    find_window_by_classname, find_window_by_title,
    utils::{
        jsonrpc_error_internal_error, jsonrpc_error_invalid_params, jsonrpc_error_method_not_found,
        jsonrpc_success_string,
    },
    AnyResult,
};

const PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\topmost";

/// 键盘管理器
pub struct TopmostManager {
    curr_topmost_hwnd: Arc<Mutex<Option<HWND>>>,

    /// IPC 接口使用 tokio 异步运行时
    rt: Runtime,

    /// IPC 请求响应函数
    on_request: Arc<RwLock<Signal<(Id, JsonRpc), JsonRpc>>>,
}

impl TopmostManager {
    ///
    pub fn new() -> anyhow::Result<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Cannot create tokio runtime for TopmostManager");

        Ok(Self {
            curr_topmost_hwnd: Default::default(),
            rt,
            on_request: Default::default(),
        })
    }

    pub fn after_init(this: &mut Arc<Mutex<Self>>) -> anyhow::Result<()> {
        info!("TopmostManager: init");

        let this2 = this.clone();

        this.lock()
            .on_request
            .write()
            .connect(move |id, req| -> JsonRpc {
                info!("TopmostManager: On request");
                match req.get_method() {
                    Some("set_window_topmost") => {
                        if let Some(Params::Map(map)) = req.get_params() {
                            if let Some(hwnd) = map.get("hwnd") {
                                if let Some(hwnd) = hwnd.as_i64() {
                                    set_window_topmost(HWND(hwnd as isize));
                                    return jsonrpc_success_string(id, "true");
                                }
                            }
                        }
                        jsonrpc_error_invalid_params(id)
                    }
                    Some("unset_window_topmost") => {
                        if let Some(Params::Map(map)) = req.get_params() {
                            if let Some(hwnd) = map.get("hwnd") {
                                if let Some(hwnd) = hwnd.as_u64() {
                                    unset_window_topmost(HWND(hwnd as isize));
                                    return jsonrpc_success_string(id, "true");
                                }
                            }
                        }
                        jsonrpc_success_string(id, "true")
                    }
                    Some("clear_all_windows_topmost") => {
                        this2.lock().clear_current_topmost_window();
                        clear_all_windows_topmost();
                        jsonrpc_success_string(id, "true")
                    }
                    Some("adjust_topmost_on_app_launched") => {
                        if let Some(Params::Map(map)) = req.get_params() {
                            if let Some(pid) = map.get("pid") {
                                if let Some(_pid) = pid.as_u64() {
                                    this2.lock().adjust_topmost_on_app_launched();
                                    return jsonrpc_success_string(id, "true");
                                }
                            }
                        }
                        jsonrpc_success_string(id, "true")
                    }
                    // 临时
                    Some("switch_eink_oled_display") => {
                        crate::switch_eink_oled_display();
                        jsonrpc_success_string(id, "true")
                    }
                    Some(&_) => jsonrpc_error_method_not_found(id),
                    None => jsonrpc_error_internal_error(id),
                }
            });

        this.lock().start_ipc_server()?;

        Ok(())
    }

    /// 新应用程序启动后，调整窗口置顶关系
    /// 1. 取消当前记忆的置顶窗口
    /// 2. 将当前前台应用设为置顶窗口
    fn adjust_topmost_on_app_launched(&mut self) {
        let curr_hwnd = self.curr_topmost_hwnd.clone();
        self.rt.spawn(async move {
            if let Some(hwnd) = curr_hwnd.lock().take() {
                unset_window_topmost(hwnd);
                crate::set_window_minimize(hwnd);
            }

            // 2s 后
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            let hwnd_in_2s = unsafe { GetForegroundWindow() };

            if hwnd_in_2s != HWND(0) {
                set_window_topmost(hwnd_in_2s);
                curr_hwnd.lock().replace(hwnd_in_2s);
            }

            // 重新置顶悬浮球
            crate::find_floating_button_and_set_topmost();
        });
    }

    pub fn clear_current_topmost_window(&mut self) {
        if let Some(hwnd) = self.curr_topmost_hwnd.lock().take() {
            unset_window_topmost(hwnd);
        }
    }

    /// 启动服务
    pub fn start(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// 启动 IPC 服务器
    fn start_ipc_server(&mut self) -> anyhow::Result<()> {
        info!("TopmostManager: start_ipc_server");
        let mut server = eink_pipe_io::server::Server::new(PIPE_NAME);

        let on_request = self.on_request.clone();

        let _ = server.on_connection(move |socket, req| {
            info!("TopmostManager: On connection");
            let on_request2 = on_request.clone();
            socket
                .lock()
                .on_request(move |_socket: Arc<Mutex<Socket>>, id: Id, req: JsonRpc| {
                    info!("TopmostManager: On request: {req:?}");
                    match on_request2.read().emit(id.clone(), req) {
                        Some(reply) => reply,
                        None => JsonRpc::error(id, jsonrpc_lite::Error::internal_error()),
                    }
                });
            0
        });

        // 在异步运行时启动
        self.rt.spawn(async move {
            info!("TopmostManager: start server listen");
            server.listen().await;
            info!("TopmostManager: stop server listen");
        });

        Ok(())
    }

    /// 停止服务
    pub fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// 设置窗口置顶
/// 1. 通知 Topmost Service
pub fn unset_window_topmost(hwnd: HWND) {
    if let Ok(api_hwnd) = find_window_by_classname(s!("AlwaysOnTopWindow")) {
        log::error!("Send Unset Topmost Message To AlwaysOnTopWindow");
        unsafe {
            SendMessageA(api_hwnd, WM_USER + 1, WPARAM::default(), LPARAM(hwnd.0));
        }
    }
}

/// 设置窗口置顶
/// 1. 通知 Topmost Service
pub fn set_window_topmost(hwnd: HWND) {
    if let Ok(api_hwnd) = find_window_by_classname(s!("AlwaysOnTopWindow")) {
        log::error!("Send Topmost Message To AlwaysOnTopWindow");
        unsafe {
            SendMessageA(api_hwnd, WM_USER, WPARAM::default(), LPARAM(hwnd.0));
        }
    }
}

/// 设置窗口置顶
/// 1. 通知 Topmost Service
pub fn clear_all_windows_topmost() {
    if let Ok(api_hwnd) = find_window_by_classname(s!("AlwaysOnTopWindow")) {
        log::error!("Send Clear Topmost Message To AlwaysOnTopWindow");
        unsafe {
            SendMessageA(api_hwnd, WM_USER + 2, WPARAM::default(), LPARAM::default());
        }
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static TOPMOST_MANAGER: Arc<Mutex<TopmostManager>> = {
    info!("Create TopmostManager");

    let mut this = Arc::new(Mutex::new(
        TopmostManager::new().expect("Cannot instantiate TopmostManager"),
    ));

    TopmostManager::after_init(&mut this).unwrap();

    this
};
