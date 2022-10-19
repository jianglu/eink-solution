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

use anyhow::{bail, Result};
use eink_eventbus::*;
use log::info;
use parking_lot::RwLock;
use std::sync::{Arc, Mutex};
use windows::Win32::System::Threading::GetExitCodeProcess;

use crate::capturer::CAPTURER_SERVICE;
use crate::eink_desktop::{EinkDesktopServiceImpl, EINK_DESKTOP_SERVICE};
use crate::global::{EVENTBUS, GENERIC_TOPIC_KEY_NAME};
use crate::virtual_monitor::{VirtualMonitorService, VIRTUAL_MONITOR_SERVICE};
use crate::win_utils::get_process_pid;
use crate::{
    eink_ton::eink_enable,
    global::{RegModeUpdateMessage, ServiceControlMessage},
};
use crate::{win_utils, winrt};

/// Eink 模式
#[derive(Debug, Clone, Copy)]
enum EinkMode {
    // 壁纸模式
    Wallpaper,
    // 系统桌面模式
    WindowsDesktop,
    // 启动器模式
    Launcher,
}

struct EinkServiceImpl {
    // Eink 当前模式(默认为壁纸模式)
    curr_mode: EinkMode,
}

impl EinkServiceImpl {
    /// 创建服务
    pub fn new() -> Result<Self> {
        Ok(Self {
            curr_mode: EinkMode::Wallpaper,
        })
    }

    /// 启动服务
    pub fn start(&mut self) -> Result<()> {
        // 开启 DispFilter 驱动程序
        let curr_dir = std::env::current_dir()?;
        let proc_name = "sc.exe";
        let proc_dir = curr_dir.to_str().unwrap();
        let proc_cmd = "C:\\Windows\\System32\\sc.exe start DispFilter";
        let pid = win_utils::run_as_admin(proc_dir, proc_cmd);

        // TODO：等待进程终止
        std::thread::sleep(std::time::Duration::from_millis(500));

        EINK_DESKTOP_SERVICE.start();

        Ok(())
    }

    /// 切换当前模式
    fn switch_mode(&mut self, new_mode: EinkMode) {
        self.curr_mode = new_mode.clone();

        match new_mode {
            EinkMode::Wallpaper => self.enter_wallpaper_mode(),
            EinkMode::WindowsDesktop => self.enter_windows_desktop_mode(),
            EinkMode::Launcher => self.enter_launcher_mode(),
        }
    }

    /// 进入壁纸模式
    fn enter_wallpaper_mode(&self) {
        info!("进入壁纸模式");

        // 确认虚拟显示器关闭
        VIRTUAL_MONITOR_SERVICE.disable_virtual_monitor();

        // 开启桌面捕获器
        CAPTURER_SERVICE.set_desktop_capturer_status(true);

        // 同步壁纸状态，设置壁纸
        // self.wallpaper_manager.sync()
    }

    /// 进入系统桌面模式
    fn enter_windows_desktop_mode(&self) {
        info!("进入系统桌面模式");

        // 确认虚拟显示器关闭
        VIRTUAL_MONITOR_SERVICE.enable_virtual_monitor();

        // 开启桌面捕获器
        CAPTURER_SERVICE.set_desktop_capturer_status(true);
    }

    // 进入 Launcher 启动器模式
    fn enter_launcher_mode(&mut self) {
        info!("进入 Launcher 启动器模式");

        // 确认虚拟显示器开启
        VIRTUAL_MONITOR_SERVICE.enable_virtual_monitor();

        // 切换桌面
        EINK_DESKTOP_SERVICE.switch_to_eink_desktop();

        // 关闭桌面捕获器
        CAPTURER_SERVICE.set_desktop_capturer_status(false).unwrap();
    }
}

/// EINK 服务
/// 1. EINK 保活
/// 2. EINK 模式管理和切换
#[derive(Clone)]
pub struct EinkService {
    inner: Arc<RwLock<EinkServiceImpl>>,
}

impl EinkService {
    /// 创建 EINK 管理服务
    pub fn new() -> Result<Self> {
        // 每隔 30 秒进行 EINK 保活
        std::thread::spawn(|| loop {
            info!("Start Eink Live Keeper");
            eink_enable();
            std::thread::sleep(std::time::Duration::from_secs(30));
        });

        Ok(Self {
            inner: Arc::new(RwLock::new(EinkServiceImpl::new()?)),
        })
    }

    pub fn start(&self) -> Result<()> {
        self.inner.write().start()?;
        EVENTBUS.register(GENERIC_TOPIC_KEY_NAME, self.clone());
        Ok(())
    }
}

// impl Listener<ServiceControlMessage> for EinkService {
//     fn handle(&self, evt: &Event<ServiceControlMessage>) {}
// }

/// 处理注册表模式切换事件
impl Listener<RegModeUpdateMessage> for EinkService {
    fn handle(&self, evt: &Event<RegModeUpdateMessage>) {
        info!("EinkService 响应注册表模式切换事件");
        let mut inner_guard = self.inner.write();
        match evt.mode {
            0 => inner_guard.switch_mode(EinkMode::Wallpaper),
            1 => inner_guard.switch_mode(EinkMode::WindowsDesktop),
            2 => inner_guard.switch_mode(EinkMode::Launcher),
            _ => (), // ignore,
        }
    }
}

/// 查找 EINK 显示器的 Stable ID
/// TODO: 使用 PID/VID 查找
pub fn find_eink_display_stable_id_with_prefix(prefix: &str) -> Result<String> {
    let manager = winrt::DisplayManager::Create(winrt::DisplayManagerOptions::None)?;
    for (i, t) in manager.GetCurrentTargets()?.into_iter().enumerate() {
        info!("Display[{}] UsageKind: {:?}", i, t.UsageKind()?);

        let monitor_id = t.StableMonitorId()?.to_string();

        info!("Display[{}] {}", i, monitor_id);

        if monitor_id.starts_with(prefix) {
            info!("Find EInk Display {}", monitor_id);
            return Ok(monitor_id);
        }
    }
    bail!("Cannot find eink display stable id");
}

pub fn find_eink_display_stable_id() -> Result<String> {
    match find_eink_display_stable_id_with_prefix("WH") {
        Ok(id) => Ok(id),
        Err(_) => find_eink_display_stable_id_with_prefix("MS_"),
    }
}
