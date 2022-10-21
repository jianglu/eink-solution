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

use std::mem::zeroed;

use anyhow::bail;
use eink_itetcon::{
    ITECloseDeviceAPI, ITEDisplayAreaAPI, ITEGetBufferAddrInfoAPI, ITEGetDriveNo,
    ITEGetSystemInfoAPI, ITELoadImage, ITEOpenDeviceAPI, ITESet8951KeepAlive, ITESetMIPIModeAPI,
    GI_MIPI_FAST_READER, TRSP_SYSTEM_INFO_DATA,
};
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;

fn main() -> anyhow::Result<()> {
    unsafe {
        // 获得设备驱动号
        let mut drive_no: u8 = 0;
        let ret = ITEGetDriveNo(&mut drive_no);
        println!("ITEGetDriveNo: ret: {}, drive_no: {}", ret, drive_no);

        // 打开设备
        let dev_path = format!("\\\\.\\{}:", (0x41 + drive_no) as char);
        println!("Dev Path: {}", dev_path);

        let cstr = std::ffi::CString::new(dev_path).unwrap();
        println!("Dev Path C: {:?}", &cstr);

        if ITEOpenDeviceAPI(&cstr) == INVALID_HANDLE_VALUE {
            bail!("Open eink device fail, in thread");
        }

        // 设置 KeepAlive 模式
        let ret = ITESet8951KeepAlive(1);
        println!("ITESet8951KeepAlive(1): {}", ret);

        // 设置 MIPI 模式
        let mut mode: u32 = 1;
        let ret = ITESetMIPIModeAPI(&mut mode);
        println!("ITESetMIPIModeAPI({}): {}", mode, ret);

        mode = 2;
        let ret = ITESetMIPIModeAPI(&mut mode);
        println!("ITESetMIPIModeAPI({}): {}", mode, ret);

        // 获得设备系统信息
        let mut sysinfo: TRSP_SYSTEM_INFO_DATA = zeroed();
        let res = ITEGetSystemInfoAPI(&mut sysinfo);
        println!("ITEGetSystemInfoAPI: res: {res}");

        // 获得图片地址（支持 3 张图片），支持 3 张图片轮询
        let mut addrs: [u32; 3] = zeroed();
        ITEGetBufferAddrInfoAPI(&mut addrs);
        println!("ITEGetBufferAddrInfoAPI: addrs: {addrs:?}");

        // 显示封面图

        // 计算当前可用图片地址
        let mut latest_image_idx: u32 = u32::max_value();

        let image_idx = if latest_image_idx == u32::max_value() {
            latest_image_idx = 0;
            0
        } else {
            (latest_image_idx + 1) % 2
        };

        let img_addr = addrs[image_idx as usize];
        println!("img_addr: {img_addr}");

        // 设置 MIPI 快速模式
        mode = GI_MIPI_FAST_READER;
        ITESetMIPIModeAPI(&mut mode);

        // 打开 cover.jpg 格式文件
        let mut img = image::open("cover.jpg").unwrap();

        // 剪裁图片
        let (img_w, img_h) = (img.width(), img.height());
        img.crop((img_w - 2560) / 2, (img_h - 1600) / 2, 2560, 1600);

        // 转换为 8bit RGBA 图像
        // let mut img_rgba8 = img.into_rgba8();
        // let img_buf = img_rgba8.as_mut_ptr();

        // 转换为 16bit RGB 图像
        // let mut img_rgb16 = img.into_rgb16();
        // let img_buf = img_rgb16.as_mut_ptr() as *mut u8;

        // 转换为 8bit 灰度图像
        // let mut img_luma8 = img.into_luma8();
        // let img_buf = img_luma8.as_mut_ptr();

        // 转换为 16bit 灰度图像
        let mut img_luma16 = img.into_luma16();
        let img_buf = img_luma16.as_mut_ptr() as *mut u8;

        let ret = ITELoadImage(img_buf, img_addr, 0, 0, 2560, 1600);
        println!("ITELoadImage: {ret}");

        latest_image_idx = image_idx;

        let ret = ITEDisplayAreaAPI(0, 0, 2560, 1600, 2, img_addr, 0);
        println!("ITEDisplayAreaAPI: {ret}");

        ITECloseDeviceAPI();
        println!("ITECloseDeviceAPI");
    }
    Ok(())
}
