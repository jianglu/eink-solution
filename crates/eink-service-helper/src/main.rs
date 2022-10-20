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
use jsonrpc_lite::{Error, JsonRpc};
use windows::{
    w,
    Win32::{
        Foundation::LPARAM,
        UI::{
            Shell::{SHAppBarMessage, ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE, APPBARDATA},
            WindowsAndMessaging::{FindWindowW, ShowWindow, SW_HIDE, SW_RESTORE},
        },
    },
};

/// 服务助手程序
/// 在 admin 权限下运行，负责 system 权限无法进行的操作
fn main() -> anyhow::Result<()> {
    // 设置当前的活动日志系统为 OutputDebugString 输出
    eink_logger::init_with_level(log::Level::Trace)?;

    std::thread::spawn(|| {
        let mut connection = BlockingIpcConnection::new().unwrap();

        connection
            .connect("\\\\.\\pipe\\eink-service-helper")
            .unwrap();

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
                        set_task_bar_auto_hide(false);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        unsafe { ShowWindow(FindWindowW(w!("Shell_TrayWnd"), None), SW_RESTORE) };
                        conn.reply_success(id, &serde_json::Value::Bool(true));
                    }
                    // 隐藏任务栏
                    "hide_taskbar" => {
                        set_task_bar_auto_hide(true);
                        unsafe { ShowWindow(FindWindowW(w!("Shell_TrayWnd"), None), SW_HIDE) };
                        conn.reply_success(id, &serde_json::Value::Bool(true));
                    }
                    _ => {
                        conn.reply_error(id, Error::method_not_found());
                    }
                }
            })
            .detach();
    });

    std::thread::sleep(std::time::Duration::from_secs(365 * 24 * 60 * 60));

    Ok(())
}

pub fn set_task_bar_auto_hide(hide: bool) {
    let mut appbar: APPBARDATA = unsafe { std::mem::zeroed() };
    appbar.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    appbar.hWnd = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };
    appbar.lParam = LPARAM(if hide { ABS_AUTOHIDE } else { ABS_ALWAYSONTOP } as isize);
    unsafe { SHAppBarMessage(ABM_SETSTATE, &mut appbar) };
}
