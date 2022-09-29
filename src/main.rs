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
use comet_eventbus::*;
use log::{error, info, warn};
use std::{ffi::OsString, time::Duration};
use windows_service::{
    service::*,
    service_control_handler::{self, *},
    service_dispatcher,
};

use crate::global::{MessageA, EVENTBUS, GENERIC_TOPIC};

//
// Modules
//

mod global;
mod logger;

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

fn run_service(arguments: Vec<OsString>) -> Result<()> {
    // The entry point where execution will start on a background thread after a call to
    // `service_dispatcher::start` from `main`.
    for arg in arguments {
        println!("Arg: {:?}", &arg);
    }

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
                let event_a = Event::new(GENERIC_TOPIC.clone(), MessageA { id: 1 });

                EVENTBUS.post(&event_a);

                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Stop | ServiceControl::Preshutdown | ServiceControl::Shutdown => {
                info!("{} ControlEvent: {:?}", EINK_SERVICE_NAME, control_event);
                // 服务退出时
                // 释放相关资源
                ServiceControlHandlerResult::NoError
            }
            _ => {
                println!("Other Control Event Not Implemented");
                ServiceControlHandlerResult::NotImplemented
            }
        }
    };

    // Register system service event handler
    let status_handle = service_control_handler::register(EINK_SERVICE_NAME, event_handler)?;

    let next_status = ServiceStatus {
        // Should match the one from system service registry
        service_type: ServiceType::OWN_PROCESS,
        // The new state
        current_state: ServiceState::Running,
        // Accept stop events when running
        controls_accepted: ServiceControlAccept::STOP,
        // Used to report an error when starting or stopping only, otherwise must be zero
        exit_code: ServiceExitCode::Win32(0),
        // Only used for pending states, otherwise must be zero
        checkpoint: 0,
        // Only used for pending states, otherwise must be zero
        wait_hint: Duration::default(),
        // Unused for setting status
        process_id: None,
    };

    // Tell the system that the service is running now
    status_handle.set_service_status(next_status)?;

    // Do some work

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
