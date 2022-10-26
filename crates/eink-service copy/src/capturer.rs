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
use parking_lot::RwLock;
use std::sync::{Arc, Mutex};
use winapi::shared::minwindef::DWORD;
use windows::{
    core::HSTRING,
    w,
    Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{FindWindowA, FindWindowW},
    },
};

use eink_eventbus::{Event, Listener};

use crate::{
    global::{
        CaptureWindowMessage, ModeSwitchMessage2, RegModeUpdateMessage, TestMessage, EVENTBUS,
        GENERIC_TOPIC_KEY, GENERIC_TOPIC_KEY_NAME,
    },
    win_utils::{self, kill_process_by_pid, run_as_admin},
};

struct CapturerServiceImpl {
    // 其他应用捕获器
    app_capturer_pid: Option<DWORD>,

    // 桌面捕获器
    desktop_capturer_pid: Option<DWORD>,

    // 启动器捕获器
    launcher_pid: Option<DWORD>,
    launcher_capturer_pid: Option<DWORD>,
}

impl CapturerServiceImpl {
    /// 构造方法
    pub fn new() -> Result<Self> {
        Ok(Self {
            app_capturer_pid: None,
            desktop_capturer_pid: None,
            launcher_pid: None,
            launcher_capturer_pid: None,
        })
    }

    /// 设置桌面捕获器状态
    pub fn set_desktop_capturer_status(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            if self.desktop_capturer_pid.is_none() {
                let curr_dir = std::env::current_dir().unwrap();

                let proc_name = "eink-capturer.exe";
                let proc_dir = curr_dir.to_str().unwrap();
                let proc_cmd = &format!("{}\\eink-capturer.exe --primary --band 0", proc_dir);

                let pid = run_as_admin(proc_dir, proc_cmd).unwrap();
                self.desktop_capturer_pid = Some(pid);

                info!("desktop_capturer_pid: {}", pid);
            }
        } else {
            // 关闭桌面捕获
            if let Some(pid) = self.desktop_capturer_pid.take() {
                kill_process_by_pid(pid, 0);
            }
        }

