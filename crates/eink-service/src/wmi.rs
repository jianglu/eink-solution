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

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use log::info;
use wmi::{COMLibrary, Variant, WMIConnection};

use eink_eventbus::Event;

use crate::global::{RegModeUpdateMessage, EVENTBUS, GENERIC_TOPIC_KEY};

struct WmiServiceImpl {}

impl WmiServiceImpl {
    pub fn new() -> anyhow::Result<Self> {
        std::thread::spawn(|| {
            let com_con = COMLibrary::new().unwrap();
            let wmi_con = WMIConnection::with_namespace_path("root/wmi", com_con.into()).unwrap();

            let iterator = wmi_con
                .raw_notification::<HashMap<String, Variant>>("SELECT * FROM Lenovo_LidEvent")
                .unwrap();

            // WBEM_E_UNPARSABLE_QUERY 0x80041058
            for result in iterator {
                let result = result.unwrap();
                let status = result.get("ULong").unwrap();
                if let Variant::UI4(status) = status {
                    info!("Lenovo_LidEvent: status: {:?}", status);

                    if status == &0 {
                        // 将热键消息发送至消息总线
                        EVENTBUS.post(&Event::new(
                            GENERIC_TOPIC_KEY.clone(),
                            RegModeUpdateMessage { mode: 2 },
                        ));
                    } else if status == &1 {
                        // 将热键消息发送至消息总线
                        EVENTBUS.post(&Event::new(
                            GENERIC_TOPIC_KEY.clone(),
                            RegModeUpdateMessage { mode: 0 },
                        ));
                    }
                }
            }
        });

        Ok(Self {})
    }
}

pub struct WmiService {
    inner: Arc<Mutex<WmiServiceImpl>>,
}

impl WmiService {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(WmiServiceImpl::new()?)),
        })
    }
}
