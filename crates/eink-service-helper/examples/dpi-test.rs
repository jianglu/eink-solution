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

use std::mem::{size_of, zeroed};

use anyhow::bail;
use windows::core::{Borrowed, InParam, GUID};
use windows::Devices::Display::Core::{DisplayManager, DisplayManagerOptions, DisplayTarget};
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, DisplayConfigSetDeviceInfo, GetDisplayConfigBufferSizes,
    QueryDisplayConfig, DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_TYPE,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_TOPOLOGY_ID,
};
use windows::Win32::Foundation::{ERROR_SUCCESS, LUID, NO_ERROR, WIN32_ERROR};
use windows::Win32::Graphics::Gdi::QDC_ONLY_ACTIVE_PATHS;

/*
 * struct DPIScalingInfo
 * @brief DPI info about a source
 * mininum :     minumum DPI scaling in terms of percentage supported by source. Will always be 100%.
 * maximum :     maximum DPI scaling in terms of percentage supported by source. eg. 100%, 150%, etc.
 * current :     currently applied DPI scaling value
 * recommended : DPI scaling value reommended by OS. OS takes resolution, physical size, and expected viewing distance
 *               into account while calculating this, however exact formula is not known, hence must be retrieved from OS
 *               For a system in which user has not explicitly changed DPI, current should eqaul recommended.
 * bInitDone :   If true, it means that the members of the struct contain values, as fetched from OS, and not the default
 *               ones given while object creation.
 */
pub struct DPIScalingInfo {
    mininum: u32,     // = 100;
    maximum: u32,     // = 100;
    current: u32,     // = 100;
    recommended: u32, // = 100;
    init_done: bool,  // = false;
}

#[repr(C)]
pub struct DISPLAYCONFIG_SOURCE_DPI_SCALE_GET {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    min_scale_rel: i32,
    cur_scale_rel: i32,
    max_scale_rel: i32,
}

#[repr(C)]
pub struct DISPLAYCONFIG_SOURCE_DPI_SCALE_SET {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    scale_rel: i32,
}

// returns min, max, suggested, and currently applied DPI scaling values.
const DISPLAYCONFIG_DEVICE_INFO_GET_DPI_SCALE: DISPLAYCONFIG_DEVICE_INFO_TYPE =
    DISPLAYCONFIG_DEVICE_INFO_TYPE(-3i32);

// set current dpi scaling value for a display
const DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE: DISPLAYCONFIG_DEVICE_INFO_TYPE =
    DISPLAYCONFIG_DEVICE_INFO_TYPE(-4i32);

pub fn get_dpi_scaling_info(adapter_id: LUID, source_id: u32) {
    let mut config: DISPLAYCONFIG_SOURCE_DPI_SCALE_GET = unsafe { zeroed() };
    config.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_DPI_SCALE;
    config.header.size = size_of::<DISPLAYCONFIG_SOURCE_DPI_SCALE_GET>() as u32;
    // if this fails => OS has changed somthing, and our reverse enginnering knowledge about the API is outdated
    assert!(0x20 == size_of::<DISPLAYCONFIG_SOURCE_DPI_SCALE_GET>());
    config.header.adapterId = adapter_id;
    config.header.id = source_id;
}

