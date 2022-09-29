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

use eink_eventbus::{Event, Listener};

use crate::global::{ModeSwitchMessage, EVENTBUS, GENERIC_TOPIC, GENERIC_TOPIC_KEY};

struct CapturerServiceImpl {}

impl CapturerServiceImpl {
    /// 构造方法
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    /// 模式发生切换
    pub fn on_mode_switch(&mut self, new_mode: u32) {
        info!("CapturerServiceImpl::on_mode_switch({})", new_mode);
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

impl Listener<ModeSwitchMessage> for CapturerService {
    fn handle(&self, evt: &Event<ModeSwitchMessage>) {
        let mut guard = self.inner.lock().unwrap();
        guard.on_mode_switch(evt.mode);
    }
}
