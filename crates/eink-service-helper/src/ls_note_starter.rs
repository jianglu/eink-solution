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
use std::mem::MaybeUninit;
use std::process::Command;
use std::sync::Arc;

use anyhow::bail;
use bitfield_struct::bitfield;
use windows::Win32::Devices::HumanInterfaceDevice::{
    HidP_GetCaps, HidP_GetUsageValue, HidP_GetValueCaps, HidP_Input, HIDP_CAPS, HIDP_VALUE_CAPS,
    HID_USAGE_GENERIC_X, HID_USAGE_GENERIC_Y, HID_USAGE_PAGE_GENERIC,
};
use windows::Win32::Foundation::{GetLastError, HANDLE, HWND};
use windows::Win32::System::RemoteDesktop::{
    WTSGetActiveConsoleSessionId, WTSQuerySessionInformationA, WTSSessionInfoEx, WTSINFOEXA,
    WTS_INFO_CLASS,
};
use windows::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoA, RegisterRawInputDevices, HRAWINPUT, RAWINPUT,
    RAWINPUTDEVICE, RAWINPUTHEADER, RIDEV_INPUTSINK, RIDI_DEVICENAME, RIDI_PREPARSEDDATA,
    RID_DEVICE_INFO_TYPE, RID_INPUT, RIM_TYPEHID,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyWindow, GetMessageW, MSG, WM_INPUT, WM_NULL};

struct HwndDropper(HWND);

#[cfg(windows)]
impl Drop for HwndDropper {
    fn drop(&mut self) {
        if self.0 != HWND::default() {
            let _ = unsafe { DestroyWindow(self.0) };
        }
    }
}

/// 锁屏笔记启动器
pub struct LockScreenNoteManager {
    hwnd: HwndDropper,

    first_point: Option<[u32; 2]>,

    lsn_launched: bool,

    /// Make sure that `HotkeyManager` is not Send / Sync. This prevents it from being moved
    /// between threads, which would prevent hotkey-events from being received.
    ///
    /// Being stuck on the same thread is an inherent limitation of the windows event system.
    _unimpl_send_sync: PhantomData<*const u8>,
}

impl LockScreenNoteManager {
    ///
    /// Create a new LockScreenNoteStarter instance. This instance can't be moved to other threads due to
    /// limitations in the windows events system.
    ///
    pub fn new() -> anyhow::Result<Self> {
        // Try to create a hidden window to receive the hotkey events for the HotkeyManager.
        // If the window creation fails, HWND 0 (null) is used which registers hotkeys to the thread
        // message queue and gets messages from all thread associated windows
        let hwnd = create_hidden_window().unwrap_or(HwndDropper(HWND::default()));
        Ok(Self {
            hwnd,
            first_point: None,
            lsn_launched: false,
            _unimpl_send_sync: PhantomData,
        })
    }

    /// Register as Pen Detecter
    /// TODO: usUsagePage, usUsage must match the hardware device
    pub fn start(&mut self) -> anyhow::Result<()> {
        let rid = [
            RAWINPUTDEVICE {
                usUsagePage: 0x0d,
                usUsage: 0x02,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: self.hwnd.0.clone(),
            },
            RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x06,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: self.hwnd.0.clone(),
            },
        ];

        let ret = unsafe {
            RegisterRawInputDevices(&rid, std::mem::size_of::<RAWINPUTDEVICE>() as u32).as_bool()
        };

        if !ret {
            bail!("Cannot RegisterRawInputDevices for Pen Detecter");
        }

