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

use log::info;
use parking_lot::Mutex;

struct TconService {}

impl TconService {
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static TCON_SERVICE: Mutex<TconService> = {
    info!("Create TconService");
    Mutex::new(TconService::new().expect("Cannot instantiate TconService"))
};
