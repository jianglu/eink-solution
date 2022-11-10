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

use eink_pipe_io::blocking::BlockingIpcConnection;
use jsonrpc_lite::Error;
use windows::w;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, ShowWindow, SW_HIDE, SW_RESTORE};

/// 启动服务线程
pub fn start_helper_thread() {
    std::thread::spawn(|| {
        loop {
            // 发生错误就不断重试
            match helper_thread_routine() {
                Ok(_) => unreachable!(),
                Err(err) => {
                    log::error!("helper_thread_routine: err: {err:?}")
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
}

fn helper_thread_routine() -> anyhow::Result<()> {
    let mut connection = BlockingIpcConnection::new()?;
    connection.connect("\\\\.\\pipe\\eink-service-helper")?;
    connection
        .on_request(|conn, request| {
            let id = request.get_id().unwrap();
            let method = request.get_method().unwrap();

            println!(
                "ServiceHelper: New Request: Id: {:?}, method: {:?}",
                id, method
            );

            match method {
                // 显示任务栏
                "show_taskbar" => {
                    eink_winkits::taskbar::set_auto_hide(false);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    unsafe { ShowWindow(FindWindowW(w!("Shell_TrayWnd"), None), SW_RESTORE) };
                    conn.reply_success(id, &serde_json::Value::Bool(true));
                }
                // 隐藏任务栏
                "hide_taskbar" => {
                    eink_winkits::taskbar::set_auto_hide(true);
                    unsafe { ShowWindow(FindWindowW(w!("Shell_TrayWnd"), None), SW_HIDE) };
                    conn.reply_success(id, &serde_json::Value::Bool(true));
                }
                _ => {
                    conn.reply_error(id, Error::method_not_found());
                }
            }
        })
        .detach();
    Ok(())
}
