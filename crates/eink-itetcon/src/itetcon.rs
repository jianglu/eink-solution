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

use std::ffi::CStr;

use windows::Win32::Foundation::HANDLE;

#[allow(non_snake_case)]

/// 和 EInkTcon.dll 的最底层对接

#[repr(C)]
pub struct TRSP_SYSTEM_INFO_DATA {
    uiStandardCmdNo: u32,   // Standard command number2T-con Communication Protocol
    uiExtendCmdNo: u32,     // Extend command number
    uiSignature: u32,       // 31 35 39 38h (8951)
    uiVersion: u32,         // command table version
    uiWidth: u32,           // Panel Width
    uiHeight: u32,          // Panel Height
    uiUpdateBufBase: u32,   // Update Buffer Address
    uiImageBufBase: u32,    // Image Buffer Address
    uiTemperatureNo: u32,   // Temperature segment number
    uiModeNo: u32,          // Display mode number
    uiFrameCount: [u32; 8], // Frame count for each mode(8).
    uiNumImgBuf: u32,
    uiWbfSFIAddr: u32,
    uiwaveforminfo: u32,    //low byte:A2 mode index
    uiMultiPanelIndex: u32, //High two byte for Y-axis, low two byte for X-axis
    uiTpXMax: u32,          // Tp resolution
    uiTpYMax: u32,
    TPVersion: [u8; 4], //e.g. v.1.0.9  TPVersion[] = {0x00,0x01,0x00,x09}
    ucEPDType: u8,      //0-old (needs 180 rotation), 1 - New(no need to 180 rotation)
    ucReserved: [u8; 3],
    uiReserved: [u32; 2],
    //	void* lpCmdInfoDatas[1]; // Command table pointer
}

// mipi mode
pub const GI_MIPI_READER: u32 = 0x0u32;
pub const GI_MIPI_MIXED: u32 = 0x01u32;
pub const GI_MIPI_BROWSER: u32 = 0x02u32;
pub const GI_MIPI_FAST_READER: u32 = 0x03u32;
pub const GI_MIPI_FAST_UI: u32 = 0x04u32;
pub const GI_MIPI_SLEEP: u32 = 0x0Fu32;
pub const GI_MIPI_NO: u32 = 0x10u32;
pub const GI_MIPI_REFRESH: u32 = 0x11u32;
pub const GI_MIPI_STANDBY: u32 = 0x12u32;
pub const GI_MIPI_DIRECT_HANDWRITING: u32 = 0x13u32;
pub const GI_MIPI_HYBRID: u32 = 0xF0u32;

#[windows_dll::dll(EInkTcon)]
extern "system" {
    pub fn ITEGetSystemInfoAPI(system_info: *mut TRSP_SYSTEM_INFO_DATA) -> u32;
    pub fn ITEGetDriveNo(drive_no: &mut u8) -> u32;
    pub fn ITEOpenDeviceAPI(dev_path: &CStr) -> HANDLE;
    pub fn ITECloseDeviceAPI() -> ();
    pub fn ITESet8951KeepAlive(enable: u32) -> u32;
    pub fn ITESetMIPIModeAPI(mode: &mut u32) -> u32;
    pub fn ITEGetBufferAddrInfoAPI(addrs: &mut [u32; 3]) -> u32;
    pub fn ITELoadImage(img_buf: *mut u8, img_buf_addr: u32, x: u32, y: u32, w: u32, h: u32)
        -> u32;
    pub fn ITEDisplayAreaAPI(
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        mode: u32,
        mem_addr: u32,
        wait_ready: u32,
    ) -> u32;
}

#[test]
fn test_get_buffer_addr_info() {
    let mut addrs: u32 = 0;
    unsafe { ITEGetBufferAddrInfoAPI(&mut addrs) };
    println!("ITEGetBufferAddrInfoAPI: addrs: {addrs}");
}
