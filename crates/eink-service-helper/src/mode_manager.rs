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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use anyhow::Result;
use eink_winkits::get_window_text;
use log::info;
use parking_lot::Mutex;
use windows::s;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    PostMessageA, SendMessageA, SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
    SWP_SHOWWINDOW, WM_USER,
};
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS};
use winreg::RegKey;

use crate::settings::SETTINGS;
use crate::specialized::set_monitor_specialized;
use crate::topmost::TOPMOST_MANAGER;
use crate::utils::get_current_exe_dir;
use crate::win_utils::{
    find_window_by_classname, find_window_by_title, run_as_admin, set_window_hidden,
    set_window_maximize, set_window_minimize, set_window_shown,
};
use crate::wmi_service::WMI_SERVICE;
use crate::{monitor, save_display_mode_to_registry, tcon_api};

static IS_OLED: AtomicBool = AtomicBool::new(true);

/// 模式管理器
pub struct ModeManager {
    // // 显示器的 Monitor ID
    // eink_monitor_id: String,
    // oled_monitor_id: String,

    // 和模式切换线程的通讯端口
    tx: Sender<LaptopMode>,
}

#[derive(Debug)]
enum LaptopMode {
    OledWindowsDesktopMode,
    EinkLauncherMode,
}

impl ModeManager {
    /// 创建模式管理器
    /// 1. 从 SETTINGS 中读取 Monitors 的 ID
    /// 2. 创建切换线程，并维持和其的 MPSC 通讯
    pub fn new() -> Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel::<LaptopMode>();
        std::thread::spawn(move || Self::switching_thread_routine(rx));
        Ok(Self { tx })
    }

    /// 模式切换线程
    /// 在一个线程中统一管理模式切换流程，防止切换冲突等异常
    fn switching_thread_routine(rx: Receiver<LaptopMode>) {
        let eink_monitor_id = SETTINGS
            .read()
            .get_string("eink_monitor_id")
            .unwrap_or_default();

        let oled_monitor_id = SETTINGS
            .read()
            .get_string("oled_monitor_id")
            .unwrap_or_default();

        loop {
            // 读取通道中的首个 Mode
            if let Ok(mut req_mode) = rx.recv() {
                log::info!("switching_thread_routine: get request mode: {req_mode:?}");

                // 此时队列中可能还有其它 Mode 等待切换，因为如果切换事件请求的太频繁，队列中就会出现事件堆积
                'inner: loop {
                    match rx.try_recv() {
                        Ok(more_mode) => {
                            log::info!("switching_thread_routine: find more mode: {more_mode:?}");
                            req_mode = more_mode;
                        }
                        Err(_) => {
                            break 'inner;
                        }
                    }
                }

                // 切屏幕之前，保存当前 Foreground Window
                if let Ok(fg_hwnd) = crate::win_utils::get_foreground_window() {
                    save_foreground_window_to_registry(fg_hwnd);
                } else {
                    save_foreground_window_to_registry(HWND(0));
                }

                // 此时拿到了队列中最新的 Mode，进入这个模式
                match req_mode {
                    LaptopMode::OledWindowsDesktopMode => {
                        Self::switch_to_oled_windows_desktop_mode(
                            &eink_monitor_id,
                            &oled_monitor_id,
                        );
                    }
                    LaptopMode::EinkLauncherMode => {
                        Self::switch_to_eink_launcher_mode(&eink_monitor_id, &oled_monitor_id);
                    }
                }
            } else {
                log::warn!("ModeManager: Cannot receive laptop mode from channel");
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    /// 请求切换到 OledWindowsDesktopMode 模式
    pub fn request_to_oled_windows_desktop_mode(&mut self) {
        self.tx.send(LaptopMode::OledWindowsDesktopMode).unwrap();
    }

    /// 请求切换到 EinkLauncherMode 模式
    pub fn request_to_eink_launcher_mode(&mut self) {
        self.tx.send(LaptopMode::EinkLauncherMode).unwrap();
    }

    /// 切换模式
    pub fn request_switch_eink_oled_display(&mut self) {
        if IS_OLED.load(Ordering::Relaxed) {
            self.request_to_eink_launcher_mode();
        } else {
            self.request_to_oled_windows_desktop_mode();
        }
    }

    /// 切换到 EINK Launcher 模式
    fn switch_to_eink_launcher_mode(eink_monitor_id: &str, oled_monitor_id: &str) {
        log::info!("switch_to_eink_launcher_mode");

        set_monitor_specialized(eink_monitor_id, false).unwrap();

        // Sleep 100ms 等待 Windows Display 稳定
        std::thread::sleep(std::time::Duration::from_millis(100));

        set_monitor_specialized(oled_monitor_id, true).unwrap();

        let launcher_title = s!("ThinkbookEinkPlus2A7678FA-39DD-4C1D-8981-34A451919F59");

        // 如果 Launcher 不存在，启动
        if let Err(_err) = find_window_by_title(launcher_title) {
            start_launcher();
        }

        // Sleep 200ms 等待 TCON 稳定后才重置
        std::thread::sleep(std::time::Duration::from_millis(200));

        // 切换到eink时，要软件启动一下
        tcon_api::eink_software_reset_tcon();

        // 等待 10s Launcher 启动
        for _i in 0..100 {
            if let Ok(_hwnd) = find_window_by_title(launcher_title) {
                log::info!("Found launcher, go next");
                break;
            }
            log::info!("Cannot found launcher, wait 100ms");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // 置顶 Launcher
        find_launcher_and_set_topmost();

        // 置顶悬浮球
        find_floating_button_and_set_topmost();

        // 将当前模式保存到注册表
        save_display_mode_to_registry("EINK");

        // 使用 WMI 接口通知主板, 1 : OLED is Working
        WMI_SERVICE.lock().set_display_working_status(2);

        WMI_SERVICE.lock().get_display_working_status();

        IS_OLED.store(false, Ordering::Relaxed);

        // 重置 DPI 作为保护性操作，可以在非关键上下文中运行
        // 将 EINK 屏幕的 DPI 设置为 200，成功后再次重新尝试置顶 Launcher
        if let Err(err) = monitor::set_dpi_by_stable_monitor_id(&eink_monitor_id, 200) {
            log::error!("Cannot reset eink dpi to 200: err: {err}");
        } else {
            find_launcher_and_set_topmost();
        }

        // OLED 桌面模式采用 Hybrid Hybrid 模式

        // GI_MIPI_BROWSER = 0x02;
        // GI_MIPI_HYBRID = 0xF0;
        tcon_api::eink_set_mipi_mode(0xF0);

        // 设置 EINK 触摸区域
        tcon_api::eink_set_tp_mask_area(tcon_api::TOUCH_EVENT_BOTH, 1, 0, 2560, 0, 1600);

        log::info!("switch_to_eink_launcher_mode completed");
    }

    // 切换搭配 OLED Windows 桌面模式
    fn switch_to_oled_windows_desktop_mode(eink_monitor_id: &str, oled_monitor_id: &str) {
        log::info!("switch_to_oled_windows_desktop_mode");

        set_monitor_specialized(&oled_monitor_id, false).unwrap();

        // Sleep 100ms 等待 Windows Display 稳定
        std::thread::sleep(std::time::Duration::from_millis(100));

        set_monitor_specialized(&eink_monitor_id, true).unwrap();

        // 设置 EINK 触摸区域
        tcon_api::eink_set_tp_mask_area(tcon_api::TOUCH_EVENT_NO_REPORT, 1, 0, 2560, 0, 1600);

        // OLED 桌面模式采用 Hybrid Browser 模式

        // GI_MIPI_BROWSER = 0x02;
        // GI_MIPI_HYBRID = 0xF0;
        //tcon_api::eink_set_mipi_mode(0x02);

        // 最小化 Launcher
        find_launcher_and_set_hidden();

        // 清除当前置顶的窗口
        TOPMOST_MANAGER.lock().clear_current_topmost_window();

        // 将当前模式保存到注册表
        save_display_mode_to_registry("OLED");

        // 使用 WMI 接口通知主板, 2 : E-ink is Working
        WMI_SERVICE.lock().set_display_working_status(1);

        WMI_SERVICE.lock().get_display_working_status();

        IS_OLED.store(true, Ordering::Relaxed);

        // 显示壁纸
        tcon_api::eink_show_shutdown_cover();

        log::info!("switch_to_oled_windows_desktop_mode completed");
    }
}

//
// 将 Native 库设置为 Lazy 全局变量
//
#[static_init::dynamic(lazy)]
pub static MODE_MANAGER: Arc<Mutex<ModeManager>> = {
    info!("Create ModeManager");

    let this = Arc::new(Mutex::new(
        ModeManager::new().expect("Cannot instantiate ModeManager"),
    ));

    this
};

/// 查找 Launcher 并且设置为置顶模式
fn find_launcher_and_set_topmost() {
    info!("find_launcher_and_set_topmost");

    if let Ok(hwnd) =
        find_window_by_title(s!("ThinkbookEinkPlus2A7678FA-39DD-4C1D-8981-34A451919F59"))
    {
        set_window_shown(hwnd);
        set_window_maximize(hwnd);

        // 调用 Windows 的置顶方法
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_SHOWWINDOW | SWP_NOSIZE,
            );
            SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_SHOWWINDOW | SWP_NOSIZE,
            );
        }

        // set_window_topmost(hwnd);
    } else {
        log::error!("Cannot find ThinkBook Eink Plus Launcher");
        start_launcher();
    }
}

