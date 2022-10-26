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

use std::{sync::Arc, thread::JoinHandle};

use anyhow::Result;
use cht::HashMap;
use log::info;
use parking_lot::{Mutex, RwLock};
use signals2::{connect::ConnectionImpl, Connect2, Emit0, Emit2, Signal};
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

use eink_eventbus::Event;

use crate::global::{RegModeUpdateMessage, EVENTBUS, GENERIC_TOPIC_KEY};

/// 注册表服务
/// 1. 监控注册表变更
pub struct RegistryServiceImpl {
    thr: Option<JoinHandle<()>>,
    handlers: Arc<HashMap<String, Signal<(String, String)>>>,
}

impl RegistryServiceImpl {
    pub fn new() -> Result<Self> {
        Ok(Self {
            thr: None,
            handlers: Arc::new(HashMap::default()),
        })
    }

    fn get_handler(&mut self, key: &String) -> Signal<(String, String)> {
        match self.handlers.get(key) {
            Some(h) => h,
            None => {
                let sig = Signal::<(String, String)>::new();
                self.handlers.insert(key.to_owned(), sig.clone());
                sig
            }
        }
    }

    pub fn on_change<F>(&mut self, key: &String, cb: F) -> ConnectionImpl<false>
    where
        F: Fn(String, String) + Sync + Send + 'static,
    {
        let handler = self.get_handler(key);
        handler.connect(cb)
    }

    pub fn start(&mut self) -> Result<()> {
        info!("RegistryService::start");

        let handlers = self.handlers.clone();

        self.thr = Some(std::thread::spawn(move || {
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            let reg_key = hklm
                .open_subkey(r#"SOFTWARE\Lenovo\ThinkBookEinkPlus"#)
                .unwrap();

            loop {
                let res = reg_watcher::watch(
                    &reg_key,
                    reg_watcher::filter::REG_LEGAL_CHANGE_FILTER,
                    true,
                    reg_watcher::Timeout::Infinite,
                );

                // let h = handlers.get("key").emit("".to_string(), "".to_string());

                if res.is_ok() {
                    info!("{:?}", res);

                    let mode: u32 = reg_key.get_value("Mode").unwrap();
                    info!("Current Mode: {}", mode);

                    // 将热键消息发送至消息总线
                    EVENTBUS.post(&Event::new(
                        GENERIC_TOPIC_KEY.clone(),
                        RegModeUpdateMessage { mode },
                    ));
                } else {
                    info!("Watch Reg Err: {:?}", res.unwrap_err());
                }
            }
        }));
        Ok(())
    }
}

impl Drop for RegistryServiceImpl {
    fn drop(&mut self) {
        // leave it
        // self.th.take().unwrap().join().unwrap();
    }
}

pub struct RegistryService {
    inner: Arc<Mutex<RegistryServiceImpl>>,
}

impl RegistryService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(RegistryServiceImpl::new()?)),
        })
    }

    pub fn start(&self) -> Result<()> {
        self.inner.lock().start()
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static REGISTRY_SERVICE: RegistryService = {
    info!("RegistryService::new");
    RegistryService::new().unwrap()
};
