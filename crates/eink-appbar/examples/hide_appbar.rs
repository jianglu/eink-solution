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

use eink_appbar::hide_taskbar;

fn main() -> anyhow::Result<()> {
    eink_logger::init()?;
    hide_taskbar();
    std::thread::sleep(std::time::Duration::from_millis(000));
    Ok(())
}
