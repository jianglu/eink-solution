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

use eink_itetcon::IteTconDevice;

fn main() -> anyhow::Result<()> {
    let mut device = IteTconDevice::new()?;
    device.open()?;
    device.set_speed_mode();
    device.set_cover_image("cover.jpg");
    device.close();
    Ok(())
}
