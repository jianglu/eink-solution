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
use std::sync::{Arc, Mutex};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

use eink_eventbus::{Event, Listener};

use crate::iddcx::{get_iddcx_device_path, recreate_iddcx_device};

// 虚拟显示器控制器
// 1. 创建虚拟显示器
// 2. 删除虚拟显示器
// 3. 保证只有一个虚拟显示器
pub struct VirtMonServiceImpl {
    dev_path: String,
    curr_mode: u32,
    monitor_id: Option<u32>,
}

impl VirtMonServiceImpl {
    /// 创建服务实例
    pub fn new() -> Result<Self> {
        // 创建驱动程序实例
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED)? };

        recreate_iddcx_device()?;

        // 查找虚拟显示器设备路径
        let dev_path = get_iddcx_device_path()?;
        info!("VirtMonServiceImpl dev_path: {}", &dev_path);

        Ok(Self {
            dev_path,
            curr_mode: 0,
            monitor_id: None,
        })
    }

    /// 启用虚拟显示器
    /// 如果当前虚拟显示器已经启动，检查有效性
    /// 否则创建新虚拟显示器
    pub fn enable_virtual_monitor(&mut self) -> Result<()> {
        info!("启用虚拟显示器：curr_monitor_id: {:?}", self.monitor_id);
        if let Some(monitor_id) = self.monitor_id.take() {
            self.monitor_id = Some(monitor_id);
            // TODO: 检查
        } else {
            // 操作 iddcx 接口，添加虚拟显示器
            self.monitor_id = Some(crate::iddcx::add_monitor(&self.dev_path, 2560, 1600)?);

            // TODO：检查显示器可访问性，这里仅仅 sleep 1000ms 代替
            // 参考 GEN3 代码
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
        Ok(())
    }

    /// 禁用虚拟显示器
    pub fn disable_virtual_monitor(&mut self) -> Result<()> {
        info!("禁用虚拟显示器：curr_monitor_id: {:?}", self.monitor_id);
        if let Some(monitor_id) = self.monitor_id.take() {
            crate::iddcx::remove_monitor(&self.dev_path, monitor_id)?;
            self.monitor_id = None;
        } else {
            // TODO: 检查
        }
        Ok(())
    }

    // /// 模式发生切换
    // pub fn on_mode_switch(&mut self, new_mode: u32) {
    //     if new_mode == 1 || new_mode == 2 {
    //         if self.curr_mode == 0 {
    //             // 模式 1，2 需要创建虚拟显示器
    //         }
    //     } else if self.monitor_id.is_some() {
    //         let monitor_id = self.monitor_id.take();
    //         // 模式 0 不需要创建虚拟显示器
    //     }

    //     // 更新当前模式
    //     self.curr_mode = new_mode;
    //     info!("self.curr_mode : {}", self.curr_mode);

    //     std::thread::sleep(std::time::Duration::from_millis(1000));
    //     info!(
    //         "After 1000 millis sleep, post new message, new_mode: {}",
    //         new_mode
    //     );

    //     // 将热键消息发送至消息总线
    //     EVENTBUS.post(&Event::new(
    //         GENERIC_TOPIC_KEY.clone(),
    //         ModeSwitchMessage2 { mode: new_mode },
    //     ));
    // }
}

impl Drop for VirtMonServiceImpl {
    fn drop(&mut self) {
        // 关闭驱动程序句柄，进程退出时也可以自动关闭
    }
}

#[derive(Clone)]
pub struct VirtualMonitorService {
    inner: Arc<RwLock<VirtMonServiceImpl>>,
}

impl VirtualMonitorService {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            inner: Arc::new(RwLock::new(VirtMonServiceImpl::new()?)),
        })
    }

    pub fn start(&self) -> anyhow::Result<&Self> {
        Ok(self)
    }

    /// 启用虚拟显示器
    /// 如果当前虚拟显示器已经启动，检查有效性
    /// 否则创建新虚拟显示器
    pub fn enable_virtual_monitor(&self) -> Result<()> {
        self.inner.write().enable_virtual_monitor()?;
        Ok(())
    }

    /// 禁用虚拟显示器
    pub fn disable_virtual_monitor(&self) -> Result<()> {
        self.inner.write().disable_virtual_monitor()?;
        Ok(())
    }
}

// impl Listener<RegModeUpdateMessage> for VirtualMonitorService {
//     fn handle(&self, evt: &Event<RegModeUpdateMessage>) {
//         let mut guard = self.inner.lock().unwrap();
//         guard.on_mode_switch(evt.mode);
//     }
// }

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static VIRTUAL_MONITOR_SERVICE: VirtualMonitorService = {
    info!("Create VIRTUAL_MONITOR_SERVICE");
    VirtualMonitorService::new().unwrap()
};
