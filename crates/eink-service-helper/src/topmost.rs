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
use eink_winkits::get_window_text;
use if_chain::if_chain;
use jsonrpc_lite::{Id, JsonRpc, Params};
use log::info;
use parking_lot::{Mutex, RwLock};
use signals2::{Connect2, Emit2, Signal};
use tokio::runtime::Runtime;
use windows::s;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, PostMessageA, SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
    SWP_SHOWWINDOW, WM_USER,
};

use crate::mode_manager::set_window_topmost;
use crate::utils::{
    jsonrpc_error_internal_error, jsonrpc_error_invalid_params, jsonrpc_error_method_not_found,
    jsonrpc_success_string,
};
use crate::win_utils::{find_window_by_classname, find_window_by_title};

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
            .worker_threads(3)
            .on_thread_start(|| {
                log::info!(
                    "TopmostManager: thread [{:?}] started",
                    std::thread::current().id()
                );
            })
            .on_thread_stop(|| {
                log::info!(
                    "TopmostManager: thread [{:?}] stopping",
                    std::thread::current().id()
                );
            })
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
                    Some("set_window_topmost") => if_chain! {
                        if let Some(Params::Map(map)) = req.get_params();
                        if let Some(hwnd) = map.get("hwnd");
                        if let Some(hwnd) = hwnd.as_i64();
                        then {
                            std::thread::spawn(move || unsafe {
                                set_window_topmost(HWND(hwnd as isize));

                                // 调用 Windows 的置顶方法，需要在异步上下文进行，因为同步 RPC 会造成消息死锁
                                SetWindowPos(
                                    HWND(hwnd as isize),
                                    HWND_TOPMOST,
                                    0,
                                    0,
                                    0,
                                    0,
                                    SWP_NOMOVE | SWP_SHOWWINDOW | SWP_NOSIZE,
                                );
                            });
                            return jsonrpc_success_string(id, "true");
                        } else {
                            jsonrpc_error_invalid_params(id)
                        }
                    },
                    Some("unset_window_topmost") => {
                        if_chain! {
                            if let Some(Params::Map(map)) = req.get_params();
                            if let Some(hwnd) = map.get("hwnd");
                            if let Some(hwnd) = hwnd.as_u64();
                            then {
                                unset_window_topmost(HWND(hwnd as isize));
                                return jsonrpc_success_string(id, "true");
                            }
                        }
                        jsonrpc_success_string(id, "true")
                    }
                    Some("clear_all_windows_topmost") => {
                        // let this2 = this2.clone();
                        // // std::thread::spawn(move || {
                        this2.lock().clear_current_topmost_window();
                        // crate::mode_manager::clear_all_windows_topmost();
                        // });
                        jsonrpc_success_string(id, "true")
                    }
                    Some("adjust_topmost_on_app_launched") => {
                        if_chain! {
                            if let Some(Params::Map(map)) = req.get_params();
                            if let Some(pid) = map.get("pid");
                            if let Some(_pid) = pid.as_u64();
                            then {
                                let this2 = this2.clone();
                                std::thread::spawn(move || {
                                    this2.lock().adjust_topmost_on_app_launched();
                                });
                                jsonrpc_success_string(id, "true")
                            } else {
                                jsonrpc_error_invalid_params(id)
                            }
                        }
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
                log::info!(
                    "unset topmost and hide 'curr_topmost_hwnd': {:?}",
                    get_window_text(hwnd)
                );

                unset_window_topmost(hwnd);
                // 将最小化调整为隐藏
                crate::win_utils::set_window_hidden(hwnd);
            }

            // 2s 后
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            let hwnd_in_2s = unsafe { GetForegroundWindow() };

            log::info!(
                "After 2s, foreground window is : {:?}",
                get_window_text(hwnd_in_2s)
            );

            if hwnd_in_2s != HWND(0) {
                if let Ok(launcher_hwnd) = find_window_by_title(s!(
                    "ThinkbookEinkPlus2A7678FA-39DD-4C1D-8981-34A451919F59"
                )) {
                    // 不能将 Launcher 置顶的
                    if hwnd_in_2s != launcher_hwnd {
                        set_window_topmost(hwnd_in_2s);
                        curr_hwnd.lock().replace(hwnd_in_2s);

                        log::info!(
                            "Set foreground window '{:?}' to curr_topmost_hwnd",
                            get_window_text(hwnd_in_2s)
                        );
                    }
                }
            }

            // 重新置顶悬浮球
            log::info!("find_floating_button_and_set_topmost");
            crate::mode_manager::find_floating_button_and_set_topmost();
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
            PostMessageA(api_hwnd, WM_USER + 1, WPARAM::default(), LPARAM(hwnd.0));
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
