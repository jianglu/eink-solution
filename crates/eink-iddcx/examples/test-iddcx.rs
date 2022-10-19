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

use eink_iddcx::{recreate_iddcx_device, get_iddcx_device_path};



fn main() -> anyhow::Result<()> {



    log::info!("recreate_iddcx_device");
    recreate_iddcx_device();

    log::info!("get_iddcx_device_path");
    let dev_path = get_iddcx_device_path()?;
    log::info!("dev_path: {}", &dev_path);

    log::info!("remove_monitor 0, 1");
    eink_iddcx::remove_monitor(&dev_path, 0)?;
    eink_iddcx::remove_monitor(&dev_path, 1)?;

    log::info!("add_monitor 1024x768");
    let monitor_id_0 = eink_iddcx::add_monitor(&dev_path, 1024, 768)?;
    log::info!("monitor_id: {}", monitor_id_0);

    log::info!("add_monitor 1920x1080");
    let monitor_id_1 = eink_iddcx::add_monitor(&dev_path, 1920, 1080)?;
    log::info!("monitor_id: {}", monitor_id_1);

    let mut line_buf = String::new();
    std::io::stdin().read_line(&mut line_buf)?;

    log::info!("remove_monitor: {}", monitor_id_0);
    eink_iddcx::remove_monitor(&dev_path, monitor_id_0)?;

    log::info!("remove_monitor: {}", monitor_id_1);
    eink_iddcx::remove_monitor(&dev_path, monitor_id_1)?;

    Ok(())
}