/// 查找悬浮球并且设置为置顶模式
/// 悬浮球已经不需要置顶了
pub fn find_floating_button_and_set_topmost() {
    info!("find_floating_button_and_set_topmost");

    // if let Ok(hwnd) = find_window_by_title(s!("86948044-41D9-464B-B533-15FE92A0BA26")) {
    //     // 使用 AlwaysOnTop 动态置顶
    //     set_window_topmost(hwnd);

    //     // 使用动态置顶后， 等待 250ms 再通过 Windows HWND_TOPMOST 避免其他动态置顶窗口遮蔽悬浮球
    //     std::thread::spawn(move || {
    //         //
    //         std::thread::sleep(std::time::Duration::from_millis(250));

    //         // 调用 Windows 的置顶方法
    //         unsafe {
    //             SetWindowPos(
    //                 hwnd,
    //                 HWND_TOPMOST,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 SWP_NOMOVE | SWP_SHOWWINDOW | SWP_NOSIZE,
    //             );
    //         }
    //     });
    // } else {
    //     log::error!("Cannot find ThinkBook Eink Plus Floating Button");
    // }
}

/// 启动 Launcher
fn start_launcher() {
    // TODO：临时通过 eink-service 来启动 launcher，类似锁屏笔记的方式
    tcon_api::eink_start_launcher();

    // // topmost manager 可执行程序和 eink-service 在同一目录
    // let exe_dir = get_current_exe_dir();
    // let launcher_exe = exe_dir.join("LenovoGen4.Launcher.exe");

    // let _pid = match crate::win_utils::run_with_ui_access(
    //     exe_dir.to_str().unwrap(),
    //     launcher_exe.to_str().unwrap(),
    // ) {
    //     Ok(_) => (),
    //     Err(_err) => {
    //         log::error!("Cannot run Launcher");
    //     }
    // };
}

