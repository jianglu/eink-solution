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

use eink_eventbus::*;
use static_init::dynamic;
use windows_service::service::ServiceControl;

#[dynamic]
pub static EVENTBUS: Eventbus = Eventbus::new();

pub const GENERIC_TOPIC_KEY: &str = "EinkService";

// Topics
#[dynamic]
pub static GENERIC_TOPIC: TopicKey = TopicKey::from(GENERIC_TOPIC_KEY);

// Application Messages

// 服务控制消息
#[derive(Debug)]
pub struct ServiceControlMessage {
    pub control_event: ServiceControl,
}

// 热键消息
#[derive(Debug)]
pub struct HotKeyMessage {}

// 模式切换消息
#[derive(Debug)]
pub struct ModeSwitchMessage {
    pub mode: u32,
}

// 模式切换消息2
#[derive(Debug)]
pub struct ModeSwitchMessage2 {
    pub mode: u32,
}
