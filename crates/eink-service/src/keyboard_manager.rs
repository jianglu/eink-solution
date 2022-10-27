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
pub struct KeyboardManager {
    pid: Option<u32>,
}

impl KeyboardManager {
    ///
    pub fn new() -> Result<Self> {
        Ok(Self { pid: None })
    }

    /// 启动服务
    pub fn start(&mut self) -> Result<()> {
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
pub static KEYBOARD_MANAGER: Mutex<KeyboardManager> = {
    info!("Create KeyboardManager");
    Mutex::new(KeyboardManager::new().expect("Cannot instantiate KeyboardManager"))
};
