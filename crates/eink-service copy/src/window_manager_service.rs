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

use anyhow::Result;
use eink_eventbus::{Event, Listener};
use log::info;
use parking_lot::Mutex;
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::SetParent};

use crate::{
    global::{EinkMode, SetLauncherWindowMessage, EVENTBUS, GENERIC_TOPIC_KEY_NAME},
    ipc_service::IPC_SERVICE,
    magnify,
};

// 窗口管理器
// 1. 管理窗口置顶
pub struct WindowManagerServiceImpl {
    // Eink 当前模式(默认为壁纸模式)
    curr_mode: EinkMode,
    curr_topmost: Option<HWND>,
}

impl WindowManagerServiceImpl {
    /// 创建服务实例
    pub fn new() -> Result<Self> {
        Ok(Self {
            curr_mode: EinkMode::Wallpaper,
            curr_topmost: None,
        })
    }

    /// 启动窗口管理器
    /// 默认为壁纸模式，通过 Tcon 接口设置锁屏壁纸
    pub fn start(&mut self) -> Result<()> {
        Ok(())
    }

    /// 设置窗口置顶
    /// 1. 如果当前有置顶窗口，将其取消置顶（将其父窗口设置为 DesktopWindow）
    /// 2. 如果
    pub fn set_window_topmost(&mut self, hwnd: HWND) -> Result<()> {
        if let Some(hwnd) = self.curr_topmost.take() {
            // reparent_window_to_desktop(hwnd);
        }

        Ok(())
    }

    /// 设置为 Launcher 窗口模式
    /// 1. 仅仅保存
    pub fn switch_to_launcher_mode(&mut self, hwnd: HWND) -> Result<()> {
        info!("Switch To Launcher Mode");

        // 通过或 Tcon 设置为壁纸模式，避免切换过程的中间状态被发现

        // 启动 Magnify
        // info!("magnify::start_magnify");
        // magnify::start_magnify();
        // let magnify_hwnd = magnify::find_magnify_window();
        // info!("magnify_hwnd: {magnify_hwnd:?}");

        // let ret = unsafe { SetParent(hwnd, magnify_hwnd) };
        // info!("SetParent: {ret:?}");

        Ok(())
    }

    /// 设置为 Windows Desktop 桌面模式
    /// 1. 仅仅保存
    pub fn switch_to_window_desktop_mode(&mut self, hwnd: HWND) -> Result<()> {
        info!("Switch To Windows Desktop Mode");

        // 通过或 Tcon 设置为壁纸模式，避免切换过程的中间状态被发现

        // 启动 Magnify
        // info!("magnify::start_magnify");
        // magnify::start_magnify();
        // let magnify_hwnd = magnify::find_magnify_window();
        // info!("magnify_hwnd: {magnify_hwnd:?}");

        // let ret = unsafe { SetParent(hwnd, magnify_hwnd) };
        // info!("SetParent: {ret:?}");

        Ok(())
    }
}

impl Drop for WindowManagerServiceImpl {
    fn drop(&mut self) {
        // 关闭驱动程序句柄，进程退出时也可以自动关闭
    }
}

/// EINK 服务
/// 1. EINK 保活
/// 2. EINK 模式管理和切换
#[derive(Clone)]
pub struct WindowManagerService {
    inner: Arc<Mutex<WindowManagerServiceImpl>>,
}

impl WindowManagerService {
    /// 创建 EINK IPC 服务
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(WindowManagerServiceImpl::new()?)),
        })
    }

    pub fn start(&self) -> Result<()> {
        self.inner.lock().start()?;

        EVENTBUS.register::<SetLauncherWindowMessage, &str, WindowManagerService>(
            GENERIC_TOPIC_KEY_NAME,
            self.clone(),
        );

        // IPC_SERVICE.add_handler("set_topmost_window", || {
        //     self.inner.lock().set_window_topmost(hwnd)
        // });

        Ok(())
    }
}

/// 响应捕获窗口消息
impl Listener<SetLauncherWindowMessage> for WindowManagerService {
    fn handle(&self, evt: &Event<SetLauncherWindowMessage>) {
        self.inner.lock().switch_to_launcher_mode(evt.hwnd.unwrap());
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static WINDOW_MANAGER_SERVICE: WindowManagerService = {
    info!("Create WINDOW_MANAGER_SERVICE");
    WindowManagerService::new().unwrap()
};
