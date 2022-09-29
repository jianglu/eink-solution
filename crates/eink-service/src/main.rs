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
use log::{error, info};
use std::{ffi::OsString, sync::mpsc::channel, time::Duration};
use vmon::VirtualMonitorManager;
use windows_service::{
    service::*,
    service_control_handler::{self, *},
    service_dispatcher,
};

use eink::EinkServiceManager;
use eink_eventbus::*;

use crate::global::{ServiceControlMessage, EVENTBUS, GENERIC_TOPIC};

//
// Modules
//

mod eink;
mod global;
mod logger;
mod vmon;

//
// Globals
//

const EINK_SERVICE_NAME: &str = "Eink Service";

define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(arguments: Vec<OsString>) {
    info!("{} Started", EINK_SERVICE_NAME);

    if let Err(e) = run_service(arguments) {
        // Handle errors in some way.
        error!("{} Error: {:?}", EINK_SERVICE_NAME, e);
    }
}

// 服务运行在独立的后台线程中
fn run_service(arguments: Vec<OsString>) -> Result<()> {
    // DEBUG
    for arg in arguments {
        println!("Arg: {:?}", &arg);
    }

    // 创建虚拟显示器管理器
    let _vmon_mgr = VirtualMonitorManager::new();

    // 创建 EINK 服务管理器
    let _eink_mgr = EinkServiceManager::new();

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
                    GENERIC_TOPIC.clone(),
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
            _ => {
                println!("Other Control Event Not Implemented");
                ServiceControlHandlerResult::NotImplemented
            }
        }
    };

    // 注册服务事件处理程序
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
