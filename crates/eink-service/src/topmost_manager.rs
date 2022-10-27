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

use std::process::{Child, Command};

use anyhow::Result;
use cmd_lib::run_cmd;
use log::info;
use parking_lot::Mutex;
use windows::Win32::System::Threading::GetCurrentProcessId;

use crate::{
    settings::SETTINGS,
    utils::{get_current_data_dir, get_current_exe_dir},
    win_utils::{kill_process_by_pid, run_as_admin},
};

/// 键盘管理器
pub struct TopmostManager {
    pid: Option<u32>,
}

impl TopmostManager {
    ///
    pub fn new() -> Result<Self> {
        Ok(Self { pid: None })
    }

    /// 启动服务
    pub fn start(&mut self) -> Result<()> {
        // topmost manager 可执行程序和 eink-service 在同一目录
        let exe_dir = get_current_exe_dir();
        let topmost_manager_exe = exe_dir.join("eink-topmost-manager.exe");

        let curr_pid = &unsafe { GetCurrentProcessId() }.to_string();

        let setting_dir = exe_dir.join("EinkTopmostManager");
        let setting_dir = setting_dir.to_str().unwrap();

        let pid = run_as_admin(
            exe_dir.to_str().unwrap(),
            &format!("\"{}\" {}", topmost_manager_exe.to_str().unwrap(), curr_pid,),
        )
        .unwrap();

        info!("eink-topmost-manager pid: {pid}");

        self.pid = Some(pid);

        Ok(())
    }

    /// 停止服务
    /// 1. 停止 eink-keyboard-manager 进程
    pub fn stop(&mut self) -> Result<()> {
        if let Some(pid) = self.pid.take() {
            kill_process_by_pid(pid, 0);
        }
        Ok(())
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static TOPMOST_MANAGER: Mutex<TopmostManager> = {
    info!("Create TopmostManager");
    Mutex::new(TopmostManager::new().expect("Cannot instantiate TopmostManager"))
};
