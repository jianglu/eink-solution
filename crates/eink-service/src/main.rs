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

#[macro_use]
extern crate windows_service;

use anyhow::Result;
use log::{debug, error, info};
use parking_lot::Mutex;
use std::{
    ffi::OsString,
    sync::{mpsc::channel, Arc},
    time::Duration,
};
use winapi::um::winuser::HWND_DESKTOP;
use windows::Win32::Foundation::HWND;
use windows_service::{
    service::*,
    service_control_handler::{self, *},
    service_dispatcher,
};

use eink::EinkService;
use eink_eventbus::*;

use crate::{
    capturer::CapturerService, composer::ComposerService, global::TestMessage, vmon::VirtMonService,
};
use crate::{
    global::{ServiceControlMessage, EVENTBUS, GENERIC_TOPIC_KEY},
    wmi::WmiService,
};
use crate::{ipc::IpcService, reg::RegistryManagerService};

//
// Modules
//

mod capturer;
mod composer;
mod disp_filter;
mod eink;
mod eink_ton;
mod global;
mod iddcx;
mod ipc;
mod logger;
mod reg;
mod vmon;
mod win_utils;
mod winrt;
mod wmi;

//
// Globals
//

const EINK_SERVICE_NAME: &str = "Eink Service";

define_windows_service!(ffi_service_main, eink_service_main);

fn eink_service_main(arguments: Vec<OsString>) {
    info!("{} Started", EINK_SERVICE_NAME);

    if let Err(e) = run_service(arguments) {
        // Handle errors in some way.
        error!("{} Error: {:?}", EINK_SERVICE_NAME, e);
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static IPC_SERVICE: IpcService = {
    info!("IpcService::new");
    IpcService::new().unwrap()
};

// 服务运行在独立的后台线程中
fn run_service(arguments: Vec<OsString>) -> Result<()> {
    // DEBUG
    for arg in arguments {
        info!("Arg: {:?}", &arg);
    }

    info!("current_dir: {:?}", std::env::current_dir());
    info!("current_exe: {:?}", std::env::current_exe());

    // 更新当前工作目录为 exe 所在目录
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    info!("exe_dir: {:?}", exe_dir);
    std::env::set_current_dir(exe_dir)?;

    // 创建虚拟显示器管理器
    info!("VirtualMonitorService::new");
    let virtmon_srv = VirtMonService::new()?;
    virtmon_srv.start()?;

    // 创建 EINK 服务管理器
    info!("EinkService::new");
    let _eink_srv = EinkService::new();

    // 创建热键管理器
    info!("EinkService::new");
    let _reg_srv = RegistryManagerService::new()?;

    // 创建合成器服务
    info!("ComposerService::new");
    let _composer_srv = ComposerService::new()?;

    // 创建捕获器
    info!("CapturerService::new");
    let capturer_srv = CapturerService::new()?;
    capturer_srv.start()?;

    // 创建 WMI 管理器
    info!("WmiService::new");
    let _wmi_srv = WmiService::new()?;

    // 创建 IPC 通讯管理器
    IPC_SERVICE.start();

    // 本地消息通道，将异步事件递交至本地执行上下文
    let (tx, rx) = channel::<ServiceStatus>();

    // 处理服务控制事件
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Pause => {
                info!("{} ControlEvent: Pause", EINK_SERVICE_NAME);

                // 更新服务状态
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Continue => {
                info!("{} ControlEvent: Continue", EINK_SERVICE_NAME);

                // 更新服务状态

                // 将服务控制消息发送至消息总线
                EVENTBUS.post(&Event::new(
                    GENERIC_TOPIC_KEY.clone(),
                    ServiceControlMessage { control_event },
                ));

                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Stop | ServiceControl::Preshutdown | ServiceControl::Shutdown => {
                info!("{} ControlEvent: {:?}", EINK_SERVICE_NAME, control_event);

                // 服务退出时

                // 释放相关资源

                // 更新服务状态
                match tx.send(ServiceStatus {
                    service_type: ServiceType::OWN_PROCESS,
                    current_state: ServiceState::Stopped,
                    controls_accepted: ServiceControlAccept::STOP,
                    exit_code: ServiceExitCode::Win32(0),
                    checkpoint: 0,
                    wait_hint: Duration::default(),
                    process_id: None,
                }) {
                    Ok(_) => ServiceControlHandlerResult::NoError,
                    Err(e) => {
                        error!("SendError: {:?}", e);
                        ServiceControlHandlerResult::Other(0)
                    }
                }
            }
            // 设备发生变化
            // SERVICE_CONTROL_DEVICEEVENT
            _ => {
                debug!("Other Control Event Not Implemented");
                ServiceControlHandlerResult::NotImplemented
            }
        }
    };

    // 注册服务事件处理程序
    info!("service_control_handler::register");
    let status_handle = service_control_handler::register(EINK_SERVICE_NAME, event_handler)?;

    // 当前状态为 Running
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    // 通知 Windows 系统，当前的服务状态
    info!("set_service_status({:?})", next_status);
    status_handle.set_service_status(next_status)?;

    // 接收状态
    loop {
        if let Ok(next_status) = rx.recv() {
            status_handle.set_service_status(next_status)?;
        } else {
            break;
        }
    }

    // TODO: 其他清理工作

    Ok(())
}

fn main() -> Result<()> {
    // TODO: 当前设置日志级别为全部输出
    log::set_max_level(log::LevelFilter::Trace);

    // 设置当前的活动日志系统为 OutputDebugString 输出
    logger::init();

    info!("{} Starting", EINK_SERVICE_NAME);

    // 注册系统服务，阻塞当前线程直到服务退出
    service_dispatcher::start(EINK_SERVICE_NAME, ffi_service_main)?;

    info!("{} was stopped", EINK_SERVICE_NAME);

    Ok(())
}
