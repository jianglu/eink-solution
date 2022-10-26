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

pub use windows::core::HSTRING;

pub use windows::core::{GUID, HRESULT, PCWSTR};
pub use windows::w;
pub use windows::Win32::Devices::DeviceAndDriverInstallation::SetupDiEnumDeviceInterfaces;
pub use windows::Win32::Devices::DeviceAndDriverInstallation::SetupDiGetClassDevsW;
pub use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiGetDeviceInterfaceDetailW, HDEVINFO, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA,
};
pub use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_DEVICEINTERFACE, DIGCF_INTERFACEDEVICE, DIGCF_PRESENT,
};
pub use windows::Win32::Devices::Enumeration::Pnp::{
    SWDeviceCapabilitiesDriverRequired, SWDeviceCapabilitiesRemovable, SwDeviceCreate, HSWDEVICE,
    SW_DEVICE_CAPABILITIES, SW_DEVICE_CREATE_INFO,
};
pub use windows::Win32::Foundation::HWND;
pub use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
pub use windows::Win32::Foundation::{CloseHandle, HANDLE};
pub use windows::Win32::System::Com::CLSIDFromString;
pub use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
pub use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject};

pub use windows::Win32::Foundation::WAIT_OBJECT_0;
pub use windows::Win32::Foundation::WIN32_ERROR;
pub use windows::Win32::Storage::FileSystem::CreateFileW;
pub use windows::Win32::Storage::FileSystem::FILE_ACCESS_FLAGS;
pub use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
pub use windows::Win32::Storage::FileSystem::FILE_SHARE_MODE;
pub use windows::Win32::Storage::FileSystem::OPEN_EXISTING;
pub use windows::Win32::System::IO::DeviceIoControl;

pub const GENERIC_READ: u32 = 0x80000000;
pub const GENERIC_WRITE: u32 = 0x40000000;
pub const FILE_SHARE_READ: u32 = 0x00000001;
pub const FILE_SHARE_WRITE: u32 = 0x00000002;

pub use windows::Devices::Display::Core::DisplayManager;
pub use windows::Devices::Display::Core::DisplayManagerOptions;
