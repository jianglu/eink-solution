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
use std::process::Command;

use anyhow::{bail, Result};
use ntapi::winapi::shared::ntdef::{PWCH, UNICODE_STRING};
use widestring::U16String;
use windows::core::GUID;
use windows::Devices::Display::Core::{DisplayManager, DisplayManagerOptions, DisplayTarget};
use windows::Win32::Devices::Display::{
    DisplayConfigSetDeviceInfo, DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_TYPE,
};
use windows::Win32::Foundation::{ERROR_GEN_FAILURE, ERROR_SUCCESS, LUID, WIN32_ERROR};

use crate::utils::get_current_exe_dir;
use crate::win_utils;

fn hash_unicode_string(slice: &str) -> u32 {
    let mut hash_value = 0u32;
    unsafe {
        let slice_string = U16String::from_str(slice);
        let slice_bytes_num = (slice_string.len() * 2) as u16;
        let mut unicode_string = UNICODE_STRING {
            Length: slice_bytes_num,
            MaximumLength: slice_bytes_num,
            Buffer: slice_string.as_ptr() as PWCH,
        };
        use ntapi::ntrtl::RtlHashUnicodeString;
        RtlHashUnicodeString(&mut unicode_string, 1, 0, &mut hash_value);
    }
    return hash_value;
}

pub fn set_monitor_specialized(monitor_id: &str, specialized: bool) -> Result<()> {
    let display_manager = DisplayManager::Create(DisplayManagerOptions::None)?;
    let display_targets = display_manager.GetCurrentTargets()?;
    for display_target in display_targets {
        if let Ok(stable_monitor_id) = display_target.StableMonitorId() {
            if stable_monitor_id == monitor_id {
                set_display_target_specialized(&display_target, specialized)?;
                return Ok(());
            }
        }
    }
    Ok(())
}

// pub fn set_monitor_specialized(monitor_id: &str, on: bool) -> Result<()> {
//     // keyboard manager 可执行程序和 eink-service 在同一目录
//     let exe_dir = get_current_exe_dir();
//     let specialize_exe = exe_dir.join("eink-specialize-display.exe");
//     let on = if on { "on" } else { "off" };

//     // let process = Command::new(specialize_exe)
//     //     .args([
//     //         "--monitor-id",
//     //         monitor_id,
//     //         "--removal",
//     //         if on { "on" } else { "off" },
//     //     ])
//     //     .status()
//     //     .expect("Cannot spawn eink-specialize-display");

//     let cmd = format!("eink-specialize-display.exe --monitor-id \"{monitor_id}\" --removal {on}");

//     log::info!("{cmd}");

//     // cmd_lib_cf::run_cmd!($cmd);
//     win_utils::run_as_admin(exe_dir.to_str().unwrap(), &cmd);

//     Ok(())
// }

// pub fn set_display_target_specialized(target: &DisplayTarget, specialized: bool) -> Result<i32> {
//     println!("set_monitor_specialized: {}", specialized);

//     let stable_monitor_id = target.StableMonitorId().unwrap();

//     let guid_monitor_override_pseudo_specialized = "f196c02f-f86f-4f9a-aa15-e9cebdfe3b96";

//     let destination = format!(
//         "{}{}",
//         stable_monitor_id, guid_monitor_override_pseudo_specialized
//     );

//     let pos = destination.len() / 2;

//     let destination_slice_1 = &destination[0..pos];
//     let destination_slice_2 = &destination[pos..];

//     let hash_value_l = hash_unicode_string(destination_slice_1);
//     let hash_value_h = hash_unicode_string(destination_slice_2);
//     let hash_value = ((hash_value_h as u64) << 32) | (hash_value_l as u64);

//     let display_adapter = target.Adapter()?;
//     let adapter_id = display_adapter.Id()?;
//     let adapter_relative_id = target.AdapterRelativeId()?;