        Ok(())
    }

    /// 捕获窗口
    pub fn capture_app(&mut self, cmdline: &str) {
        info!("CapturerServiceImpl::capture_app({:?})", cmdline);

        // 关闭上一次的 APP 捕获器
        let pid = self.app_capturer_pid.take();

        if pid.is_some() {
            kill_process_by_pid(pid.unwrap(), 0);
        }

        // 启动 Capturer
        let curr_dir = std::env::current_dir().unwrap();

        let proc_name = "eink-capturer.exe";
        let proc_dir = curr_dir.to_str().unwrap();
        let proc_cmd = &format!(
            "{}\\eink-capturer.exe --command-line \"{}\" --band 2",
            proc_dir, cmdline
        );

        let pid = run_as_admin(proc_dir, proc_cmd).unwrap();
        self.app_capturer_pid = Some(pid);

        info!("app capturer pid: {}", pid);
    }

    /// 捕获窗口
    pub fn capture_window(&mut self, hwnd: HWND) {
        info!("CapturerServiceImpl::capture_window({:?})", hwnd);

        // 关闭上一次的 APP 捕获器
        let pid = self.app_capturer_pid.take();

        if pid.is_some() {
            kill_process_by_pid(pid.unwrap(), 0);
        }

        // 启动 Capturer
        let curr_dir = std::env::current_dir().unwrap();

        let proc_name = "eink-capturer.exe";
        let proc_dir = curr_dir.to_str().unwrap();
        let proc_cmd = &format!(
            "{}\\eink-capturer.exe --window-id {} --band 2",
            proc_dir, hwnd.0
        );

        let pid = run_as_admin(proc_dir, proc_cmd).unwrap();
        self.app_capturer_pid = Some(pid);

        info!("app capturer pid: {}", pid);

        // std::thread::spawn(|| {
        //     // 超时，判断窗口是否存在
        //     if let Some(hwnd) = hwnd {
        //         if IsWindow(hwnd) == BOOL(0) {
        //             error!("Windows is exist: exit");
        //             break;
        //         }
        //     }
        // });
    }

    /// 模式发生切换
    pub fn on_mode_switch(&mut self, new_mode: u32) {
        info!("CapturerServiceImpl::on_mode_switch({})", new_mode);

        match new_mode {
            1 => {
                // Mode1: 原生桌面模式，需要虚拟桌面，不捕获 Launcher, App
                info!("Mode1: 原生桌面模式，需要虚拟桌面，不捕获 Launcher, App");

                if let Some(pid) = self.app_capturer_pid.take() {
                    kill_process_by_pid(pid, 0);
                }

                // 停止 Launcher
                if let Some(pid) = self.launcher_pid.take() {
                    kill_process_by_pid(pid, 0);
                }

                // 启动桌面捕获
                if self.desktop_capturer_pid.is_none() {
                    let curr_dir = std::env::current_dir().unwrap();

                    let proc_name = "eink-capturer.exe";
                    let proc_dir = curr_dir.to_str().unwrap();
                    let proc_cmd = &format!("{}\\eink-capturer.exe --primary --band 0", proc_dir);

                    // winproc::run_as_system
                    // winproc::run_as_user
                    // winproc::run_as_admin

                    let pid = run_as_admin(proc_dir, proc_cmd).unwrap();
                    self.desktop_capturer_pid = Some(pid);

                    info!("desktop_capturer_pid: {}", pid);
                }
            }
            2 => {
                // Mode2: 应用置顶模式，需要虚拟桌面
                info!("Mode2: 应用置顶模式，需要虚拟桌面");

                // 关闭 App Capturer
                if let Some(pid) = self.app_capturer_pid.take() {
                    kill_process_by_pid(pid, 0);
                }

                // 关闭桌面捕获
                if let Some(pid) = self.desktop_capturer_pid.take() {
                    kill_process_by_pid(pid, 0);
                }

                // 关闭桌面捕获
                if let Some(pid) = self.launcher_capturer_pid.take() {
                    kill_process_by_pid(pid, 0);
                }

                // 重新动 Launcher
                let proc_name = "EinkPlus.exe";
                let proc_dir = "C:\\Program Files\\Lenovo\\ThinkBookEinkPlus";
                let proc_cmd = "C:\\Program Files\\Lenovo\\ThinkBookEinkPlus\\EinkPlus.exe";

                info!("proc_name: {}", proc_name);
                info!("proc_dir: {}", proc_dir);
                info!("proc_cmd: {}", proc_cmd);

                let pid = run_as_admin(proc_dir, proc_cmd).unwrap();
                self.launcher_pid = Some(pid);

                info!("launcher pid: {}", pid);

                // // 启动桌面捕获
                // if self.desktop_capturer_pid.is_none() {
                //     let curr_dir = std::env::current_dir().unwrap();

                //     let proc_name = "eink-capturer.exe";
                //     let proc_dir = curr_dir.to_str().unwrap();
                //     let proc_cmd = &format!("{}\\eink-capturer.exe --primary --band 0", proc_dir);

                //     // winproc::run_as_system
                //     // winproc::run_as_user
                //     // winproc::run_as_admin

                //     let pid = run_as_admin(proc_name, proc_dir, proc_cmd).unwrap();
                //     self.desktop_capturer_pid = Some(pid);

                //     info!("desktop_capturer_pid: {}", pid);
                // }

                // info!("等待 Launcher 启动, 10s");

                // 等待 Launcher 启动, 10s
                // unsafe {
                //     for _ in 0..10 {
                //         let hwnd = FindWindowW(None, w!("ThinkBookEinkPlus\0"));

                //         if hwnd != HWND(0) {
                //             break;
                //         }

                //         info!("ThinkBookEinkPlus {:?}", hwnd);

                std::thread::sleep(std::time::Duration::from_secs(2));
                //     }
                // }

                info!("启动 Launcher Capturer {:?}", self.launcher_capturer_pid);

                // 启动 Launcher Capturer
                if self.launcher_capturer_pid.is_none() {
                    let curr_dir = std::env::current_dir().unwrap();

                    let proc_name = "eink-capturer.exe";
                    let proc_dir = curr_dir.to_str().unwrap();
                    let proc_cmd = &format!(
                        "\"{}\\eink-capturer.exe\" --window-title ThinkBookEinkPlus --band 1",
                        proc_dir
                    );

                    let pid = run_as_admin(proc_dir, proc_cmd).unwrap();
                    self.launcher_capturer_pid = Some(pid);

                    info!("launcher_capturer_pid: {}", pid);
                }
            }
            _ => {
                // Mode0: 停止所有 Capturer，显示壁纸
                info!("Mode0: 停止所有 Capturer，显示壁纸");

                if let Some(pid) = self.desktop_capturer_pid.take() {
                    kill_process_by_pid(pid, 0);
                }

                if let Some(pid) = self.launcher_capturer_pid.take() {
                    kill_process_by_pid(pid, 0);
                }

                // 停止 Launcher
                if let Some(pid) = self.launcher_pid.take() {
                    kill_process_by_pid(pid, 0);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct CapturerService {
    inner: Arc<RwLock<CapturerServiceImpl>>,
}

impl CapturerService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Arc::new(RwLock::new(CapturerServiceImpl::new()?)),
        })
    }

    /// 关闭所有捕获器
    pub fn close_all_capturer(&self) -> Result<()> {
        Ok(())
    }

    /// 设置桌面捕获器状态
    pub fn set_desktop_capturer_status(&self, enabled: bool) -> Result<()> {
        self.inner.write().set_desktop_capturer_status(enabled)?;
        Ok(())
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

        EVENTBUS.register::<TestMessage, &str, CapturerService>(
            GENERIC_TOPIC_KEY_NAME, //
            self.clone(),
        );
        Ok(self)
    }
}

impl Listener<ModeSwitchMessage2> for CapturerService {
    fn handle(&self, evt: &Event<ModeSwitchMessage2>) {
        info!("Capturer got ModeSwitchMessage2, mode: {}", evt.mode);
        let mut guard = self.inner.write();
        guard.on_mode_switch(evt.mode);
    }
}

/// 响应捕获窗口消息
impl Listener<CaptureWindowMessage> for CapturerService {
    fn handle(&self, evt: &Event<CaptureWindowMessage>) {
        let mut guard = self.inner.write();
        if evt.hwnd.is_some() {
            guard.capture_window(evt.hwnd.unwrap());
        } else if evt.cmdline.is_some() {
            let cmdline = evt.cmdline.as_ref().unwrap().to_owned();
            guard.capture_app(&cmdline);
        }
    }
}

/// 响应捕获窗口消息
impl Listener<TestMessage> for CapturerService {
    fn handle(&self, evt: &Event<TestMessage>) {
        // (evt.reply_fn).lock()(99);
        (evt.reply_chan).send(99);
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static CAPTURER_SERVICE: CapturerService = {
    info!("Create CAPTURER_SERVICE");
    CapturerService::new().unwrap()
};
