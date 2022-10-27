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
