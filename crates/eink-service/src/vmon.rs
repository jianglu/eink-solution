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

use anyhow::Result;

use crate::iddcx::{get_iddcx_device_path, recreate_iddcx_device};

// 虚拟显示器控制器
pub struct VirtualMonitorService {
    dev_path: String,
}

impl VirtualMonitorService {
    pub fn new() -> Result<Self> {
        // 创建驱动程序实例
        recreate_iddcx_device()?;

        // 查找虚拟显示器设备路径
        let dev_path = get_iddcx_device_path()?;

        Ok(Self { dev_path })
    }
}

impl VirtualMonitorService {
    
}

impl Drop for VirtualMonitorService {
    fn drop(&mut self) {
        // 关闭驱动程序句柄，进程退出时也可以自动关闭
    }
}
