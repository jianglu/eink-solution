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

#![recursion_limit = "1024"]

///////////////////////////////////////////////////////////////////////////////
/// Mods
///
mod keyboard_manager;
mod service_helper;
mod service_main;
mod settings;
mod tcon_service;
mod topmost_manager;
mod utils;
mod win_utils;

use std::ffi::OsString;

use anyhow::bail;
///////////////////////////////////////////////////////////////////////////////
/// Package Imports
///
use log::info;
use service_helper::SERVICE_HELPER;

use crate::keyboard_manager::KEYBOARD_MANAGER;
use crate::tcon_service::TCON_SERVICE;
use crate::topmost_manager::TOPMOST_MANAGER;

///////////////////////////////////////////////////////////////////////////////
/// Functions
///

/// 初始化 Panic 的输出为 OutputDebugString
fn init_panic_output() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("PANIC: {:?}, BACKTRACE: {:?}", info, backtrace);
    }));
}

/// 重置当前工作目录为 exe 所在目录
fn init_working_dir() -> anyhow::Result<()> {
    info!("current_dir: {:?}", std::env::current_dir());
    info!("current_exe: {:?}", std::env::current_exe());

    // 更新当前工作目录为 exe 所在目录
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    info!("set_current_dir: {:?}", exe_dir);

    std::env::set_current_dir(exe_dir)?;

    Ok(())
}

windows_service::define_windows_service!(ffi_service_main, service_main);

fn service_main(arguments: Vec<OsString>) -> anyhow::Result<()> {
    log::info!("\n\nEinkService Start !\n");

    // 服务正常启动的前置检查
    // explorer.exe 必须已经启动
    loop {
        let res = win_utils::get_process_pid("explorer.exe");
        if res.is_err() || res.unwrap() == 0 {
            std::thread::sleep(std::time::Duration::from_secs(3));
            log::info!("Cannot found explorer process, wait for 3 secs");
        } else {
            break;
        }
    }

    // 前置条件准备就绪
    // 1. 如果是新启动计算机，当前 User 并没有准备就绪，service 上半部先执行，等待时机执行下半部
    // 2. 如果是重启等情况，当前 User 会很快准备好，service 下半部会很快得到执行机会
    if let Err(e) = service_main::run_service(arguments) {
        // Handle errors in some way.
        log::error!("ERROR: {:?}", e);
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    //
    // 初始化日志系统
    eink_logger::init_with_level(log::Level::Trace)?;

    // 设置 PANIC 错误输出
    init_panic_output();
    init_working_dir().expect("Error reset working dir");

    // 根据启动参数，判断功能
    // --install 安装服务
    // --uninstall 卸载服务

    // Register generated `ffi_service_main` with the system and start the service, blocking
    // this thread until the service is stopped.
    windows_service::service_dispatcher::start("EinkService", ffi_service_main)?;
    Ok(())
}