        Ok(())
    }

    /// Wait for a single a hotkey event and execute the callback if all keys match. This returns
    /// the callback result if it was not interrupted. The function call will block until a hotkey
    /// is triggered or it is interrupted.
    ///
    /// If the event is interrupted, `None` is returned, otherwise `Some` is returned with the
    /// return value of the executed callback function.
    ///
    /// ## Windows API Functions used
    /// - <https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessagew>
    ///
    pub fn handle_event(&mut self) -> Option<()> {
        loop {
            let mut msg = MaybeUninit::<MSG>::uninit();

            // Block and read a message from the message queue. Filtered to receive messages from
            // WM_NULL to WM_HOTKEY
            let ok = unsafe { GetMessageW(msg.as_mut_ptr(), self.hwnd.0, WM_NULL, WM_INPUT) };

            if ok.as_bool() {
                let msg = unsafe { msg.assume_init() };

                if WM_INPUT == msg.message {
                    self.handle_input_message(msg);
                } else if WM_NULL == msg.message {
                    return None;
                }
            }
        }
    }

    /// Run the event loop, listening for hotkeys. This will run indefinitely until interrupted and
    /// execute any hotkeys registered before.
    ///
    pub fn event_loop(&mut self) {
        while self.handle_event().is_some() {}
    }

    /// Processing WM_INPUT message from HID device
    fn handle_input_message(&mut self, msg: MSG) {
        // 如果不在锁屏界面，清除首个锁屏落笔点的数据
        if !is_in_lockscreen() {
            self.first_point = None;
            self.lsn_launched = false;
            return;
        }

        // 如果在锁屏界面，并且锁屏笔记已经启动，忽略剩下的触笔事件
        if self.lsn_launched {
            return;
        }

        let mut buf: [u8; 0x1000] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut buf_len = buf.len() as u32;

        let _data_size = unsafe {
            GetRawInputData(
                HRAWINPUT(msg.lParam.0),
                RID_INPUT,
                Some(buf.as_mut_ptr() as *mut libc::c_void),
                &mut buf_len,
                std::mem::size_of::<RAWINPUTHEADER>() as u32,
            )
        };

        let rawdata: &mut RAWINPUT = unsafe { &mut *(buf.as_mut_ptr() as *mut RAWINPUT) };

        if RID_DEVICE_INFO_TYPE(rawdata.header.dwType) != RIM_TYPEHID {
            return;
        }

        let device = rawdata.header.hDevice;

        // device_name: \\?\HID#WACF2200&Col05#5&31f12b3c&0&0004#{4d1e55b2-f16f-11cf-88cb-001111000030}
        // let device_name = hid_get_device_name(device).unwrap_or("Unknown Device".to_string());
        // log::error!("XXXXX device_name: {device_name}");

        let preparsed_data = match hid_get_preparsed_data(device) {
            Ok(v) => v,
            Err(_) => return,
        };

        // let mut caps: HIDP_CAPS = unsafe { std::mem::zeroed() };
        // let ret = unsafe { HidP_GetCaps(preparsed_data.as_ptr() as isize, &mut caps) };
        // if ret.is_ok() {
        //     // HIDP_VALUE_CAPS* value_caps = (HIDP_VALUE_CAPS*)malloc(caps.NumberInputValueCaps * sizeof(HIDP_VALUE_CAPS));
        //     let mut value_caps =
        //         Vec::<HIDP_VALUE_CAPS>::with_capacity(caps.NumberInputValueCaps as usize);
        //     let mut value_caps_count = caps.NumberInputValueCaps;
        //     unsafe { value_caps.set_len(value_caps_count as usize) };
        //     log::error!("XXXXX value_caps_count: {value_caps_count}");

        //     let ret = unsafe {
        //         HidP_GetValueCaps(
        //             HidP_Input,
        //             value_caps.as_mut_ptr(),
        //             &mut value_caps_count,
        //             preparsed_data.as_ptr() as isize,
        //         )
        //     };

        //     if ret.is_ok() {
        //         for i in 0..value_caps_count as usize {
        //             let mut usage_value = 0u32;
        //             unsafe {
        //                 let mut report_data = rawdata.data.hid.bRawData;
        //                 let report_size = rawdata.data.hid.dwSizeHid as usize;

        //                 let mut report = unsafe {
        //                     Vec::from_raw_parts(report_data.as_mut_ptr(), report_size, report_size)
        //                 };

        //                 let ret = HidP_GetUsageValue(
        //                     HidP_Input,
        //                     value_caps[i].UsagePage,
        //                     0,
        //                     value_caps[i].Anonymous.Range.UsageMin,
        //                     &mut usage_value,
        //                     preparsed_data.as_ptr() as isize,
        //                     report.as_mut_slice(),
        //                 );

        //                 // let usage_value =
        //                 //     usage_value << (16 - value_caps[i].BitSize) & 0xffff;

        //                 std::mem::forget(report);

        //                 // HID_USAGE_GENERIC_X
        //                 // usage_value: 16350
        //                 // usage_value: 24978
        //                 // usage_value: 0
        //                 // usage_value: 4608
        //                 // usage_value: 0
        //                 // usage_value: 0
        //                 // usage_value: 0
        //                 // usage_value: 0
        //                 // usage_value: 18

        //                 if ret.is_ok() {
        //                     log::error!(
        //                         "XXXXX usage_page: {}, usage: {}, usage_value: {usage_value}",
        //                         value_caps[i].UsagePage,
        //                         value_caps[i].Anonymous.Range.UsageMin
        //                     );
        //                 } else {
        //                     //   HIDP_STATUS_INVALID_REPORT_LENGTH    = NTSTATUS($C0110003);
        //                     log::error!(
        //                         "XXXXX Err: {:?}, GetLastError: {:?}",
        //                         ret.unwrap_err(),
        //                         GetLastError()
        //                     );
        //                 }
        //             }
        //         }
        //     } else {
        //         log::error!("XXXXX Cannot HidP_GetValueCaps for device: {device_name}");
        //     }
        // } else {
        //     log::error!("XXXXX Cannot HidP_GetCaps for device: {device_name}");
        // }

        // Get Value
        let report_count = unsafe { rawdata.data.hid.dwCount as usize };

        for i in 0..report_count {
            let report_size = unsafe { rawdata.data.hid.dwSizeHid as usize };
            let report_raw_p = unsafe { rawdata.data.hid.bRawData.as_mut_ptr() };
            let report_raw_p = unsafe { report_raw_p.add(report_size * i) };
            let mut report = unsafe { Vec::from_raw_parts(report_raw_p, report_size, report_size) };

            let mut generic_x = 0u32;
            let mut generic_y = 0u32;
            let mut pressure = 0u32;

            let _ret = unsafe {
                HidP_GetUsageValue(
                    HidP_Input,
                    HID_USAGE_PAGE_GENERIC,
                    0,
                    HID_USAGE_GENERIC_X,
                    &mut generic_x,
                    preparsed_data.as_ptr() as isize,
                    report.as_mut_slice(),
                )
            };

            let _ret = unsafe {
                HidP_GetUsageValue(
                    HidP_Input,
                    HID_USAGE_PAGE_GENERIC,
                    0,
                    HID_USAGE_GENERIC_Y,
                    &mut generic_y,
                    preparsed_data.as_ptr() as isize,
                    report.as_mut_slice(),
                )
            };

            const HID_USAGE_PAGE_DIGITIZER: u16 = 0x0D;
            const HID_USAGE_DIGITIZER_TIP_PRESSURE: u16 = 0x30;

            let _ret = unsafe {
                HidP_GetUsageValue(
                    HidP_Input,
                    HID_USAGE_PAGE_DIGITIZER,
                    0,
                    HID_USAGE_DIGITIZER_TIP_PRESSURE,
                    &mut pressure,
                    preparsed_data.as_ptr() as isize,
                    report.as_mut_slice(),
                )
            };

            std::mem::forget(report);

            log::error!("XXXXX Position: {generic_x}, {generic_y} P {pressure}");

            if self.first_point.is_none() {
                // 首个点
                self.first_point = Some([generic_x, generic_y]);
            } else {
                // 判断和首个点之间的距离，如果大于某个阈值，启动锁屏笔记

                const threshold: f32 = 500.0;

                let fp = self.first_point.as_ref().unwrap();

                if f32::sqrt(
                    (generic_x as f32 - fp[0] as f32) * (generic_x as f32 - fp[0] as f32)
                        + (generic_y as f32 - fp[1] as f32) * (generic_y as f32 - fp[1] as f32),
                ) > threshold
                {
                    log::error!("XXXXX Launch LockScreen Note");
                    start_lockscreen_note();
                    self.lsn_launched = true;
                }
            }
        }
    }
}

