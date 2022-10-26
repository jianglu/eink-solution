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

// 使用 windows subsystem 子系统
#![cfg_attr(not(test), windows_subsystem = "windows")]

//
mod global;
mod ipc_service;
mod magnify;
mod registry_service;
mod win_utils;
mod window_manager_service;
mod winrt;

//
use ipc_service::IPC_SERVICE;
use log::info;
use registry_service::REGISTRY_SERVICE;
use window_manager_service::WINDOW_MANAGER_SERVICE;
use windows::Win32::System::Threading::ExitProcess;
use windows_hotkeys::{
    keys::{ModKey, VKey},
    HotkeyManager,
};

fn main() -> anyhow::Result<()> {
    // 设置当前的活动日志系统为 OutputDebugString 输出
    eink_logger::init_with_level(log::Level::Trace)?;
    reset_current_dir()?;

    // 创建虚拟显示器管理器
    WINDOW_MANAGER_SERVICE.start()?;

    // 创建 IPC 通讯管理器
    IPC_SERVICE.start()?;

    // 启动注册表服务
    REGISTRY_SERVICE.start()?;

    // magnify::start_magnify();

    // let (tx, rx) = std::sync::mpsc::channel();

    // // 将 ctrl-c 响应转化为总线消息，通知各服务
    // ctrlc::set_handler(move || tx.send(()).unwrap()).expect("Error setting Ctrl-C handler");
    // info!("Waiting for Ctrl-C...");

    // rx.recv().expect("Could not receive from channel.");
    // info!("Got it! Exiting...");

    // 开启热键响应线程
    // ALT-SHIFT-B 退出 eink-service
    let mut hkm = HotkeyManager::new();
    hkm.register(VKey::B, &[ModKey::Alt, ModKey::Shift], move || {
        unsafe { ExitProcess(0) };
    })
    .unwrap();
    hkm.event_loop();

    Ok(())
}

fn reset_current_dir() -> anyhow::Result<()> {
    info!("current_dir: {:?}", std::env::current_dir());
    info!("current_exe: {:?}", std::env::current_exe());

    // 更新当前工作目录为 exe 所在目录
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    info!("exe_dir: {:?}", exe_dir);
    std::env::set_current_dir(exe_dir)?;

    Ok(())
}
