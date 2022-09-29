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

use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
};

use anyhow::{bail, Result};
use log::{debug, info};

use crate::winrt;

unsafe extern "system" fn creation_callback(
    _device: winrt::HSWDEVICE,
    _create_result: winrt::HRESULT,
    context: *const c_void,
    _device_instance_id: winrt::PCWSTR,
) {
    info!("Device Creation Callback Start");
    let event = *(context as *const winrt::HANDLE);
    winrt::SetEvent(event);
    info!("Device Creation Callback End");
}

pub fn recreate_iddcx_device() -> Result<()> {
    info!("recreate_iddcx_device");

    let event = unsafe { winrt::CreateEventW(None, false, false, winrt::PCWSTR::null())? };

    let mut create_info: winrt::SW_DEVICE_CREATE_INFO = unsafe { zeroed() };

    create_info.cbSize = size_of::<winrt::SW_DEVICE_CREATE_INFO>() as u32;
    create_info.pszzCompatibleIds = winrt::w!("fusioniddcx\0\0").into();
    create_info.pszInstanceId = winrt::w!("fusioniddcx").into();
    create_info.pszzHardwareIds = winrt::w!("fusioniddcx\0\0").into();
    create_info.pszDeviceDescription = winrt::w!("fusioniddcx\0\0").into();

    create_info.CapabilityFlags = (winrt::SWDeviceCapabilitiesRemovable.0
        + winrt::SWDeviceCapabilitiesDriverRequired.0) as u32;

    info!("recreate_iddcx_device 1");

    // Create the device
    let device = unsafe {
        winrt::HSWDEVICE(winrt::SwDeviceCreate(
            winrt::w!("fusioniddcx"),
            winrt::w!("HTREE\\ROOT\\0"),
            &create_info,
            None,
            Some(creation_callback),
            Some(&event as *const winrt::HANDLE as *const c_void),
        )?)
    };

    info!("SwDeviceCreate: device: {:?}", device);

    // HANDLE hEvent = CreateEvent(nullptr, FALSE, FALSE, nullptr);
    // HSWDEVICE hSwDevice;
    // SW_DEVICE_CREATE_INFO createInfo = { 0 };
    // PCWSTR description = L"fusioniddcx";

    // // These match the Pnp id's in the inf file so OS will load the driver when the device is
    // // created
    // PCWSTR instanceId = L"fusioniddcx";
    // PCWSTR hardwareIds = L"fusioniddcx\0\0";
    // PCWSTR compatibleIds = L"fusioniddcx\0\0";

    // // Create the device
    // HRESULT hr = SwDeviceCreate(L"fusioniddcx",
    //                             L"HTREE\\ROOT\\0",
    //                             &createInfo,
    //                             0,
    //                             nullptr,
    //                             creation_callback,
    //                             &hEvent,
    //                             &hSwDevice);
    // if (FAILED(hr))
    // {
    //     DBG(L"SwDeviceCreate failed with 0x%lx\n", hr);
    //     return ERROR_CREATE_FAILED;
    // }

    // Wait for callback to signal that the device has been created
    info!("Waiting for device to be created ...");
    let wait_result = unsafe { winrt::WaitForSingleObject(event, 10 * 1000) };

    if wait_result != winrt::WAIT_OBJECT_0 {
        bail!("Wait for device creation failed");
    }

    info!("Device was created");

    Ok(())
}

