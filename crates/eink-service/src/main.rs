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

// !!当前不使用!!
// 当前只能使用 console 子系统，需要从 runner 获得
//
// 使用 windows subsystem 子系统
// #![cfg_attr(not(test), windows_subsystem = "windows")]

///////////////////////////////////////////////////////////////////////////////
/// Mods
///
mod keyboard_manager;
mod service_helper;
mod settings;
mod tcon_service;
mod topmost_manager;
mod utils;
mod win_utils;

///////////////////////////////////////////////////////////////////////////////
/// Package Imports
///
use log::info;
use service_helper::SERVICE_HELPER;

use crate::{
    keyboard_manager::KEYBOARD_MANAGER, tcon_service::TCON_SERVICE,
    topmost_manager::TOPMOST_MANAGER,
};

///////////////////////////////////////////////////////////////////////////////
/// Functions
///

/// 应用程序主入口
/// 1. 初始化各种服务
/// 2. 等待 runner 程序发送的 CTRL-C 信号以终止程序
fn main() -> anyhow::Result<()> {
    // 设置当前的活动日志系统为 OutputDebugString 输出
    eink_logger::init_with_level(log::Level::Trace)?;

    init_panic_output();
    init_working_dir().expect("Error reset working dir");

    //
    // 启动各种服务
    //

    // 启动服务助手
    SERVICE_HELPER
        .lock()
        .start()
        .expect("Error start SERVICE_HELPER");

    // 启动键盘管理器
    KEYBOARD_MANAGER
        .lock()
        .start()
        .expect("Error start KEYBOARD_MANAGER");

    // 启动窗口置顶管理
    TOPMOST_MANAGER
        .lock()
        .start()
        .expect("Error start TOPMOST_MANAGER");

    // 启动 TCON 管理器
    TCON_SERVICE
        .lock()
        .start()
        .expect("Error start TCON_SERVICE");

    //
    // 等待 CTRL-C 信号，通知服务终止
    info!("Start waiting for Ctrl-C ...");
    let (tx, rx) = std::sync::mpsc::channel();

    ctrlc::set_handler(move || tx.send(()).unwrap()).expect("Error setting Ctrl-C handler");
    info!("Waiting for Ctrl-C ...");

    rx.recv().expect("Could not receive from channel.");
    info!("Got Ctrl-C, Exiting ...");

    // 依次终止各服务，尽量不要有先后相关性依赖

    KEYBOARD_MANAGER
        .lock()
        .stop()
        .expect("Error stop KEYBOARD_MANAGER");

    TOPMOST_MANAGER
        .lock()
        .stop()
        .expect("Error stop TOPMOST_MANAGER");

    SERVICE_HELPER
        .lock()
        .stop()
        .expect("Error stop SERVICE_HELPER");

    TCON_SERVICE.lock().stop().expect("Error stop TCON_SERVICE");

    Ok(())
}

/// 初始化 Panic 的输出为 OutputDebugString
fn init_panic_output() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {:?}", info);
    }));
}

/// 重置当前工作目录为 exe 所在目录
fn init_working_dir() -> anyhow::Result<()> {
    info!("current_dir: {:?}", std::env::current_dir());
    info!("current_exe: {:?}", std::env::current_exe());

    // 更新当前工作目录为 exe 所在目录
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    info!("exe_dir: {:?}", exe_dir);

    std::env::set_current_dir(exe_dir)?;

    Ok(())
}
