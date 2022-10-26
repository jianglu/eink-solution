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
use parking_lot::Mutex;

pub struct ServiceHelper {}

impl ServiceHelper {
    ///
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    ///
    pub fn start(&mut self) -> Result<()> {
        Ok(())
    }

    ///
    pub fn stop(&mut self) -> Result<()> {
        Ok(())
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static SERVICE_HELPER: Mutex<ServiceHelper> = {
    info!("Create ServiceHelper");
    Mutex::new(ServiceHelper::new().expect("Cannot instantiate ServiceHelper"))
};
