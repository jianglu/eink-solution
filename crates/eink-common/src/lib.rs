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

use std::{
    io,
    path::{Path, PathBuf},
};

/// 获得日志存储目录
///
/// %localappdata%\Lenovo\ThinkBookEinkPlus\logging
//
pub fn get_eink_logging_dir() -> PathBuf {
    let mut local_dir = dirs::data_local_dir().unwrap_or_default();
    local_dir.push(&"Lenovo\\ThinkBookEinkPlus\\logging");
    local_dir
}

/// 如果目录不存在则创建
pub fn create_dir_if_not_exists<P>(dir_path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    if !dir_path.as_ref().exists() {
        std::fs::create_dir_all(dir_path.as_ref())?;
    }
    Ok(())
}

#[test]
fn test_get_eink_logging_dir() {
    let logging_dir = get_eink_logging_dir();
    assert!(logging_dir.ends_with("logging"));
}

#[test]
fn test_create_dir_if_not_exists() {
    let logging_dir = get_eink_logging_dir();
    create_dir_if_not_exists(&logging_dir).expect("create_dir_if_not_exists");
}
