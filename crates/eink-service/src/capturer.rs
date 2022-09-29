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

use anyhow::{Ok, Result};
use log::info;
use std::sync::{Arc, Mutex};
use winapi::shared::minwindef::DWORD;

use eink_eventbus::{Event, Listener};

use crate::{
    global::{ModeSwitchMessage, ModeSwitchMessage2, EVENTBUS, GENERIC_TOPIC, GENERIC_TOPIC_KEY},
    win_utils,
};

struct CapturerServiceImpl {
    pid: Option<DWORD>,
}

impl CapturerServiceImpl {
    /// 构造方法
    pub fn new() -> Result<Self> {
        Ok(Self { pid: None })
    }

    /// 模式发生切换
    pub fn on_mode_switch(&mut self, new_mode: u32) {
        info!("CapturerServiceImpl::on_mode_switch({})", new_mode);

        match new_mode {
            1 => {
                // EINK 显示桌面
                let pid = self.pid.take();

                if pid.is_some() {
                    win_utils::kill_process_by_pid(pid.unwrap(), 0);
                }

                // 等待虚拟屏幕

                let curr_dir = std::env::current_dir().unwrap();

                let proc_name = "eink-capturer.exe";
                let proc_dir = curr_dir.to_str().unwrap();
                let proc_cmd = &format!("{}\\eink-capturer.exe --primary", proc_dir);

                info!("proc_name: {}", proc_name);
                info!("proc_dir: {}", proc_dir);
                info!("proc_cmd: {}", proc_cmd);

                let pid = win_utils::run_admin_privilege(proc_name, proc_dir, proc_cmd).unwrap();

                self.pid = Some(pid);
            }
            2 => {
                // EINK 显示 Launcher
                let pid = self.pid.take();

                if pid.is_some() {
                    win_utils::kill_process_by_pid(pid.unwrap(), 0);
                }

                let curr_dir = std::env::current_dir().unwrap();

                let proc_name = "eink-capturer.exe";
                let proc_dir = curr_dir.to_str().unwrap();
                let proc_cmd = &format!("{}\\eink-capturer.exe --window Edge", proc_dir);

                info!("proc_name: {}", proc_name);
                info!("proc_dir: {}", proc_dir);
                info!("proc_cmd: {}", proc_cmd);

                let pid = win_utils::run_admin_privilege(proc_name, proc_dir, proc_cmd).unwrap();

                info!("pid: {}", pid);

                self.pid = Some(pid);
            }
            _ => {
                // 停止捕获，显示壁纸
                let pid = self.pid.take();

                if pid.is_some() {
                    let pid = pid.unwrap();
                    info!("kill_process_by_pid: {}", pid);
                    win_utils::kill_process_by_pid(pid, 0);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct CapturerService {
    inner: Arc<Mutex<CapturerServiceImpl>>,
}

impl CapturerService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(CapturerServiceImpl::new()?)),
        })
    }
    pub fn start(&self) -> Result<&Self> {
        EVENTBUS.register(GENERIC_TOPIC_KEY, self.clone());
        Ok(self)
    }
}

impl Listener<ModeSwitchMessage2> for CapturerService {
    fn handle(&self, evt: &Event<ModeSwitchMessage2>) {
        let mut guard = self.inner.lock().unwrap();
        guard.on_mode_switch(evt.mode);
    }
}
