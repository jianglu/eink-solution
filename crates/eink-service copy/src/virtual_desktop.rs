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

use std::sync::Arc;

use anyhow::Result;
use cmd_lib::{run_cmd, run_fun};
use log::info;
use parking_lot::RwLock;

pub fn create_new_desktop(name: &str) -> anyhow::Result<()> {
    // with pipes
    let n = run_fun!(VirtualDesktop11.exe /NEW /NAME:"$name")?;
    println!("Create New Desktop: {}", n);
    Ok(())
}
