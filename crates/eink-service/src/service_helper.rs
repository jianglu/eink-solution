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
use parking_lot::Mutex;
use windows::Win32::System::Threading::GetCurrentProcessId;

use crate::{
    utils::{self, get_current_exe_dir},
    win_utils::{run_as_admin, kill_process_by_pid},
};

pub struct ServiceHelper {
    pid: Option<u32>,
}

impl ServiceHelper {
    ///
    pub fn new() -> Result<Self> {
        Ok(Self { pid: None })
    }

    /// 启动服务
    /// 1. 启动 eink-service-helper 进程
    pub fn start(&mut self) -> Result<()> {
        // service-helper 可执行程序和 eink-service 在同一目录
        let exe_dir = get_current_exe_dir();
        let service_helper_exe = exe_dir.join("eink-service-helper.exe");

        let curr_pid = &unsafe { GetCurrentProcessId() }.to_string();

        let mut data_dir = utils::get_current_data_dir();
        data_dir.push("eink-service-helper.json");
        let config_file = data_dir.to_str().unwrap();

        let pid = run_as_admin(
            exe_dir.to_str().unwrap(),
            &format!(
                "\"{}\" --pid {} --config-file \"{}\"",
                service_helper_exe.to_str().unwrap(),
                curr_pid,
                config_file
            ),
        )
        .expect("Cannot start eink-service-helper");

        info!("eink-service-helper pid: {pid}");

        self.pid = Some(pid);

        Ok(())
    }

    ///
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
pub static SERVICE_HELPER: Mutex<ServiceHelper> = {
    info!("Create ServiceHelper");
    Mutex::new(ServiceHelper::new().expect("Cannot instantiate ServiceHelper"))
};
