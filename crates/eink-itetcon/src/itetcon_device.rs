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

use std::{ffi::CString, mem::zeroed};

use anyhow::{bail, Result};
use log::info;
use widestring::U16CString;
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;

use crate::{
    DisableLoadImg, EiTurn180, EicConvertToT1000Format, EicLoadImage, EicReleaseImage,
    EnableLoadImg, ITECleanUpEInkAPI, ITECloseDeviceAPI, ITEDisplayAreaAPI,
    ITEGetBufferAddrInfoAPI, ITEGetDriveNo, ITEGetSystemInfoAPI, ITELoadImage, ITEOpenDeviceAPI,
    ITESetMIPIModeAPI, RecoveryLoadImg, StopLoadImg, EIMC_GRAY16, EIMC_IMG_FILL, GI_MIPI_BROWSER,
    GI_MIPI_FAST_READER, GI_MIPI_HYBRID, GI_MIPI_READER, TRSP_SYSTEM_INFO_DATA,ITEResetTcon,
};

pub struct IteTconDevice {
    drive_no: u8,
    dev_path: String,
    is_open: bool,
    img_addrs: [u32; 3],
    sysinfo: TRSP_SYSTEM_INFO_DATA,
    latest_image_idx: u32,
    screen_width: u32,
    screen_height: u32,
}

impl IteTconDevice {
    /// 创建设备对象
    /// TODO: 设备尺寸可配置
    pub fn new() -> Result<Self> {
        Ok(Self {
            drive_no: 0,
            dev_path: "".to_string(),
            is_open: true,
            img_addrs: unsafe { zeroed() },
            sysinfo: unsafe { zeroed() },
            latest_image_idx: u32::max_value(),
            screen_width: 2560,
            screen_height: 1600,
        })
    }

    /// 打开设备
    pub fn open(&mut self) -> Result<()> {
        // 获得设备驱动号
        let mut drive_no: u8 = 0;
        let ret = unsafe { ITEGetDriveNo(&mut drive_no) };
        info!("EinkTcon DriveNo: ret: {}, drive_no: {}", ret, drive_no);

        // 打开设备
        let dev_path = format!("\\\\.\\{}:", (0x41 + drive_no) as char);
        info!("EinkTcon Dev Path: {}", dev_path);

        let cstr = CString::new(dev_path.clone())?;
        info!("EinkTcon Dev Path C: {:?}", &cstr);

        if unsafe { ITEOpenDeviceAPI(&cstr) } == INVALID_HANDLE_VALUE {
            bail!("EinkTcon Open eink device fail, in thread");
        }

        // 获得设备系统信息
        let mut sysinfo: TRSP_SYSTEM_INFO_DATA = unsafe { zeroed() };
        let res = unsafe { ITEGetSystemInfoAPI(&mut sysinfo) };
        info!("EinkTcon ITEGetSystemInfoAPI: res: {res}");

        // 获得图片地址（支持 3 张图片），支持 3 张图片轮询
        let mut addrs: [u32; 3] = unsafe { zeroed() };
        unsafe { ITEGetBufferAddrInfoAPI(&mut addrs) };
        info!("EinkTcon ITEGetBufferAddrInfoAPI: addrs: {addrs:?}");

        self.drive_no = drive_no;
        self.dev_path = dev_path;
        self.img_addrs = addrs;
        self.sysinfo = sysinfo;
        self.is_open = true;

        Ok(())
    }

    /// 关闭设备
    pub fn close(&mut self) {
        unsafe { ITECloseDeviceAPI() };
        info!("ITECloseDeviceAPI");
        self.is_open = false;
    }

    pub fn refresh(&self) {
        info!("tcon_refresh");
        unsafe { StopLoadImg() };
        unsafe { ITECleanUpEInkAPI() };
        unsafe { RecoveryLoadImg() };
    }

    /// 设置为静态刷新模式
    pub fn set_speed_mode(&self) {
        // 设置 MIPI 快速模式
        info!("set_speed_mode GI_MIPI_FAST_READER");

        let mut mode = GI_MIPI_FAST_READER;
        unsafe { StopLoadImg() };
        unsafe { ITESetMIPIModeAPI(&mut mode) };
        unsafe { RecoveryLoadImg() };
    }

