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

use comet_eventbus::*;
use log::{info, warn};
use std::{ffi::OsString, time::Duration};
use windows_service::{
    service::*,
    service_control_handler::{self, *},
    service_dispatcher, Result,
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
    let status_handle =
        service_control_handler::register(EINK_SERVICE_NAME, event_handler).unwrap();

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
    status_handle.set_service_status(next_status).unwrap();

    // Do some work
}

fn main() -> Result<()> {
    log::set_max_level(log::LevelFilter::Trace);

    // Sets the DebuggerLogger as the currently-active logger.
    logger::init();
    logger::output_debug_string("TEST LOGGING !!!");

    info!("Starting {}", EINK_SERVICE_NAME);

    // Register generated `ffi_service_main` with the system and start the service, blocking
    // this thread until the service is stopped.
    service_dispatcher::start(EINK_SERVICE_NAME, ffi_service_main)?;

    Ok(())
}
