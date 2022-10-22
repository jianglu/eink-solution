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

pub const EIMC_GRAY16: u32 = 1; // 16色灰度，0x00,0x10 ... 0xF0
pub const EIMC_BLACKWHITE: u32 = 2; // 黑白两色，0x00,0xF0
pub const EIMC_ARGB: u32 = 4; // ARGB图像

pub const EIMC_FLAG_NONE: u32 = 0;
pub const EIMC_DITHER_RIGHTDOWN: u32 = 1; // 向右下抖动
pub const EIMC_ENHANCING_5R1: u32 = 1; // 采用中心像素5，上下左右像素-1的核，做增强

pub const EIMC_IMG_FILL: u32 = 0;
pub const EIMC_IMG_CENTER: u32 = 1;
pub const EIMC_IMG_STRETCH: u32 = 2;
pub const EIMC_IMG_TILE: u32 = 3;

#[windows_dll::dll(ImgCodec)]
extern "system" {

    // 释放本Wic对象，暂时解决一个wic失效的未知bug，ax Dec.19,2017
    pub fn EiReleaseWic() -> ();

    // 读取文件，并且转换为指定的格式
    // 返回的指针，不用时，调用EicReleaseImage释放
    pub fn EicLoadImage(
        npPathName: *const u16, // 文件名
        nuFormat: u32,          // EIMC_GRAY16 or EIMC_BLACKWHITE
        nuLayout: u32,
        nuWidth: u32,  // 转换后宽度，也可以是EIMC_AUTO，或者是（width|EIMC_FIX_RATIO)
        nuHeight: u32, // 转换后高度，也可以是EIMC_AUTO，或者是（width|EIMC_FIX_RATIO)
        npWidthR: &mut u32, // 返回转换后宽度
        xnpHeightR: &mut u32, // 返回转换后高度
    ) -> *mut u8;

    //	when nuFormat == EIMC_GRAY16, nuFlag can be set to EIMC_FLAG_NONE or EIMC_ENHANCING_5R1
    //		 nuFormat == EIMC_BLACKWHITE, nuFlag can be set to  EIMC_FLAG_NONE or EIMC_DITHER_RIGHTDOWN

    // 将ARGB图像转化为Gray H16图像
    // 返回转换后的数据，不使用时，调用EicReleaseImage释放
    pub fn EicConvertToGray16(
        npArgb: *mut u8,
        nuLayout: u32,
        nuWidth: u32,
        nuHeight: u32,
        nuImageWidth: u32,
        nuImageHeight: u32,
        nbEnhancing: bool,
    ) -> *mut u8;

    // 将ARGB图像转化为Black&White图像
    // 返回转换后的数据，不使用时，调用EicReleaseImage释放
    pub fn EicConvertToBlackWhite(
        npArgb: *mut u8,
        nuWidth: u32,
        nuHeight: u32,
        nbDither: bool,
    ) -> *mut u8;

    // 将普通的灰度值转换为T1000支持的灰度值
    pub fn EicConvertToT1000Format(mpBufImage: *mut u8, nuWidth: u32, nuHeight: u32);

    // 抖动Black&White图像
    // 返回转换后的数据，不使用时，调用EicReleaseImage释放
    pub fn EicDither(npGrayH16: *mut u8, nuWidth: u32, nuHeight: u32) -> *mut u8;

    // 将H16数据旋转180度
    pub fn EiTurn180(npGrayH16: *mut u8, nuWidth: u32, nuHeight: u32);

    // 将H16数据顺时针旋转90、180、270度，旋转90和270度将导致图像的宽度和高度产生交换而改变
    pub fn EiTurn(
        npGrayH16: *mut u8,
        npNewH16: *mut u8,
        nuWidth: u32,
        nuHeight: u32,
        nuAngle: u32, // 90,180,270
    );

    pub fn EicSaveToImageFile(
        npPathName: *const u16, // 文件名
        npArgb: *mut u8,        // 图像缓冲区
        nuWidth: u32,           // 宽度
        nuHeight: u32,          // 高度
    ) -> bool;

    // 将输入图像缩放到指定大小后保存为新文件
    pub fn EicResizeImage(
        npFileInput: *const u16,  // 输入文件名
        npFileOutput: *const u16, // 输出文件名
        nuWidth: u32,
        nuHeight: u32,
        nuColor: u32, // 用于填充空白的像素值
    ) -> bool;

    pub fn EicReleaseImage(npImage: *mut u8);
}
