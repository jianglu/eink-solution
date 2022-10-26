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
    sync::{
        atomic::{AtomicIsize, AtomicU32, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

use log::info;
use parking_lot::RwLock;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{BringWindowToTop, ShowWindow, SW_MINIMIZE},
};

use crate::AnyResult;

use super::window_overlapping;

/// 窗口守望者
/// 1. 确保 “单一应用” 不被其他应用覆盖
pub struct Watcher {
    // Launcher 窗口
    launcher_hwnd: Arc<AtomicIsize>,

    // Topmost App 置顶应用窗口
    topmost_app_hwnd: Arc<AtomicIsize>,

    // Watch 线程
    watch_thread: JoinHandle<()>,

    // 排除的窗口
    exclude_hwnds: Arc<RwLock<Vec<HWND>>>,
}

impl Watcher {
    pub fn new() -> AnyResult<Self> {
        let launcher_hwnd = Arc::new(AtomicIsize::new(0));
        let topmost_app_hwnd = Arc::new(AtomicIsize::new(0));

        let launcher_hwnd_cloned = launcher_hwnd.clone();
        let topmost_app_hwnd_cloned = topmost_app_hwnd.clone();

        let exclude_hwnds = Arc::new(RwLock::new(Vec::default()));
        let exclude_hwnds_cloned = exclude_hwnds.clone();

        // Watch 线程
        let watch_thread = std::thread::spawn(move || loop {
            // 对于置顶的应用
            let hwnd_id = { topmost_app_hwnd_cloned.load(Ordering::Relaxed) };
            if hwnd_id != 0 {
                Watcher::watch_window(HWND(hwnd_id), &exclude_hwnds_cloned);
            }

            // Launcher 不需要特殊处理

            // 等待 1000ms
            std::thread::sleep(std::time::Duration::from_millis(1000));
        });

        Ok(Self {
            launcher_hwnd,
            topmost_app_hwnd,
            watch_thread,
            exclude_hwnds,
        })
    }

    // 设置置顶应用
    pub fn set_topmost_app_hwnd(&mut self, hwnd: HWND) {
        self.topmost_app_hwnd.store(hwnd.0, Ordering::Relaxed);
    }

    /// 开始守望某应用
    /// 枚举所有桌面顶级窗口
    /// 如果窗口和被守望窗口有重叠，将他们移到底层或者最小化
    fn watch_window(hwnd: HWND, exclude_hwnds: &Arc<RwLock<Vec<HWND>>>) {
        // 先主动置顶
        unsafe { BringWindowToTop(hwnd) };

        // 等待 10ms 让 Windows 处理正确
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 判断覆盖情况，如果存在覆盖的应用程序，说明其采用某种 TOPMOST 技术，采用最小化方式使其避让
        let desktop_wins = window_overlapping::enumerate_overlapping_windows(hwnd);

        for overlapping_win in desktop_wins.into_iter() {
            let mut excluded = false;

            for exwin in exclude_hwnds.read().iter() {
                if overlapping_win.handle.0 == exwin.0 {
                    excluded = true;
                    break;
                }
            }

            // Launcher 窗口必须被排除
            // [18680] INFO  [eink_service_helper::window::window_watcher] OverlappingWin-Title: "ThinkbookEinkPlus2A7678FA-39DD-4C1D-8981-34A451919F59"

            if overlapping_win.title.contains("ThinkbookEinkPlus2A7678FA-39DD-4C1D-8981-34A451919F59") {
                excluded = true;
            }

            if excluded {
                // 有窗口被排除的情况
            } else {
                info!("OverlappingWin-Title: {:?}", overlapping_win.title);

                // TODO: 如果是特权级窗口，比如在更高优先级的 Band 上的窗口
                // unsafe { ShowWindow(overlapping_win.handle, SW_MINIMIZE) };
            }
        }
    }

    pub fn add_exclude_win(&mut self, hwnd: HWND) {
        self.exclude_hwnds.write().push(hwnd)
    }

    pub fn clear_exclude_win(&mut self) {
        self.exclude_hwnds.write().clear()
    }
}
