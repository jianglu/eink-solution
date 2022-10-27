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

use std::path::PathBuf;

/// 获得当前 exe 所在目录
pub fn get_current_exe_dir() -> PathBuf {
    let exe_path = std::env::current_exe().expect("Cannot get current exe path from env");
    let exe_dir = exe_path.parent().expect("Current exe path is wrong");
    exe_dir.to_owned()
}

/// 获得当前数据存储目录
pub fn get_current_data_dir() -> PathBuf {
    let mut data_dir = dirs::data_local_dir().expect("Cannot get data local dir");
    data_dir.push(&"Lenovo\\ThinkBookEinkPlus\\");
    data_dir
}
