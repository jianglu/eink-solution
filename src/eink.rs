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

use comet_eventbus::*;

use crate::global::ServiceControlMessage;

/// 服务控制中心
pub struct EinkServiceManager {}

impl EinkServiceManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl Listener<ServiceControlMessage> for EinkServiceManager {
    fn handle(&self, evt: &Event<ServiceControlMessage>) {}
}