/// 根据 monitor_id 设置 DPI
/// 100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500u32,
pub fn set_dpi_by_stable_monitor_id(
    monitor_id: &str,
    mut scale_to_set: u32,
) -> anyhow::Result<WIN32_ERROR> {
    // WinRT 显示管理器
    let display_manager = DisplayManager::Create(DisplayManagerOptions::None)?;
    let targets = display_manager.GetCurrentTargets()?;

    // 查找对应的 DisplayTarget 显示目标
    let mut display_target: Option<DisplayTarget> = None;

    for target in targets {
        if let Ok(id) = target.StableMonitorId() {
            if id != monitor_id {
                display_target = Some(target);
                break;
            }
        }
    }

    if display_target.is_none() {
        bail!("Cannot found DisplayTarget for monitor id: {monitor_id}")
    }

    // 读取显示路径
    let mut num_paths: u32 = 0;
    let mut paths: [DISPLAYCONFIG_PATH_INFO; 32] = unsafe { zeroed() };

    let mut num_modes: u32 = 0;
    let mut modes: [DISPLAYCONFIG_MODE_INFO; 32] = unsafe { zeroed() };

    let ret = unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut num_paths, &mut num_modes)
    };
    if WIN32_ERROR(ret as u32) != ERROR_SUCCESS {
        bail!("GetDisplayConfigBufferSizes Failed: WIN32_ERROR({})", ret)
    }

    let ret = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    };
    if WIN32_ERROR(ret as u32) != ERROR_SUCCESS {
        bail!("QueryDisplayConfig Failed: WIN32_ERROR({})", ret)
    }

    let target = display_target.unwrap();

    let display_adapter = target.Adapter()?;
    let adapter_id = display_adapter.Id()?;
    let adapter_relative_id = target.AdapterRelativeId()?;

    let mut source_id: Option<u32> = None;

    // 遍历 Target-Source 路径
    for i in 0..num_paths as usize {
        let path = paths[i];

        if path.targetInfo.adapterId.HighPart == adapter_id.HighPart
            && path.targetInfo.adapterId.LowPart == adapter_id.LowPart
        // && path.targetInfo.id == adapter_relative_id
        {
            source_id = Some(path.sourceInfo.id);
            break;
        }
    }

    if source_id.is_none() {
        bail!("Cannot found DisplayTarget source id for monitor id: {monitor_id}")
    }

    let source_id = source_id.unwrap();

    // Get DPI
    let mut get_config: DISPLAYCONFIG_SOURCE_DPI_SCALE_GET = unsafe { zeroed() };
    get_config.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_DPI_SCALE;
    get_config.header.size = size_of::<DISPLAYCONFIG_SOURCE_DPI_SCALE_GET>() as u32;
    assert!(0x20 == size_of::<DISPLAYCONFIG_SOURCE_DPI_SCALE_GET>());
    get_config.header.adapterId = LUID {
        LowPart: adapter_id.LowPart,
        HighPart: adapter_id.HighPart,
    };
    get_config.header.id = source_id;

    let ret = unsafe {
        DisplayConfigGetDeviceInfo(
            &get_config as *const DISPLAYCONFIG_SOURCE_DPI_SCALE_GET
                as *mut DISPLAYCONFIG_DEVICE_INFO_HEADER,
        )
    };
    if WIN32_ERROR(ret as u32) != ERROR_SUCCESS {
        bail!("DisplayConfigGetDeviceInfo Failed: WIN32_ERROR({})", ret)
    }

    let dpi_vals = [
        100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500u32,
    ];

    // 边界条件正规化
    if get_config.cur_scale_rel < get_config.min_scale_rel {
        get_config.cur_scale_rel = get_config.min_scale_rel;
    } else if get_config.cur_scale_rel > get_config.max_scale_rel {
        get_config.cur_scale_rel = get_config.max_scale_rel;
    }

    let min_abs = i32::abs(get_config.min_scale_rel);

    let mut mininum = 100;
    let mut maximum = 100;
    let mut current = 100;
    let mut recommended = 100;

    if dpi_vals.len() >= (min_abs + get_config.max_scale_rel + 1) as usize {
        // all ok
        current = dpi_vals[(min_abs + get_config.cur_scale_rel) as usize];
        recommended = dpi_vals[min_abs as usize];
        maximum = dpi_vals[(min_abs + get_config.max_scale_rel) as usize];
    }

    println!(
        "mininum: {mininum}, maximum: {maximum}, current: {current}, recommended: {recommended}"
    );

    if scale_to_set == current {
        return Ok(NO_ERROR);
    }

    if scale_to_set < mininum {
        scale_to_set = mininum;
    } else if scale_to_set > maximum {
        scale_to_set = maximum;
    }

    let mut idx1 = -1;
    let mut idx2 = -1;

    let mut i = 0;

    for val in dpi_vals {
        if val == scale_to_set {
            idx1 = i;
        }

        if val == recommended {
            idx2 = i;
        }
        i = i + 1;
    }

    if idx1 == -1 || idx2 == -1 {
        bail!("Cannot find dpi value")
    }

    let dpi_rel_val = idx1 - idx2;

    // Set DPI
    let mut config: DISPLAYCONFIG_SOURCE_DPI_SCALE_SET = unsafe { zeroed() };
    config.header.r#type = DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE;
    config.header.size = size_of::<DISPLAYCONFIG_SOURCE_DPI_SCALE_SET>() as u32;
    assert!(0x18 == size_of::<DISPLAYCONFIG_SOURCE_DPI_SCALE_SET>());
    config.header.adapterId = LUID {
        LowPart: adapter_id.LowPart,
        HighPart: adapter_id.HighPart,
    };
    config.header.id = source_id;
    config.scale_rel = dpi_rel_val;

    let ret = unsafe {
        DisplayConfigSetDeviceInfo(
            &config as *const DISPLAYCONFIG_SOURCE_DPI_SCALE_SET
                as *mut DISPLAYCONFIG_DEVICE_INFO_HEADER,
        )
    };

    Ok(WIN32_ERROR(ret as u32))
}

/// 应用程序主入口
/// 1. 初始化各种服务
/// 2. 等待 runner 程序发送的 CTRL-C 信号以终止程序
fn main() -> anyhow::Result<()> {
    let display_manager = DisplayManager::Create(DisplayManagerOptions::None)?;

    let targets = display_manager.GetCurrentTargets()?;

    for target in targets {
        let monitor_id = target.StableMonitorId()?;
        if monitor_id.is_empty() {
            continue;
        }
        println!("monitor_id: {monitor_id}");
    }

    set_dpi_by_stable_monitor_id("SDC41820_00_07E5_74", 200)?;

    Ok(())
}