fn setupdi_get_device_interface_detail2(
    dev_info: winrt::HDEVINFO,
    dev_iface_data: &mut winrt::SP_DEVICE_INTERFACE_DATA,
    dev_info_data: &mut winrt::SP_DEVINFO_DATA,
) -> Result<String> {
    let mut required_size: u32 = 0;

    // First call to get the size to allocate
    let result = unsafe {
        winrt::SetupDiGetDeviceInterfaceDetailW(
            dev_info,
            dev_iface_data,
            None,
            0,
            Some(&mut required_size),
            Some(dev_info_data),
        )
    };

    info!("SetupDiGetDeviceInterfaceDetailW: {:?}", result);
    info!("required_size: {}", required_size);

    info!("dev_info_data.cbSize: {:?}", dev_info_data.cbSize);
    info!("dev_info_data.ClassGuid: {:?}", dev_info_data.ClassGuid);
    info!("dev_info_data.DevInst: {:?}", dev_info_data.DevInst);

    // As we passed an empty buffer we know that the function will fail, not need to check the
    // result.

    use windows::Win32::Foundation::GetLastError;
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;

    let last_error = unsafe { GetLastError() };
    if last_error != ERROR_INSUFFICIENT_BUFFER {
        bail!("ERROR_INSUFFICIENT_BUFFER");
    }

    let mut detail_buf = Vec::<u8>::with_capacity(required_size as usize);
    detail_buf.resize(required_size as usize, 0);

    let detail = detail_buf.as_mut_ptr() as *mut winrt::SP_DEVICE_INTERFACE_DETAIL_DATA_W;
    unsafe { (*detail).cbSize = size_of::<winrt::SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32 };
    info!("(*detail).cbSize: {:?}", unsafe { (*detail).cbSize });

    // Second call to get the value
    let success = unsafe {
        winrt::SetupDiGetDeviceInterfaceDetailW(
            dev_info,
            dev_iface_data,
            Some(detail),
            required_size,
            None,
            Some(dev_info_data),
        )
    };
    info!("SetupDiGetDeviceInterfaceDetailW: {:?}", success);
    info!("GetLastError: {:?}", unsafe { GetLastError() });

    let device_path = unsafe { &(*detail).DevicePath as *const u16 as *mut u16 };

    let device_path = unsafe { widestring::U16CStr::from_ptr_str(device_path) };
    let device_path = device_path.to_string()?;
    // let device_path = unsafe { String::from_utf16_lossy(&(*detail).DevicePath) };

    info!("device_path: {:?}", device_path);

    Ok(device_path)
}

/// 查找 iddcx 驱动设备访问路径
pub fn get_iddcx_device_path_internal() -> Result<String> {
    let class_guid_hstr = winrt::HSTRING::from("{ccf0a4d1-cbca-47cf-bf58-1baafc2ae082}");
    let class_guid: winrt::GUID = unsafe { winrt::CLSIDFromString(&class_guid_hstr)? };

    info!("class_guid_hstr: {:?}", &class_guid_hstr);
    info!("class_guid: {:?}", &class_guid);

    let dev_info = unsafe {
        winrt::SetupDiGetClassDevsW(
            Some(&class_guid),
            winrt::PCWSTR::null(),
            winrt::HWND::default(),
            winrt::DIGCF_PRESENT | winrt::DIGCF_DEVICEINTERFACE, // | win32::DIGCF_INTERFACEDEVICE,
        )?
    };

    info!("dev_info: {:?}", &dev_info);

    let member_index = 0;

    let mut dev_iface_data: winrt::SP_DEVICE_INTERFACE_DATA = unsafe { zeroed() };
    dev_iface_data.cbSize = std::mem::size_of::<winrt::SP_DEVICE_INTERFACE_DATA>() as u32;
    println!("dev_iface_data.cbSize: {:?}", dev_iface_data.cbSize);

    let result = unsafe {
        winrt::SetupDiEnumDeviceInterfaces(
            dev_info,
            None,
            &class_guid,
            member_index,
            &mut dev_iface_data,
        )
    };

    use windows::Win32::Foundation::BOOL;
    if result == BOOL(0) {
        bail!("GetLastError: {:?}", unsafe { GetLastError() });
    }

    info!("SetupDiEnumDeviceInterfaces: {:?}", result);

    use windows::Win32::Foundation::GetLastError;
    info!("GetLastError: {:?}", unsafe { GetLastError() });
    info!(
        "dev_iface_data.InterfaceClassGuid: {:?}",
        dev_iface_data.InterfaceClassGuid
    );
    info!("dev_iface_data.Flags: {:?}", dev_iface_data.Flags);
    info!("dev_iface_data.Reserved: {:?}", dev_iface_data.Reserved);

    let mut dev_info_data: winrt::SP_DEVINFO_DATA = unsafe { zeroed() };
    dev_info_data.cbSize = std::mem::size_of::<winrt::SP_DEVINFO_DATA>() as u32;
    info!("dev_info_data.cbSize: {:?}", dev_info_data.cbSize);

    let device_path =
        setupdi_get_device_interface_detail2(dev_info, &mut dev_iface_data, &mut dev_info_data);

    info!("device_path: {:?}", device_path);

    Ok(device_path?)
}

fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    ((device_type) << 16) | ((access) << 14) | ((function) << 2) | (method)
}

fn send_request_to_device(dev_path: &str, request: &str) -> Result<String> {
    let device_handle = unsafe {
        winrt::CreateFileW(
            &winrt::HSTRING::from(dev_path),
            winrt::FILE_ACCESS_FLAGS(winrt::GENERIC_READ | winrt::GENERIC_WRITE), // Administrative privilege is required
            winrt::FILE_SHARE_MODE(winrt::FILE_SHARE_READ | winrt::FILE_SHARE_WRITE),
            None,
            winrt::OPEN_EXISTING,
            winrt::FILE_ATTRIBUTE_NORMAL,
            None,
        )?
    };

    println!("Device Handle: {:?}", device_handle);

    const FILE_DEVICE_BUS_EXTENDER: u32 = 0x0000002a;
    const METHOD_BUFFERED: u32 = 0;
    const FILE_ANY_ACCESS: u32 = 0;
    const FILE_READ_ACCESS: u32 = 0x0001;
    const FILE_WRITE_ACCESS: u32 = 0x0002;

    let ioctl_command = ctl_code(
        FILE_DEVICE_BUS_EXTENDER,
        0,
        METHOD_BUFFERED,
        FILE_ANY_ACCESS | FILE_READ_ACCESS | FILE_WRITE_ACCESS,
    );

    let response_buffer_size = 1024;
    let mut response_buffer: Vec<u16> = Vec::with_capacity(response_buffer_size);

    let mut response_size = 0;

    use widestring::U16String;

    let u16str = U16String::from(request);

    info!("u16str.len(): {:?}", u16str.len());
    info!("size_of::<u16>(): {:?}", size_of::<u16>());

    let result = unsafe {
        winrt::DeviceIoControl(
            device_handle,
            ioctl_command,
            Some(u16str.as_ptr() as *const c_void),
            (u16str.len() * size_of::<u16>()) as u32,
            Some(response_buffer.as_mut_ptr() as *mut c_void),
            response_buffer_size as u32,
            Some(&mut response_size),
            None,
        )
    };

    info!("DeviceIoControl: {:?}", result);
    info!("response_size: {:?}", response_size);

    let resp_str =
        unsafe { U16String::from_ptr(response_buffer.as_ptr(), response_size as usize / 2) };
    let resp = resp_str.to_string()?;
    info!("Response: {:?}", &resp);

    unsafe { winrt::CloseHandle(device_handle) };

    Ok(resp)
}

/// 查找 iddcx 驱动设备访问路径
pub fn get_iddcx_device_path() -> Result<String> {
    for _i in 0..10 {
        if let Ok(dev_path) = get_iddcx_device_path_internal() {
            return Ok(dev_path);
        }
    }
    bail!("Cannot find iddcx device path")
}

pub fn add_monitor(dev_path: &str, width: u32, height: u32) -> anyhow::Result<u32> {
    let request = serde_json::json!({
        "method": "add_monitor",
        "params": {
            "modes": [{
                "width": width,
                "height": height,
            }]
        }
    });

    let request_str = request.to_string();
    info!("Request: {}", &request_str);

    let resp = send_request_to_device(dev_path, &request_str)?;

    Ok(0)
}

pub fn remove_monitor(dev_path: &str, monitor_id: u32) -> anyhow::Result<()> {
    let request = serde_json::json!({
        "method": "remove_monitor",
        "monitor_id": monitor_id
    });

    let request_str = request.to_string();
    info!("Request: {}", &request_str);

    let resp = send_request_to_device(dev_path, &request_str)?;

    Ok(())
}
