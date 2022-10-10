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
use windows::Win32::Foundation::HWND;

use eink_eventbus::{Event, Listener};

use crate::{
    global::{
        CaptureWindowMessage, ModeSwitchMessage, ModeSwitchMessage2, TestMessage, EVENTBUS,
        GENERIC_TOPIC_KEY, GENERIC_TOPIC_KEY_NAME,
    },
    win_utils,
};

struct CapturerServiceImpl {
    capturer_pid: Option<DWORD>,
    app_capturer_pid: Option<DWORD>,

    launcher_pid: Option<DWORD>,
}

impl CapturerServiceImpl {
    /// 构造方法
    pub fn new() -> Result<Self> {
        Ok(Self {
            capturer_pid: None,
            app_capturer_pid: None,
            launcher_pid: None,
        })
    }

    /// 捕获窗口
    pub fn capture_window(&mut self, hwnd: HWND) {
        info!("CapturerServiceImpl::capture_window({:?})", hwnd);

        // 关闭上一次的 APP 捕获器
        let pid = self.app_capturer_pid.take();

        if pid.is_some() {
            win_utils::kill_process_by_pid(pid.unwrap(), 0);
        }

        // 启动 Capturer
        let curr_dir = std::env::current_dir().unwrap();

        let proc_name = "eink-capturer.exe";
        let proc_dir = curr_dir.to_str().unwrap();
        let proc_cmd = &format!("{}\\eink-capturer.exe --window-id {}", proc_dir, hwnd.0);

        let pid = win_utils::run_admin_privilege(proc_name, proc_dir, proc_cmd).unwrap();
        self.app_capturer_pid = Some(pid);

        info!("app capturer pid: {}", pid);
    }

    /// 模式发生切换
    pub fn on_mode_switch(&mut self, new_mode: u32) {
        info!("CapturerServiceImpl::on_mode_switch({})", new_mode);

        match new_mode {
            1 => {
                // 停止 Capturer，显示壁纸
                if let Some(pid) = self.capturer_pid.take() {
                    win_utils::kill_process_by_pid(pid, 0);
                }

                // 停止 Launcher
                if let Some(pid) = self.launcher_pid.take() {
                    win_utils::kill_process_by_pid(pid, 0);
                }

                // 等待虚拟屏幕

                let curr_dir = std::env::current_dir().unwrap();

                let proc_name = "eink-capturer.exe";
                let proc_dir = curr_dir.to_str().unwrap();
                let proc_cmd = &format!("{}\\eink-capturer.exe --primary", proc_dir);

                let pid = win_utils::run_admin_privilege(proc_name, proc_dir, proc_cmd).unwrap();
                self.capturer_pid = Some(pid);
            }
            2 => {
                // EINK 显示 Launcher
                let pid = self.capturer_pid.take();

                if pid.is_some() {
                    win_utils::kill_process_by_pid(pid.unwrap(), 0);
                }

                //
                // EinkService.exe
                // EinkPlus.exe
                // EinkCapturer.exe
                // EinkSettings.exe
                //

                // 启动 Launcher
                let proc_name = "EinkPlus.exe";
                let proc_dir = "C:\\Program Files\\Lenovo\\ThinkBookEinkPlus";
                let proc_cmd = "C:\\Program Files\\Lenovo\\ThinkBookEinkPlus\\EinkPlus.exe";

                info!("proc_name: {}", proc_name);
                info!("proc_dir: {}", proc_dir);
                info!("proc_cmd: {}", proc_cmd);

                let pid = win_utils::run_admin_privilege(proc_name, proc_dir, proc_cmd).unwrap();
                self.launcher_pid = Some(pid);

                info!("launcher pid: {}", pid);

                std::thread::sleep(std::time::Duration::from_millis(2000));

                // 启动 Capturer
                let curr_dir = std::env::current_dir().unwrap();

                let proc_name = "eink-capturer.exe";
                let proc_dir = curr_dir.to_str().unwrap();
                let proc_cmd = &format!(
                    "\"{}\\eink-capturer.exe\" --window-title mainwindow",
                    proc_dir
                );

                let pid = win_utils::run_admin_privilege(proc_name, proc_dir, proc_cmd).unwrap();
                self.capturer_pid = Some(pid);

                info!("capturer pid: {}", pid);
            }
            _ => {
                // 停止 Capturer，显示壁纸
                if let Some(pid) = self.capturer_pid.take() {
                    win_utils::kill_process_by_pid(pid, 0);
                }

                // 停止 Launcher
                if let Some(pid) = self.launcher_pid.take() {
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
        EVENTBUS.register::<ModeSwitchMessage2, &str, CapturerService>(
            GENERIC_TOPIC_KEY_NAME,
            self.clone(),
        );

        EVENTBUS.register::<CaptureWindowMessage, &str, CapturerService>(
            GENERIC_TOPIC_KEY_NAME,
            self.clone(),
        );

        EVENTBUS
            .register::<TestMessage, &str, CapturerService>(GENERIC_TOPIC_KEY_NAME, self.clone());
        Ok(self)
    }
}

impl Listener<ModeSwitchMessage2> for CapturerService {
    fn handle(&self, evt: &Event<ModeSwitchMessage2>) {
        let mut guard = self.inner.lock().unwrap();
        guard.on_mode_switch(evt.mode);
    }
}

/// 响应捕获窗口消息
impl Listener<CaptureWindowMessage> for CapturerService {
    fn handle(&self, evt: &Event<CaptureWindowMessage>) {
        let mut guard = self.inner.lock().unwrap();
        guard.capture_window(evt.hwnd);
    }
}

/// 响应捕获窗口消息
impl Listener<TestMessage> for CapturerService {
    fn handle(&self, evt: &Event<TestMessage>) {
        // (evt.reply_fn).lock()(99);
        (evt.reply_chan).send(99);
    }
}
