// //
// // Copyright (C) Lenovo ThinkBook Gen4 Project.
// //
// // This program is protected under international and China copyright laws as
// // an unpublished work. This program is confidential and proprietary to the
// // copyright owners. Reproduction or disclosure, in whole or in part, or the
// // production of derivative works therefrom without the express permission of
// // the copyright owners is prohibited.
// //
// // All rights reserved.
// //



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

use std::process::{Child, Command};

use anyhow::Result;
use cmd_lib::run_cmd;
use log::info;
use parking_lot::Mutex;
use windows::Win32::System::Threading::GetCurrentProcessId;

use crate::{
    settings::SETTINGS,
    utils::{get_current_data_dir, get_current_exe_dir},
    win_utils::{kill_process_by_pid, run_as_admin},
};

/// 键盘管理器
pub struct KeyboardManager {
    pid: Option<u32>,
}

impl KeyboardManager {
    ///
    pub fn new() -> Result<Self> {
        Ok(Self { pid: None })
    }

    /// 启动服务
    pub fn start(&mut self) -> Result<()> {
        Ok(())
    }

    /// 禁用 Win / AltTab 按键
    /// 1. 启动 eink-keyboard-manager 进程
    pub fn disable_win_key(&mut self) -> Result<()> {
        // keyboard manager 可执行程序和 eink-service 在同一目录
        let exe_dir = get_current_exe_dir();
        let keyboard_manager_exe = exe_dir.join("eink-keyboard-manager.exe");

        // .\eink-keyboard-manager.exe /SettingsDir:"C:\Users\JiangLu\AppData\Local\Lenovo\ThinkBookEinkPlus\eink-keyboard-manager"

        let curr_pid = &unsafe { GetCurrentProcessId() }.to_string();

        let setting_dir = exe_dir.join("EinkKeyboardManager");
        let setting_dir = setting_dir.to_str().unwrap();

        // let process = Command::new(keyboard_manager_exe)
        //     .args([
        //         &format!("/Pid={}", curr_pid),
        //         &format!("/SettingsDir={}", setting_dir),
        //     ])
        //     .spawn()
        //     .expect("Cannot spawn keyboard manager instance");

        let pid = run_as_admin(
            exe_dir.to_str().unwrap(),
            &format!(
                "\"{}\" /Pid={} /SettingsDir=\"{}\"",
                keyboard_manager_exe.to_str().unwrap(),
                curr_pid,
                setting_dir
            ),
        )
        .unwrap();

        info!("eink-keyboard-manager pid: {pid}");

        self.pid = Some(pid);

        Ok(())
    }

    /// 启用 Win / AltTab 按键
    pub fn enable_win_key(&mut self) -> Result<()> {
        if let Some(pid) = self.pid.take() {
            kill_process_by_pid(pid, 0);
        }
        Ok(())
    }

    /// 停止服务
    /// 1. 停止 eink-keyboard-manager 进程
    pub fn stop(&mut self) -> Result<()> {
        self.enable_win_key()
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static KEYBOARD_MANAGER: Mutex<KeyboardManager> = {
    info!("Create KeyboardManager");
    Mutex::new(KeyboardManager::new().expect("Cannot instantiate KeyboardManager"))
};








































// use anyhow::Result;
// use windows::{Win32::{UI::{HiDpi::{
//     SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
// }, WindowsAndMessaging::WNDCLASSEXW}, System::LibraryLoader::GetModuleHandleA}, w};

// #[derive(Default)]
// struct AlwaysOnTop {}

// impl AlwaysOnTop {
//     pub fn new() -> Result<Self> {
//         // dpi_aware::enable_dpi_awareness_for_this_process();
//         unsafe {
//             SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
//         }

//         let mut this = Self::default();

//         this.init_main_window();
//         // InitializeWinhookEventIds();

//         // AlwaysOnTopSettings::instance().InitFileWatcher();
//         // AlwaysOnTopSettings::instance().LoadSettings();

//         // RegisterHotkey();
//         // RegisterLLKH();

//         // SubscribeToEvents();
//         // StartTrackingTopmostWindows();

//         Ok(Self {})
//     }

//     fn init_main_window(&mut self) -> Result<()> {
//         let instance = unsafe { GetModuleHandleA(None) }?;

//         let class_name = widestring::U16CStr::from("AlwaysOnTopWindow");

//         let wc = WNDCLASSEXW {
//             // hCursor: LoadCursorW(None, IDC_ARROW)?,
//             // hInstance: instance,
//             // lpszClassName: window_class,
//             // style: CS_HREDRAW | CS_VREDRAW,
//             // lpfnWndProc: Some(wndproc),
//             cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
//             lpfnWndProc: Some(wndproc_helper),
//             hInstance: instance,
//             lpszClassName: class_name.as_ptr(),
//             ..Default::default()
//         };

//         unsafe { RegisterClassExW(&wcex) };
    
//         m_window = CreateWindowExW(WS_EX_TOOLWINDOW, NonLocalizable::TOOL_WINDOW_CLASS_NAME, L"", WS_POPUP, 0, 0, 0, 0, nullptr, nullptr, m_hinstance, this);
//         if (!m_window)
//         {
//             Logger::error(L"Failed to create AlwaysOnTop window: {}", get_last_error_or_default(GetLastError()));
//             return false;
//         }
    
//     }
// }

// eink_topmost_helper