/// 启动锁屏笔记
fn start_lockscreen_note() {
    let _ = std::thread::spawn(|| {
        let exe_path = "C:\\Program Files\\Lenovo\\ThinkBookNotePlus\\EInkLockSNote.exe";
        match Command::new(exe_path).spawn() {
            Ok(_) => (),
            Err(err) => {
                log::error!("Cannot spawn EInkLockSNote: {:?}", err);
            }
        }
    });
}

/// 判断是否处于锁屏界面
fn is_in_lockscreen() -> bool {
    let session_id = unsafe { WTSGetActiveConsoleSessionId() };
    let mut pbuffer = windows::core::PSTR::null();
    let mut pbuffer_size: u32 = 0;
    unsafe {
        WTSQuerySessionInformationA(
            HANDLE::default(),
            session_id,
            WTSSessionInfoEx,
            &mut pbuffer,
            &mut pbuffer_size,
        )
    };

    let wts_info = unsafe { &*(pbuffer.0 as *mut WTSINFOEXA) };

    if wts_info.Level != 1 {
        return false;
    }

    let session_flags = unsafe { wts_info.Data.WTSInfoExLevel1.SessionFlags };

    if session_flags != 0 {
        return false;
    }

    true
}

/// 获取 HID 设备名称
fn hid_get_device_name(device: HANDLE) -> anyhow::Result<String> {
    unsafe {
        let mut buf_len = 0u32;

        // Probe buffer size
        let size = GetRawInputDeviceInfoA(Some(device), RIDI_DEVICENAME, None, &mut buf_len);
        if size != 0 {
            bail!("Cannot probe buffer size for hid device {device:?}");
        }

        let mut buf: Vec<u8> = Vec::with_capacity(buf_len as usize);
        let size = GetRawInputDeviceInfoA(
            Some(device),
            RIDI_DEVICENAME,
            Some(buf.as_mut_ptr() as *mut libc::c_void),
            &mut buf_len,
        );
        if (size as i32) < 0 {
            bail!("Cannot get device name for hid device {device:?}");
        }

        buf.set_len(size as usize);

        Ok(String::from_utf8(buf)?)
    }
}