    /// 设置为静态刷新模式
    pub fn set_gybrid_mode(&self) {
        // 设置 MIPI 快速模式
        let mut mode = GI_MIPI_HYBRID;
        unsafe { StopLoadImg() };
        unsafe { ITESetMIPIModeAPI(&mut mode) };
        unsafe { RecoveryLoadImg() };
    }

    /// 设置为 READER 模式
    pub fn set_reader_mode(&self) {
        // 设置 MIPI 快速模式
        let mut mode = GI_MIPI_READER;

        unsafe { StopLoadImg() };
        unsafe { ITESetMIPIModeAPI(&mut mode) };
        unsafe { RecoveryLoadImg() };
    }

    // 设置显示 Cover 图像
    pub fn show_cover_image(&mut self) {
        self.set_speed_mode();

        if self.latest_image_idx == u32::max_value() {
            self.latest_image_idx = 0;
        }

        let img_addr = self.img_addrs[self.latest_image_idx as usize];
        let ret = unsafe {
            ITEDisplayAreaAPI(
                0,
                0,
                self.screen_width,
                self.screen_height,
                GI_MIPI_FAST_READER, // TODO: ?? 确认此接口的模式指定
                img_addr,
                0,
            )
        };

        self.refresh();
        // info!("ITEDisplayAreaAPI: {ret}");
        // info!("ITEDisplayAreaAPI(1): {}", ret);
    }

    /// 设置为 Cover 图像（SLOW，需要在后台线程运行）
    pub fn set_cover_image(&mut self, img_path: &str) {
        //
        // 计算当前可用图片地址
        let image_idx = if self.latest_image_idx == u32::max_value() {
            self.latest_image_idx = 0;
            0
        } else {
            (self.latest_image_idx + 1) % 2
        };
        self.latest_image_idx = image_idx;
        let img_addr = self.img_addrs[image_idx as usize];
        info!("img_addr: {image_idx}");

        // // 打开 cover.jpg 格式文件
        // let mut img = image::open(img_path).unwrap();

        // // 剪裁图片, 使其居中显示
        // // TODO: 增加其他剪裁算法
        // let (img_w, img_h) = (img.width(), img.height());
        // img.crop(
        //     (img_w - self.screen_width) / 2,
        //     (img_h - self.screen_height) / 2,
        //     self.screen_width,
        //     self.screen_height,
        // );

        // // 转换为 16bit 灰度图像
        // // // TODO: ColorEink 设备和黑白设备有区别，IT8951_USB_API 是否需要更新 ？
        // // let mut img_luma16 = img.into_luma16();
        // // let img_buf = img_luma16.as_mut_ptr() as *mut u8;

        // // 转换为 8bit 灰度图像
        // // let mut img_luma8 = img.into_luma8();
        // // let img_buf = img_luma8.as_mut_ptr() as *mut u8;

        // let mut img_luma8 = img.into_rgb8();
        // let img_buf = img_luma8.as_mut_ptr() as *mut u8;

        self.set_speed_mode();

        info!("EicLoadImage");
        let img_path_cstring = U16CString::from_str(img_path).unwrap();
        let mut img_width: u32 = 0;
        let mut img_height: u32 = 0;
        let img_buf = unsafe {
            EicLoadImage(
                img_path_cstring.as_ptr(),
                EIMC_GRAY16,
                EIMC_IMG_FILL,
                self.screen_width,
                self.screen_height,
                &mut img_width,
                &mut img_height,
            )
        };

        if !img_buf.is_null() {
            unsafe {
                info!("EicConvertToT1000Format");
                EicConvertToT1000Format(img_buf, img_width, img_height);

                info!("EiTurn180");
                EiTurn180(img_buf, img_width, img_height);
            }

            info!("ITELoadImage");
            let ret = unsafe {
                ITELoadImage(
                    img_buf,
                    img_addr,
                    0,
                    0,
                    self.screen_width,
                    self.screen_height,
                )
            };
            info!("ITELoadImage: {ret}");

            // 保存新的可用图片序号
            self.latest_image_idx = image_idx;

            // let ret = unsafe {
            //     ITEDisplayAreaAPI(
            //         0,
            //         0,
            //         self.screen_width,
            //         self.screen_height,
            //         GI_MIPI_BROWSER, // TODO: ?? 确认此接口的模式指定
            //         img_addr,
            //         0,
            //     )
            // };
            //info!("ITEDisplayAreaAPI: {ret}");
            self.set_gybrid_mode();
            unsafe { EicReleaseImage(img_buf) };
        }
    }
}
