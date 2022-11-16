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

use std::marker::PhantomData;

use windows::core::PCWSTR;
use windows::s;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK, WINEVENTPROC};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, GetMessageW, RegisterWindowMessageA, EVENT_OBJECT_DESTROY,
    EVENT_OBJECT_FOCUS, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND,
    EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZEEND, MSG,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_CLOSE, WM_CREATE, WM_HOTKEY, WM_NULL,
    WM_QUIT, WNDCLASSEXW,
};

/// Wrapper around a HWND windows pointer that destroys the window on drop
///
#[cfg(windows)]
struct HwndDropper(HWND);

#[cfg(windows)]
impl Drop for HwndDropper {
    fn drop(&mut self) {
        if !self.0 .0 == 0 {
            let _ = unsafe { DestroyWindow(self.0) };
        }
    }
}

#[static_init::dynamic(lazy)]
pub static WM_PRIV_SETTINGS_CHANGED: u32 =
    unsafe { RegisterWindowMessageA(s!("{11978F7B-221A-4E65-B8A8-693F7D6E4B25}")) };

pub struct AlwaysOnTop {
    /// Handle to the hidden window that is used to receive the hotkey events
    hwnd: HwndDropper,

    win_event_hooks: Vec<HWINEVENTHOOK>,

    /// Make sure that `HotkeyManager` is not Send / Sync. This prevents it from being moved
    /// between threads, which would prevent hotkey-events from being received.
    ///
    /// Being stuck on the same thread is an inherent limitation of the windows event system.
    _unimpl_send_sync: PhantomData<*const u8>,
}

/// 窗口置顶系统
impl AlwaysOnTop {
    ///
    pub fn new() -> anyhow::Result<Self> {
        // enable dpi awareness for this process()
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        // Try to create a hidden window to receive the windows events for the AlwaysOnTop.
        // If the window creation fails, HWND 0 (null) is used which registers hotkeys to the thread
        // message queue and gets messages from all thread associated windows
        let hwnd = create_hidden_window()?;

        // if this.init_main_window() {
        //     this.init_winhook_event_ids();

        //     AlwaysOnTopSettings::instance().init_file_watcher();
        //     AlwaysOnTopSettings::instance().load_settings();

        //     this.register_hotkey();
        //     this.register_llkh();

        //     this.subscribe_to_events();
        //     this.start_tracking_topmost_windows();
        // } else {
        //     log::error!("Failed to init AlwaysOnTop module");
        //     // TODO: show localized message
        // }

        Ok(Self {
            hwnd,
            win_event_hooks: Default::default(),
            _unimpl_send_sync: PhantomData,
        })
    }

    /// Run the event loop, listening for hotkeys. This will run indefinitely until interrupted and
    /// execute any hotkeys registered before.
    ///
    pub fn event_loop(&mut self) {
        while self.handle_message().is_some() {}
    }

    /// Wait for a single a message
    ///
    /// ## Windows API Functions used
    /// - <https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessagew>
    ///
    pub fn handle_message(&mut self) -> Option<u32> {
        loop {
            let mut msg = std::mem::MaybeUninit::<MSG>::uninit();

            // Block and read a message from the message queue. Filtered to receive messages from
            // WM_NULL to WM_HOTKEY
            if !unsafe {
                GetMessageW(
                    msg.as_mut_ptr(),
                    self.hwnd.0,
                    WM_NULL, // No message filter
                    WM_NULL,
                )
                .as_bool()
            } {
                return None;
            }

            let msg = unsafe { msg.assume_init() };

            if msg.message == *WM_PRIV_SETTINGS_CHANGED {
                log::info!("AlwaysOnTop::WM_PRIV_SETTINGS_CHANGED");
            } else {
                match msg.message {
                    WM_CREATE => {
                        log::info!("AlwaysOnTop::WM_CREATE");
                        self.subscribe_to_events();
                    }
                    WM_HOTKEY => {
                        log::info!("AlwaysOnTop::WM_HOTKEY");
                    }
                    WM_NULL => {
                        log::info!("AlwaysOnTop::WM_NULL");
                    }
                    WM_QUIT => return None,
                    _ => {
                        unsafe { DefWindowProcW(self.hwnd.0, msg.message, msg.wParam, msg.lParam) };
                    }
                }
            }
            return Some(msg.message);
        }
    }

    unsafe extern "system" fn hook_proc(
        hwineventhook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        idobject: i32,
        idchild: i32,
        ideventthread: u32,
        dwmseventtime: u32,
    ) {
    }

    // subscribe to windows events
    fn subscribe_to_events(&mut self) {
        for event in [
            EVENT_OBJECT_LOCATIONCHANGE,
            EVENT_SYSTEM_MINIMIZESTART,
            EVENT_SYSTEM_MINIMIZEEND,
            EVENT_SYSTEM_MOVESIZEEND,
            EVENT_SYSTEM_FOREGROUND,
            EVENT_OBJECT_DESTROY,
            EVENT_OBJECT_FOCUS,
        ] {
            let hook = unsafe {
                SetWinEventHook(
                    event,
                    event,
                    HINSTANCE::default(),
                    Some(Self::hook_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                )
            };
            self.win_event_hooks.push(hook);
        }
    }
    // /// 初始化消息窗口
    // pub fn init_main_window(&mut self) {
    //     let mut wcex: WNDCLASSEXW = Default::default();

    //     unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    //         LRESULT(0)
    //     };

    //     let instance = unsafe { GetModuleHandleW(PCWSTR::null()) };

    //     wcex.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
    //     wcex.lpfnWndProc = Some(wnd_proc);
    //     wcex.hInstance = instance;
    //     wcex.lpszClassName = w!("AlwaysOnTopWindow");

    //     let window = unsafe {
    //         RegisterClassExW(&wcex);
    //         CreateWindowExW(WS_EX_TOOLWINDOW, NonLocalizable::TOOL_WINDOW_CLASS_NAME, L"", WS_POPUP, 0, 0, 0, 0, nullptr, nullptr, instance, this);
    //     };

    //     if (!m_window)
    //     {
    //         Logger::error(L"Failed to create AlwaysOnTop window: {}", get_last_error_or_default(GetLastError()));
    //         return false;
    //     }

    //     return true;
    // }
}

/// Try to create a hidden "message-only" window
///
#[cfg(windows)]
fn create_hidden_window() -> anyhow::Result<HwndDropper> {
    // Scope imports
    use anyhow::bail;
    use windows::core::PCSTR;
    use windows::s;
    use windows::Win32::System::LibraryLoader::GetModuleHandleA;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExA, HWND_MESSAGE, WS_EX_TOOLWINDOW, WS_POPUP,
    };

    let hwnd = unsafe {
        // Get the current module handle
        let hinstance = GetModuleHandleA(PCSTR::null()).unwrap();

        CreateWindowExA(
            WS_EX_TOOLWINDOW,
            // The "Static" class is not intended for windows, but this shouldn't matter since the
            // window is hidden anyways
            s!("Static"),
            s!("AlwaysOnTopWindow"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            hinstance,
            None,
        )
    };

    if hwnd == HWND(0) {
        bail!("Cannot create always-on-top base window")
    } else {
        Ok(HwndDropper(hwnd))
    }
}
