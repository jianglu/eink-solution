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
use std::sync::{Arc, Mutex};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

use eink_eventbus::{Event, Listener};

use crate::{
    global::{ModeSwitchMessage, EVENTBUS, GENERIC_TOPIC_KEY},
    iddcx::{get_iddcx_device_path, recreate_iddcx_device},
};

// 虚拟显示器控制器
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

    /// 模式发生切换
    pub fn on_mode_switch(&mut self, new_mode: u32) {
        info!("VirtMonServiceImpl::on_mode_switch({})", new_mode);

        if new_mode == 1 || new_mode == 2 {
            if self.curr_mode == 0 {
                // 模式 1，2 需要创建虚拟显示器
                self.monitor_id =
                    Some(crate::iddcx::add_monitor(&self.dev_path, 2560, 1600).unwrap());
            }
        } else if self.monitor_id.is_some() {
            let monitor_id = self.monitor_id.take();
            // 模式 0 不需要创建虚拟显示器
            crate::iddcx::remove_monitor(&self.dev_path, monitor_id.unwrap()).unwrap();
        }
    }
}

impl Drop for VirtMonServiceImpl {
    fn drop(&mut self) {
        // 关闭驱动程序句柄，进程退出时也可以自动关闭
    }
}

#[derive(Clone)]
pub struct VirtMonService {
    inner: Arc<Mutex<VirtMonServiceImpl>>,
}

impl VirtMonService {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(VirtMonServiceImpl::new()?)),
        })
    }

    pub fn start(&self) -> anyhow::Result<&Self> {
        EVENTBUS.register(GENERIC_TOPIC_KEY, self.clone());
        Ok(self)
    }
}

impl Listener<ModeSwitchMessage> for VirtMonService {
    fn handle(&self, evt: &Event<ModeSwitchMessage>) {
        let mut guard = self.inner.lock().unwrap();
        guard.on_mode_switch(evt.mode);
    }
}
