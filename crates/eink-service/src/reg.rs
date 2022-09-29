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

use std::thread::JoinHandle;

use anyhow::Result;
use log::info;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

use eink_eventbus::Event;

use crate::global::{ModeSwitchMessage, EVENTBUS, GENERIC_TOPIC};

pub struct RegistryManagerService {
    _th: Option<JoinHandle<()>>,
}

impl RegistryManagerService {
    pub fn new() -> Result<Self> {
        let h = std::thread::spawn(|| {
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            let reg_key = hklm
                .open_subkey(r#"SOFTWARE\Lenovo\ThinkBookPlusGen4EinkPlus"#)
                .unwrap();

            loop {
                let res = reg_watcher::watch(
                    &reg_key,
                    reg_watcher::filter::REG_LEGAL_CHANGE_FILTER,
                    true,
                    reg_watcher::Timeout::Infinite,
                )
                .unwrap();
                info!("{:?}", res);

                let mode: u32 = reg_key.get_value("Mode").unwrap();
                info!("Current Mode: {}", mode);

                // 将热键消息发送至消息总线
                EVENTBUS.post(&Event::new(
                    GENERIC_TOPIC.clone(),
                    ModeSwitchMessage { mode },
                ));
            }
        });

        Ok(Self { _th: Some(h) })
    }
}

impl Drop for RegistryManagerService {
    fn drop(&mut self) {
        // leave it
        // self.th.take().unwrap().join().unwrap();
    }
}
