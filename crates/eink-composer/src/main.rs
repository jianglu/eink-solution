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

use std::{path::PathBuf, sync::Arc};

use anyhow::{Error, Result};

use clap::Parser;
use log::info;
use ntapi::winapi::um::winioctl::{CTL_CODE, FILE_READ_ACCESS, FILE_WRITE_ACCESS, METHOD_NEITHER};
use parking_lot::Mutex;
use pico_args::Arguments;
use serde::{Deserialize, Serialize};
use surface_flinger::SurfaceFlinger;

mod client;
mod drawing_context;
mod eink;
mod iterable;
mod layer;
mod logger;
mod shader;
mod specialized;
mod surface_composer;
mod surface_flinger;
mod swap_chain;
mod utility;
mod winrt;
mod winrt_ext;

use windows::{
    core::{HRESULT, HSTRING, PCWSTR, PSTR},
    Devices::Display::Core::{DisplayManager, DisplayManagerOptions},
    Win32::{
        Foundation::{CloseHandle, ERROR_SERVICE_DOES_NOT_EXIST, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_ACCESS_FLAGS, FILE_FLAG_NO_BUFFERING, FILE_SHARE_READ,
            FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{
            Services::{
                CloseServiceHandle, CreateServiceW, OpenSCManagerW, OpenServiceW, StartServiceA,
                ENUM_SERVICE_TYPE, SC_MANAGER_ALL_ACCESS, SERVICE_DEMAND_START,
                SERVICE_ERROR_IGNORE, SERVICE_KERNEL_DRIVER,
            },
            SystemServices::{GENERIC_READ, GENERIC_WRITE},
            IO::DeviceIoControl,
        },
    },
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Use test background
    #[clap(long)]
    test_background: Option<bool>,

    /// Add test layer
    #[clap(long)]
    test_layer: Option<bool>,

    /// Stable monitor id
    #[clap(long)]
    monitor_id: String,
}

fn main() -> Result<()> {
    // 初始化日志模块
    // env_logger::init();

    log::set_max_level(log::LevelFilter::Trace);
    logger::init();

    // 解析命令行参数，支持
    info!("starting up");

    // unsafe { eink::eink_enable() };

    info!("install driver");
    install_driver()?;

    info!("");
    let manager = DisplayManager::Create(DisplayManagerOptions::None)?;
    for (i, t) in manager.GetCurrentTargets()?.into_iter().enumerate() {
        let monitor_id = t.StableMonitorId()?;
        if monitor_id.len() > 0 {
            info!("Display[{}] {}", i, monitor_id);
        }
    }
    info!("");

    info!("{:?}", std::env::args_os());

    // let mut args = Arguments::from_env();
    // info!("args: {:?}", args);

    // let monitor_id: String = args.value_from_str("monitor_id")?;
    // let test_background: Option<bool> = args.opt_value_from_str("test_background")?;
    // let test_layer: Option<bool> = args.opt_value_from_str("test_layer")?;

    let args = Args::parse();

    info!("args.monitor_id: {:?}", args.monitor_id);
    info!("args.test_background: {:?}", args.test_background);
    info!("args.test_layer: {:?}", args.test_layer);

    // std::thread::sleep(std::time::Duration::from_secs(2));

    unsafe {
        let driver = CreateFileW(
            &HSTRING::from("\\\\.\\DispFilter"),
            FILE_ACCESS_FLAGS(GENERIC_READ | GENERIC_WRITE),
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_NO_BUFFERING,
            HANDLE::default(),
        );

        if let Ok(driver) = driver {
            let DISP_FILTER_DEVICE: u32 = 0x8000;
            let IOCTL_DISP_FILTER_ENABLE = CTL_CODE(
                DISP_FILTER_DEVICE,
                0x800,
                METHOD_NEITHER,
                FILE_WRITE_ACCESS | FILE_READ_ACCESS,
            );

            let mut bytes_returned: u32 = 0;

            DeviceIoControl(
                driver,
                IOCTL_DISP_FILTER_ENABLE,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0,
                &bytes_returned as *const u32 as *mut u32,
                std::ptr::null_mut(),
            );

            CloseHandle(driver);
        } else {
            info!("DispFilter failed !");
        }
    }

    let mut sf = SurfaceFlinger::new(
        args.test_background.unwrap_or_default(),
        args.test_layer.unwrap_or_default(),
        &args.monitor_id,
    )?;

    sf.run();

    Ok(())
}

fn install_driver() -> Result<()> {
    unsafe {
        let sch = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)?;
        let service_name = HSTRING::from("DispFilter");
        let sh = OpenServiceW(sch, &service_name, SC_MANAGER_ALL_ACCESS);

        // Err(Error { code: 0x80070424, message: 指定的服务未安装。 })
        match sh {
            Ok(sh) => {
                info!("DispFilter Service: {:?}", sh);

                let ret = StartServiceA(sh, &[PSTR::null()]);
                info!("DispFilter Start Service: {:?}", ret);

                CloseServiceHandle(sh);
            }
            Err(err) => {
                info!("err.code() : {:?}", err.code());
                info!(
                    "ERROR_SERVICE_DOES_NOT_EXIST : {:?}",
                    ERROR_SERVICE_DOES_NOT_EXIST
                );
                #[allow(overflowing_literals)]
                if err.code() == HRESULT(0x80070424) {
                    let mut bin_path = std::env::current_exe()?;
                    bin_path.set_file_name("DispFilter.sys");
                    info!("Driver BinPath: {:?}", bin_path);

                    let bin_path = bin_path.to_str().unwrap().to_string();

                    match CreateServiceW(
                        sch,
                        &service_name,
                        &service_name,
                        SC_MANAGER_ALL_ACCESS,
                        SERVICE_KERNEL_DRIVER,
                        SERVICE_DEMAND_START,
                        SERVICE_ERROR_IGNORE,
                        &HSTRING::from(bin_path),
                        PCWSTR::null(),
                        std::ptr::null() as *const u32 as *mut u32,
                        PCWSTR::null(),
                        PCWSTR::null(),
                        PCWSTR::null(),
                    ) {
                        Ok(_) => {
                            info!("Create DispFilter Service Success");

                            let sh = OpenServiceW(sch, &service_name, SC_MANAGER_ALL_ACCESS)?;
                            let ret = StartServiceA(sh, &[PSTR::null()]);
                            info!("DispFilter Start Service: {:?}", ret);
                            CloseServiceHandle(sh);
                        }
                        Err(err) => {
                            info!("Create DispFilter Service Failed: {:?}", err);
                        }
                    }
                }
            }
        }

        CloseServiceHandle(sch);
    }
    Ok(())
}
