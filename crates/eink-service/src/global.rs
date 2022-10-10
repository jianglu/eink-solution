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

use std::sync::Arc;

use eink_eventbus::*;
use static_init::dynamic;
use windows::Win32::Foundation::HWND;
use windows_service::service::ServiceControl;

#[dynamic]
pub static EVENTBUS: Eventbus = Eventbus::new();

pub const GENERIC_TOPIC_KEY_NAME: &str = "EinkService";

// Topics
#[dynamic]
pub static GENERIC_TOPIC_KEY: TopicKey = TopicKey::from(GENERIC_TOPIC_KEY_NAME);

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

// 捕获窗口消息
#[derive(Debug)]
pub struct CaptureWindowMessage {
    pub hwnd: HWND,
}

// 测试消息
pub struct TestMessage {
    pub hwnd: HWND,
    // pub reply_fn: Arc<Mutex<dyn Fn(i32) + Send + 'a>>,
    pub reply_chan: crossbeam_channel::Sender<i32>,
}

// 带 Reply 的总线消息传递
// {
//     let (tx, rx) = crossbeam_channel::unbounded::<i32>();
//     // 将服务控制消息发送至消息总线
//     EVENTBUS.post(&Event::new(
//         GENERIC_TOPIC_KEY.clone(),
//         TestMessage {
//             hwnd: HWND(0),
//             reply_chan: tx,
//         },
//     ));
//     let reply = rx.recv().unwrap();
//     info!("reply: {}", reply);
// }
