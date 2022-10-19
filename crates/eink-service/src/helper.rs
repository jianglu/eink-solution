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

use anyhow::Result;
use log::info;
use parking_lot::RwLock;
use pipe_ipc::blocking::{BlockingIpcConnection, BlockingIpcServer};
use serde_json::json;
use std::{process::Command, sync::Arc, thread::JoinHandle};
use viaduct::{ViaductParent, ViaductTx};

use jsonrpc_lite::JsonRpc;

use crate::win_utils::{kill_process_by_pid, run_as_admin};

// 虚拟显示器控制器
// 1. 创建虚拟显示器
// 2. 删除虚拟显示器
// 3. 保证只有一个虚拟显示器
pub struct HelperServiceImpl {
    helper_pid: Option<u32>,
    shared_conn: Arc<RwLock<Option<BlockingIpcConnection>>>,
    thread: JoinHandle<()>,
}

impl HelperServiceImpl {
    /// 创建服务实例
    pub fn new() -> Result<Self> {
        let shared_conn: Arc<RwLock<Option<BlockingIpcConnection>>> = Arc::new(RwLock::new(None));
        let shared_conn2 = shared_conn.clone();

        let thread = std::thread::spawn(move || {
            let pipe_name = "\\\\.\\pipe\\eink-service-helper";
            let ipc_server = BlockingIpcServer::new(pipe_name).unwrap();
            ipc_server
                .on_connection(move |conn| {
                    info!("\n\nHelperServer: On New Connection");
                    let conn_cloned = conn.clone();
                    *shared_conn2.write() = Some(conn_cloned);
                })
                .detach();
            ipc_server.listen().unwrap();
        });

        // 启动分离进程
        let curr_dir = std::env::current_dir().unwrap();
        let proc_dir = curr_dir.to_str().unwrap();
        let proc_cmd = &format!("{}\\eink-service-helper.exe", proc_dir);
        let helper_pid = Some(run_as_admin(proc_dir, proc_cmd)?);

        info!("helper_pid: {:?}", helper_pid);

        Ok(Self {
            helper_pid,
            shared_conn,
            thread,
        })
    }

    /// 隐藏任务栏
    pub fn hide_taskbar(&mut self) -> Result<()> {
        if let Some(conn) = self.shared_conn.write().as_mut() {
            let resp = conn.call_with_params("hide_taskbar", json!({}))?;
            info!("eink_service: hide_taskbar: {:?}", resp);
        }
        Ok(())
    }

    /// 显示任务栏
    pub fn show_taskbar(&mut self) -> Result<()> {
        if let Some(conn) = self.shared_conn.write().as_mut() {
            let resp = conn.call_with_params("show_taskbar", json!({}))?;
            info!("eink_service: show_taskbar: {:?}", resp);
        }
        Ok(())
    }
}

impl Drop for HelperServiceImpl {
    fn drop(&mut self) {
        if let Some(pid) = self.helper_pid.take() {
            kill_process_by_pid(pid, 0);
        }
    }
}

#[derive(Clone)]
pub struct HelperService {
    inner: Arc<RwLock<HelperServiceImpl>>,
}

impl HelperService {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            inner: Arc::new(RwLock::new(HelperServiceImpl::new()?)),
        })
    }

    pub fn start(&self) -> anyhow::Result<&Self> {
        Ok(self)
    }

    pub fn hide_taskbar(&self) -> Result<()> {
        self.inner.write().hide_taskbar()?;
        Ok(())
    }

    pub fn show_taskbar(&self) -> Result<()> {
        self.inner.write().show_taskbar()?;
        Ok(())
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static HELPER_SERVICE: HelperService = {
    info!("Create HELPER_SERVICE");
    HelperService::new().unwrap()
};
