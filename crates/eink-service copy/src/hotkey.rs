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

use std::{ptr::null_mut, thread::JoinHandle};

use anyhow::Result;
use log::info;
use mki::Keyboard;
use windows::Win32::{
    Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{CallNextHookEx, SetWindowsHookExW, HHOOK, WH_KEYBOARD_LL},
};
use windows_hotkeys::{
    keys::{ModKey, VKey},
    HotkeyManager,
};

use eink_eventbus::Event;

use crate::global::{HotKeyMessage, EVENTBUS, GENERIC_TOPIC};

pub struct HotkeyService {
    th: Option<JoinHandle<()>>,
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    info!("SetWindowsHookExW: code: {}", code);
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

impl HotkeyService {
    ///
    pub fn new() -> Result<Self> {
        let hhook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), HINSTANCE::default(), 0)? };
        info!("SetWindowsHookExW: {:?}", hhook);

        Keyboard::A.bind(|_| {
            info!("A pressed, sending B");
            Keyboard::B.click();
        });

        mki::enable_debug();
        mki::register_hotkey(&[Keyboard::LeftControl, Keyboard::B], || {
            info!("Ctrl+B Pressed")
        });

        let h = std::thread::spawn(|| {
            let mut hkm = HotkeyManager::new();

            hkm.register(VKey::A, &[ModKey::Ctrl, ModKey::Shift], || {
                info!("Hotkey Win + Shift + A was pressed");

                // 将热键消息发送至消息总线
                EVENTBUS.post(&Event::new(
                    GENERIC_TOPIC.clone(),
                    HotKeyMessage {
                        key: VKey::A,
                        key_modifiers: &[ModKey::Alt],
                    },
                ));
            })
            .unwrap();

            hkm.event_loop();
        });

        Ok(Self { th: Some(h) })
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        self.th.take().unwrap().join().unwrap();
    }
}
