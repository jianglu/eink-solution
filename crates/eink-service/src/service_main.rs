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

use std::ffi::OsString;
use std::time::Duration;

use windows_service::service::{
    HardwareProfileChangeParam, PowerEventParam, ServiceControl, ServiceControlAccept,
    ServiceExitCode, ServiceState, ServiceStatus, ServiceType, SessionChangeParam,
    SessionChangeReason,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};

use crate::keyboard_manager::KEYBOARD_MANAGER;
use crate::service_helper::SERVICE_HELPER;
use crate::tcon_service::TCON_SERVICE;
use crate::topmost_manager::TOPMOST_MANAGER;

/// 服务主入口
/// 1. 初始化各种服务
/// 2. 等待 runner 程序发送的 CTRL-C 信号以终止程序
pub fn run_service(_arguments: Vec<OsString>) -> anyhow::Result<()> {
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

    // 服务退出码
    let mut service_exit_code = ServiceExitCode::NO_ERROR;

    //
    // 服务事件响应
    //
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Continue => {
                log::debug!("ServiceControl::Continue");
                ServiceControlHandlerResult::NoError
            }
            // All services must accept Interrogate even if it's a no-op.
            ServiceControl::Interrogate => {
                log::debug!("ServiceControl::Interrogate");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::NetBindAdd => {
                log::debug!("ServiceControl::NetBindAdd");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::NetBindDisable => {
                log::debug!("ServiceControl::NetBindDisable");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::NetBindEnable => {
                log::debug!("ServiceControl::NetBindEnable");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::NetBindRemove => {
                log::debug!("ServiceControl::NetBindRemove");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::ParamChange => {
                log::debug!("ServiceControl::ParamChange");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Pause => {
                log::debug!("ServiceControl::Pause");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Preshutdown => {
                log::debug!("ServiceControl::Preshutdown");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Shutdown => {
                log::debug!("ServiceControl::Shutdown");

                // 通知主线程继续执行
                shutdown_tx.send(()).unwrap();

                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Stop => {
                log::info!("ServiceControl::Stop");

                // 通知主线程继续执行
                shutdown_tx.send(()).unwrap();

                // 处理服务停止事件，正常返回，将控制权交给系统
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::HardwareProfileChange(param) => {
                log::info!("ServiceControl::HardwareProfileChange: {:?}", param);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::PowerEvent(param) => {
                log::info!("ServiceControl::Continue: {:?}", param);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::SessionChange(param) => {
                log::info!("ServiceControl::SessionChange: param: {:?}", param);
                log::info!(
                    "ServiceControl::SessionChange: param.reason: {:?}",
                    param.reason
                );

                // 用户登录
                if param.reason == SessionChangeReason::SessionLogon {
                    // 启动服务助手
                    if let Err(_err) = SERVICE_HELPER.lock().start() {
                        log::error!("Error start SERVICE_HELPER")
                    }
                }

                ServiceControlHandlerResult::NoError
            }
            ServiceControl::TimeChange => {
                log::info!("ServiceControl::TimeChange");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::TriggerEvent => {
                log::info!("ServiceControl::TriggerEvent");
                ServiceControlHandlerResult::NoError
            }
        }
    };

    // 注册服务状态处理
    let status_handle = service_control_handler::register("EinkService", event_handler)?;

    // 从这里开始，服务处于 START_PENDING 状态

    // 启动服务助手
    if let Err(_err) = SERVICE_HELPER.lock().start() {
        log::error!("Error start SERVICE_HELPER")
    }

    // 启动键盘管理器
    if let Err(_err) = KEYBOARD_MANAGER.lock().start() {
        log::error!("Error start KEYBOARD_MANAGER")
    }

    // 启动窗口置顶管理
    if let Err(_err) = TOPMOST_MANAGER.lock().start() {
        log::error!("Error start TOPMOST_MANAGER")
    }

    // 启动 TCON 管理器
    if let Err(_err) = TCON_SERVICE.lock().start() {
        log::error!("Error start TCON_SERVICE")
    }

    // 服务完整初始化，将服务切换为 RUNNING 状态
    status_handle.set_service_status(ServiceStatus {
        // Should match the one from system service registry
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        // 接受 STOP 服务停止事件
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SESSION_CHANGE,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    log::debug!("Entering main service loop");

    'outer: loop {
        match shutdown_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // 服务切换为 STOP_PENDING 状态
                status_handle.set_service_status(ServiceStatus {
                    service_type: ServiceType::OWN_PROCESS,
                    current_state: ServiceState::StopPending,
                    controls_accepted: ServiceControlAccept::empty(),
                    exit_code: ServiceExitCode::NO_ERROR,
                    checkpoint: 0,
                    wait_hint: std::time::Duration::from_millis(5000),
                    process_id: None,
                })?;

                break 'outer;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (),
        }
    }

    log::debug!("Stop all eink-service modules");

    // 停止各服务模块
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

    // 服务切换为停止状态
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: service_exit_code,
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;

    Ok(())
}
