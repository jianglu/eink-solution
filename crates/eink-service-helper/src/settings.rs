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

use config::Config;
use parking_lot::RwLock;

//
// 将 Native 库设置为 Lazy 全局变量
//
// C:\Windows\System32\config\systemprofile\AppData\Local\Lenovo\ThinkBookEinkPlus
//
#[static_init::dynamic(lazy)]
pub static SETTINGS: RwLock<Config> = {
    let mut config_dir = dirs::data_local_dir().unwrap_or_default();
    config_dir.push(&"Lenovo\\ThinkBookEinkPlus\\");

    // 如果配置文件目录不存在则创建
    // TODO: CHECK ERROR
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).expect("Cannot create config dirs");
    }

    let mut file_path = config_dir.clone();
    file_path.push("service-helper-settings.json");

    // 如果配置文件不存在，写入默认值
    // TODO: CHECK ERROR
    if !file_path.exists() {
        let bytes = include_bytes!("../default-service-helper-settings.json");
        std::fs::write(&file_path, bytes).expect("Cannot write default config file");
    }

    let settings = Config::builder()
        .add_source(config::File::from(file_path))
        .build()
        .expect("Cannot build config");

    RwLock::new(settings)
};

#[test]
fn test_settings() {
    use std::collections::HashMap;

    let settings = SETTINGS.read().unwrap().clone();
    // Print out our settings (as a HashMap)
    println!(
        "{:?}",
        settings
            .try_deserialize::<HashMap<String, String>>()
            .unwrap()
    );

    // std::thread::spawn(|| {
    //     let mut settings = SETTINGS.write().unwrap().clone();
    //     settings.set(key, value)
    // });
}