/// 查找 Launcher 并且设置为隐藏
fn find_launcher_and_set_hidden() {
    if let Ok(hwnd) =
        find_window_by_title(s!("ThinkbookEinkPlus2A7678FA-39DD-4C1D-8981-34A451919F59"))
    {
        // Maybe: 用最小化窗口代替隐藏窗口
        set_window_hidden(hwnd);
        // set_window_minimize(hwnd);
    } else {
        log::error!("Cannot find ThinkBook Eink Plus Launcher");
    }
}

/// 设置窗口置顶
/// 1. 通知 Topmost Service
pub fn set_window_topmost(hwnd: HWND) {
    if let Ok(api_hwnd) = find_window_by_classname(s!("AlwaysOnTopWindow")) {
        let win_text = get_window_text(hwnd).unwrap_or("unknown window".to_string());

        log::info!("Send ({win_text}) Topmost Message To AlwaysOnTopWindow");

        unsafe {
            PostMessageA(api_hwnd, WM_USER, WPARAM::default(), LPARAM(hwnd.0));
        }
    }
}

/// 设置窗口置顶
/// 1. 通知 Topmost Service
pub fn clear_all_windows_topmost() {
    if let Ok(api_hwnd) = find_window_by_classname(s!("AlwaysOnTopWindow")) {
        log::error!("Send Clear Topmost Message To AlwaysOnTopWindow");
        unsafe {
            PostMessageA(api_hwnd, WM_USER + 2, WPARAM::default(), LPARAM::default());
        }
    }
}

/// 将当前显示模式保存到注册表
pub fn save_foreground_window_to_registry(hwnd: HWND) {
    let key_path = r#"SOFTWARE\Lenovo\ThinkBookEinkPlus"#;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let mut key = hklm.open_subkey_with_flags(key_path, KEY_ALL_ACCESS);

    if key.is_err() {
        // Maybe notfound, ignore any error
        let _ = hklm.create_subkey(key_path);

        key = hklm.open_subkey(key_path);
        if key.is_err() {
            // 多次错误，只能退出，输出到日志
            log::error!("Cannot open '{}' registry subkey", key_path);
            return;
        }
    }

    let key = key.unwrap();
    key.set_value("LastForegroundWindow", &(hwnd.0 as u32))
        .expect("Cannot save 'LastForegroundWindow' to registry");
}
