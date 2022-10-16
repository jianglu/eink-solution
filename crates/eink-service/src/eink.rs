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
use std::sync::{Arc, Mutex};

use crate::winrt;
use crate::{
    eink_ton::eink_enable,
    global::{ModeSwitchMessage, ServiceControlMessage},
};

struct EinkServiceImpl {
    mode: u32,
}

impl EinkServiceImpl {
    /// 切换当前模式
    fn set_mode(&mut self, new_mode: u32) {
        self.mode = new_mode;
    }
}

/// EINK 服务
/// 1. EINK 保活
/// 2. EINK 模式管理和切换
pub struct EinkService {
    inner: Arc<Mutex<EinkServiceImpl>>,
}

impl EinkService {
    /// 创建 EINK 管理服务
    pub fn new() -> Self {
        // 每隔 30 秒进行 EINK 保活
        std::thread::spawn(|| loop {
            info!("Start Eink Live Keeper");
            eink_enable();
            std::thread::sleep(std::time::Duration::from_secs(30));
        });

        Self {
            inner: Arc::new(Mutex::new(EinkServiceImpl { mode: 0 })),
        }
    }
}

impl Listener<ServiceControlMessage> for EinkService {
    fn handle(&self, evt: &Event<ServiceControlMessage>) {}
}

/// 处理模式切换事件
impl Listener<ModeSwitchMessage> for EinkService {
    fn handle(&self, evt: &Event<ModeSwitchMessage>) {
        let mut inner_guard = self.inner.lock().unwrap();
        inner_guard.set_mode(evt.mode);
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
