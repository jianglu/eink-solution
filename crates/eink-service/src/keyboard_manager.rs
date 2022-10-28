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
    process::{Child, Command},
    sync::Arc,
};

use anyhow::Result;
use cmd_lib::run_cmd;
use eink_pipe_io::server::Socket;
use jsonrpc_lite::{Id, JsonRpc};
use log::info;
use parking_lot::{Mutex, RwLock};
use signals2::{Connect2, Emit2, Signal};
use tokio::runtime::Runtime;
use windows::Win32::System::Threading::GetCurrentProcessId;

use crate::{
    settings::SETTINGS,
    utils::{
        get_current_data_dir, get_current_exe_dir, jsonrpc_error_internal_error,
        jsonrpc_error_method_not_found, jsonrpc_success_string,
    },
    win_utils::{kill_process_by_pid, run_as_admin},
};

const PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\keyboard";

/// 键盘管理器
pub struct KeyboardManager {
    pid: Option<u32>,

    /// IPC 接口使用 tokio 异步运行时
    rt: Runtime,

    /// IPC 请求响应函数
    on_request: Arc<RwLock<Signal<(Id, JsonRpc), JsonRpc>>>,
}

impl KeyboardManager {
    ///
    pub fn new() -> Result<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Cannot create tokio runtime for KeyboardManager");

        Ok(Self {
            pid: None,
            rt,
            on_request: Default::default(),
        })
    }

    pub fn after_init(this: &mut Arc<Mutex<Self>>) -> Result<()> {
        info!("KeyboardManager: init");

        let this2 = this.clone();

        this.lock().on_request.write().connect(move |id, req| {
            info!("KeyboardManager: On request");
            match req.get_method() {
                Some("disable_win_key") => {
                    this2.lock().disable_win_key().unwrap();
                    jsonrpc_success_string(id, "true")
                }
                Some("enable_win_key") => {
                    this2.lock().enable_win_key().unwrap();
                    jsonrpc_success_string(id, "true")
                }
                Some(&_) => jsonrpc_error_method_not_found(id),
                None => jsonrpc_error_internal_error(id),
            }
        });

        this.lock().start_ipc_server()?;

        Ok(())
    }

    /// 启动服务
    pub fn start(&mut self) -> Result<()> {
        Ok(())
    }

    /// 启动 IPC 服务器
    fn start_ipc_server(&mut self) -> Result<()> {
        info!("KeyboardManager: start_ipc_server");
        let mut server = eink_pipe_io::server::Server::new(PIPE_NAME);

        let on_request = self.on_request.clone();

        let _ = server.on_connection(move |socket, req| {
            info!("KeyboardManager: On connection");
            let on_request2 = on_request.clone();
            socket
                .lock()
                .on_request(move |_socket: Arc<Mutex<Socket>>, id: Id, req: JsonRpc| {
                    info!("KeyboardManager: On request: {req:?}");
                    match on_request2.read().emit(id.clone(), req) {
                        Some(reply) => reply,
                        None => JsonRpc::error(id, jsonrpc_lite::Error::internal_error()),
                    }
                });
            0
        });

        // 在异步运行时启动
        self.rt.spawn(async move {
            info!("KeyboardManager: start server listen");
            server.listen().await;
            info!("KeyboardManager: stop server listen");
        });

        Ok(())
    }

    /// 禁用 Win / AltTab 按键
    /// 1. 启动 eink-keyboard-manager 进程
    pub fn disable_win_key(&mut self) -> Result<()> {
        // keyboard manager 可执行程序和 eink-service 在同一目录
        let exe_dir = get_current_exe_dir();
        let keyboard_manager_exe = exe_dir.join("eink-keyboard-manager.exe");

        // .\eink-keyboard-manager.exe /SettingsDir:"C:\Users\JiangLu\AppData\Local\Lenovo\ThinkBookEinkPlus\eink-keyboard-manager"

        let curr_pid = &unsafe { GetCurrentProcessId() }.to_string();

        let setting_dir = exe_dir.join("EinkKeyboardManager");
        let setting_dir = setting_dir.to_str().unwrap();

        // let process = Command::new(keyboard_manager_exe)
        //     .args([
        //         &format!("/Pid={}", curr_pid),
        //         &format!("/SettingsDir={}", setting_dir),
        //     ])
        //     .spawn()
        //     .expect("Cannot spawn keyboard manager instance");

        let pid = run_as_admin(
            exe_dir.to_str().unwrap(),
            &format!(
                "\"{}\" /Pid={} /SettingsDir=\"{}\"",
                keyboard_manager_exe.to_str().unwrap(),
                curr_pid,
                setting_dir
            ),
        )
        .unwrap();

        info!("eink-keyboard-manager pid: {pid}");

        self.pid = Some(pid);

        Ok(())
    }

    /// 启用 Win / AltTab 按键
    pub fn enable_win_key(&mut self) -> Result<()> {
        if let Some(pid) = self.pid.take() {
            kill_process_by_pid(pid, 0);
        }
        Ok(())
    }

    /// 停止服务
    /// 1. 停止 eink-keyboard-manager 进程
    pub fn stop(&mut self) -> Result<()> {
        self.enable_win_key()
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static KEYBOARD_MANAGER: Arc<Mutex<KeyboardManager>> = {
    info!("Create KeyboardManager");

    let mut this = Arc::new(Mutex::new(
        KeyboardManager::new().expect("Cannot instantiate KeyboardManager"),
    ));

    KeyboardManager::after_init(&mut this).unwrap();

    this
};
