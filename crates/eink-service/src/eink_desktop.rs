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
use cmd_lib::{run_cmd, run_fun};
use log::info;
use parking_lot::RwLock;
use winapi::um::winuser::GetDesktopWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

/// 桌面管理服务
///
/// 1. 监听合盖事件，如果合盖，切换到 EINK 桌面
///
/// Default         默认桌面
/// EinkDesktop     EINK 专用桌面
///
/// 虚拟桌面控制器
/// 1. 创建虚拟显示器
/// 2. 删除虚拟显示器
/// 3. 保证只有一个虚拟显示器
pub struct EinkDesktopServiceImpl {}

impl EinkDesktopServiceImpl {
    // 创建桌面管理服务
    pub fn new() -> Result<Self> {
        info!("DesktopService::new");
        Ok(Self {})
    }

    /// 启动服务
    /// 1. 创建 EINK_DESKTOP 桌面
    pub fn start(&mut self) {
        unsafe {
            // let desktop_hwnd = GetDesktopWindow();
            // GetWindowThreadProcessId(desktop_hwnd, lpdwprocessid);

            // .\VirtualDesktop11.exe /GetCurrentDesktop
            // let output = run_fun!(VirtualDesktop11.exe "/GetCurrentDesktop");
            // Current desktop: 'Desktop 1' (desktop number 0)
        }
    }

    /// 创建 EINK 虚拟桌面
    pub fn switch_to_eink_desktop(&mut self) -> Result<()> {
        // 删除多余的 EINK_DESKTOP 桌面
        run_cmd!(VirtualDesktop11.exe "/REMOVE:EINK_DESKTOP");
        run_cmd!(VirtualDesktop11.exe "/REMOVE:EINK_DESKTOP");

        // 创建新的 EINK_DESKTOP 桌面
        run_cmd!(VirtualDesktop11.exe "/New" "/Name:EINK_DESKTOP");

        // 切换到新的 EINK_DESKTOP 桌面
        run_cmd!(VirtualDesktop11.exe "/SWITCH:EINK_DESKTOP");
        Ok(())
    }

    /// 切换到标准桌面
    pub fn switch_to_standard_desktop() -> Result<()> {
        // TODO: 将所有在 EINK_DESKTOP 桌面上的 APP 移动到标准桌面，并且最小化

        // 删除所有的的 EINK_DESKTOP 桌面
        run_cmd!(VirtualDesktop11.exe "/REMOVE:EINK_DESKTOP");
        run_cmd!(VirtualDesktop11.exe "/REMOVE:EINK_DESKTOP");

        Ok(())
    }
}

pub struct EinkDesktopService {
    inner: Arc<RwLock<EinkDesktopServiceImpl>>,
}

impl EinkDesktopService {
    // 创建桌面管理服务
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Arc::new(RwLock::new(EinkDesktopServiceImpl::new()?)),
        })
    }

    /// 启动服务
    /// 1. 创建 EINK_DESKTOP 桌面
    pub fn start(&self) {
        unsafe {
            // self.inner.lockck
            // let desktop_hwnd = GetDesktopWindow();
            // GetWindowThreadProcessId(desktop_hwnd, lpdwprocessid);
        }
    }

    /// 创建切换到 EINK 桌面
    pub fn switch_to_eink_desktop(&self) -> Result<()> {
        self.inner.write().switch_to_eink_desktop()?;
        Ok(())
    }

    /// 切换到标准桌面
    pub fn switch_to_standard_desktop(&self) -> Result<()> {
        self.inner.write().switch_to_eink_desktop()?;
        Ok(())
    }
}

// struct EinkDesktopService {
//     inner: Arc<RwLock<EinkDesktopServiceImpl>>,
// }

// impl EinkDesktopService {
//     pub fn new() -> Result<Self> {
//         Ok(Self {
//             inner: EinkDesktopServiceImpl::new()?,
//         })
//     }

//     /// 切换到 EINK 桌面
//     pub fn switch_to_eink_desktop(&self) {
//         self.inner.write().switch_to_eink_desktop()
//     }

//     /// 切换到标准桌面
//     pub fn switch_to_standard_desktop(&self) {
//         self.inner.write().switch_to_standard_desktop()
//     }
// }

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static EINK_DESKTOP_SERVICE: EinkDesktopService = {
    info!("Create EINK_DESKTOP_SERVICE");
    EinkDesktopService::new().unwrap()
};
