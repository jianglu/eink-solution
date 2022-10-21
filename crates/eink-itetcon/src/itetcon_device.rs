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

pub struct IteTconDevice {
    drive_no: u8,
    dev_path: String,
    is_open: bool,
}

impl IteTconDevice {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            drive_no: 0,
            dev_path: "".to_string(),
            is_open: true,
        })
    }

    /// 设置为速度模式
    pub fn set_speed_mode() {
        
    }
}