//     #[allow(non_camel_case_types)]
//     struct DISPLAYCONFIG_MONITOR_SPECIALIZATION {
//         mode: DISPLAYCONFIG_DEVICE_INFO_TYPE,
//         size: u32,
//         adapter_id: LUID,
//         id: u32,
//         guid: GUID,
//         specialized: u32,
//         hash: u64,
//     }

//     let mut config: DISPLAYCONFIG_MONITOR_SPECIALIZATION = unsafe { zeroed() };
//     config.mode = DISPLAYCONFIG_DEVICE_INFO_TYPE(0xFFFFFFE9 as u32 as i32);
//     config.size = size_of::<DISPLAYCONFIG_MONITOR_SPECIALIZATION>() as u32;
//     config.adapter_id.HighPart = adapter_id.HighPart;
//     config.adapter_id.LowPart = adapter_id.LowPart;
//     config.id = adapter_relative_id;
//     config.guid = GUID::from(guid_monitor_override_pseudo_specialized);
//     config.specialized = if specialized { 1 } else { 0 };
//     config.hash = hash_value;

//     let result = unsafe {
//         DisplayConfigSetDeviceInfo(
//             &config as *const DISPLAYCONFIG_MONITOR_SPECIALIZATION
//                 as *const DISPLAYCONFIG_DEVICE_INFO_HEADER,
//         )
//     };

//     if WIN32_ERROR(result as u32) == ERROR_SUCCESS {
//         Ok(result)
//     } else {
//         bail!("DisplayConfigSetDeviceInfo failed: WIN32_ERROR({result})")
//     }

//     // // ERROR_SUCCESS
//     // //      The function succeeded.
//     // // ERROR_INVALID_PARAMETER
//     // //      The combination of parameters and flags specified are invalid.
//     // // ERROR_NOT_SUPPORTED
//     // //      The system is not running a graphics driver that was written according to the Windows Display Driver Model (WDDM). The function is only supported on a system with a WDDM driver running.
//     // // ERROR_ACCESS_DENIED
//     // //      The caller does not have access to the console session. This error occurs if the calling process does not have access to the current desktop or is running on a remote session.
//     // // ERROR_INSUFFICIENT_BUFFER
//     // //      The size of the packet that the caller passes is not big enough.
//     // // ERROR_GEN_FAILURE
//     // //      An unspecified error occurred.
//     // // ERROR_GEN_FAILURE
//     // println!("DisplayConfigSetDeviceInfo: {}", result);
// }

fn set_display_target_specialized(target: &DisplayTarget, removal: bool) -> Result<()> {
    let stable_monitor_id = target.StableMonitorId()?;

    let guid_monitor_override_pseudo_specialized = "{f196c02f-f86f-4f9a-aa15-e9cebdfe3b96}";

    let destination = format!(
        "{}{}",
        stable_monitor_id, guid_monitor_override_pseudo_specialized
    );

    let pos = destination.len() / 2;

    let destination_slice_1 = &destination[0..pos];
    let destination_slice_2 = &destination[pos..];

    let hash_value_l = hash_unicode_string(destination_slice_1);
    let hash_value_h = hash_unicode_string(destination_slice_2);
    let hash_value = ((hash_value_h as u64) << 32) | (hash_value_l as u64);

    let display_adapter = target.Adapter().unwrap();
    let adapter_id = display_adapter.Id().unwrap();
    let adapter_relative_id = target.AdapterRelativeId().unwrap();

    Command::new("C:\\Windows\\System32\\SystemSettingsAdminFlows.exe")
        .arg("SpecializeDisplay")
        .arg(format!("{}", adapter_id.LowPart))
        .arg(format!("{}", adapter_id.HighPart))
        .arg(format!("{}", adapter_relative_id))
        .arg(if removal { "1" } else { "0" })
        .arg(format!("{}", hash_value))
        .output()
        .expect("failed to execute process");

    Ok(())
}