/// 获取 HID 设备名称
fn hid_get_preparsed_data(device: HANDLE) -> anyhow::Result<Vec<u8>> {
    unsafe {
        let mut buf_len = 0u32;

        // HIDP_VALUE_CAPS
        // Probe buffer size
        let size = GetRawInputDeviceInfoA(Some(device), RIDI_PREPARSEDDATA, None, &mut buf_len);
        if size != 0 {
            bail!("Cannot probe RIDI_PREPARSEDDATA size for hid device {device:?}");
        }

        let mut buf: Vec<u8> = Vec::with_capacity(buf_len as usize);
        let size = GetRawInputDeviceInfoA(
            Some(device),
            RIDI_PREPARSEDDATA,
            Some(buf.as_mut_ptr() as *mut libc::c_void),
            &mut buf_len,
        );
        if (size as i32) < 0 {
            bail!("Cannot get RIDI_PREPARSEDDATA for hid device {device:?}");
        }

        buf.set_len(size as usize);

        Ok(buf)
    }
}

/// Try to create a hidden "message-only" window
///
#[cfg(windows)]
fn create_hidden_window() -> Result<HwndDropper, ()> {
    use windows::core::PCSTR;
    use windows::s;
    use windows::Win32::System::LibraryLoader::GetModuleHandleA;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExA, HWND_MESSAGE, WS_DISABLED, WS_EX_NOACTIVATE,
    };

    let hwnd = unsafe {
        // Get the current module handle
        let hinstance = GetModuleHandleA(PCSTR::null()).unwrap();
        CreateWindowExA(
            WS_EX_NOACTIVATE,
            // The "Static" class is not intended for windows, but this shouldn't matter since the
            // window is hidden anyways
            Some(s!("Static")),
            Some(s!("")),
            WS_DISABLED,
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
    if hwnd == HWND::default() {
        Err(())
    } else {
        Ok(HwndDropper(hwnd))
    }
}
