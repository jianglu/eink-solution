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

use windows::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_SUCCESS, WAIT_OBJECT_0, WIN32_ERROR},
    System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
};

/// 等待进程终止
/// 在分离线程启动事件监听，如果目标进程终止，调用回调函数
/// 回调函数运行在
pub fn on_process_terminate<F>(pid: u32, cb: F)
where
    F: FnOnce(WIN32_ERROR) + Sync + Send + 'static,
{
    std::thread::spawn(move || unsafe {
        let process = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) // 打开父进程
            .expect("Cannot open parent process");
        if !process.is_invalid() {
            const INFINITE: u32 = 0xFFFFFFFFu32;
            if WaitForSingleObject(process, INFINITE) == WAIT_OBJECT_0 {
                CloseHandle(process);
                cb(ERROR_SUCCESS);
            } else {
                CloseHandle(process);
                cb(GetLastError());
            }
        } else {
            cb(GetLastError());
        }
    });
}
