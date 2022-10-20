use std::{collections::HashSet, ffi::CStr, mem::zeroed, path::PathBuf};

use anyhow::Result;
use cmd_lib::{run_cmd, spawn};
use log::info;
use widestring::{U16CStr, U16CString};
use windows::{
    core::{PCWSTR, PWSTR},
    w,
    Graphics::Capture::{GraphicsCaptureItem, IGraphicsCaptureItem},
    Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        Storage::FileSystem::STANDARD_RIGHTS_REQUIRED,
        System::{
            StationsAndDesktops::{
                CloseDesktop, CreateDesktopW, EnumDesktopWindows, EnumDesktopsW, GetThreadDesktop,
                OpenDesktopW, SwitchDesktop, DF_ALLOWOTHERACCOUNTHOOK,
            },
            SystemServices::{
                DESKTOP_CREATEMENU, DESKTOP_CREATEWINDOW, DESKTOP_ENUMERATE, DESKTOP_HOOKCONTROL,
                DESKTOP_JOURNALPLAYBACK, DESKTOP_JOURNALRECORD, DESKTOP_READOBJECTS,
                DESKTOP_SWITCHDESKTOP, DESKTOP_WRITEOBJECTS,
            },
            Threading::{
                CreateProcessW, GetCurrentThreadId, CREATE_NEW_CONSOLE, NORMAL_PRIORITY_CLASS,
                PROCESS_INFORMATION, STARTUPINFOW,
            },
            WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
        },
        UI::WindowsAndMessaging::{
            CloseWindow, EnumWindows, GetAncestor, GetClassNameA, GetDesktopWindow, GetWindowLongA,
            GetWindowLongW, GetWindowTextA, GetWindowTextW, GetWindowThreadProcessId,
            PostQuitMessage, PostThreadMessageA, RealGetWindowClassA, GA_ROOT, GET_ANCESTOR_FLAGS,
            GWL_STYLE, HCF_DEFAULTDESKTOP, WM_QUIT, WNDENUMPROC, WS_VISIBLE,
        },
    },
};

fn get_window_ancestor(hwnd: HWND) -> anyhow::Result<HWND> {
    unsafe {
        return Ok(GetAncestor(hwnd, GA_ROOT));
    }
}

fn get_window_class(hwnd: HWND) -> anyhow::Result<String> {
    unsafe {
        let mut buf: [u8; 256] = std::mem::zeroed();
        GetClassNameA(hwnd, &mut buf);
        let class_name = CStr::from_bytes_with_nul_unchecked(&buf);
        return Ok(class_name
            .to_str()?
            .trim_end_matches(|c: char| c == '\0')
            .to_string());
    }
}

fn get_window_real_class(hwnd: HWND) -> anyhow::Result<String> {
    unsafe {
        let mut buf: [u8; 256] = std::mem::zeroed();
        RealGetWindowClassA(hwnd, &mut buf);
        let class_name = CStr::from_bytes_with_nul_unchecked(&buf);
        return Ok(class_name
            .to_str()?
            .trim_end_matches(|c: char| c == '\0')
            .to_string());
    }
}

fn get_window_text(hwnd: HWND) -> anyhow::Result<String> {
    unsafe {
        let mut utf16 = vec![0x0u16; 1024];
        GetWindowTextW(hwnd, &mut utf16);
        let title = U16CString::from_vec_unchecked(utf16);
        Ok(title.to_string_lossy())
    }
}

fn find_all_windows() -> HashSet<isize> {
    unsafe extern "system" fn enum_hwnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let mut hwnds = Box::from_raw(lparam.0 as *mut HashSet<isize>);

        let hwnd_ancestor = GetAncestor(hwnd, GA_ROOT);
        if hwnd_ancestor == hwnd {
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let visible = (style & WS_VISIBLE.0) == WS_VISIBLE.0;

            if visible {
                hwnds.insert(hwnd.0);
            }
        }

        std::mem::forget(hwnds);
        BOOL(1)
    }

    let boxed_hwnds = Box::new(HashSet::<isize>::new());
    let boxed_hwnds_ptr = Box::into_raw(boxed_hwnds) as isize;

    unsafe {
        EnumWindows(Some(enum_hwnd), LPARAM(boxed_hwnds_ptr));
        let hwnds = Box::from_raw(boxed_hwnds_ptr as *mut HashSet<isize>);
        return *hwnds;
    }
}

fn main() -> Result<()> {
    // From MSDN
    // HDESK CreateDesktopA(
    //     [in]           LPCSTR                lpszDesktop,
    //                    LPCSTR                lpszDevice,
    //                    DEVMODEA              *pDevmode,
    //     [in]           DWORD                 dwFlags,
    //     [in]           ACCESS_MASK           dwDesiredAccess,
    //     [in, optional] LPSECURITY_ATTRIBUTES lpsa
    // );
    let orig_desk = unsafe { GetThreadDesktop(GetCurrentThreadId()).unwrap() };

    // Initialize logger with OutputDebugString
    eink_logger::init_with_env()?;

    const GENERIC_ALL: u32 = DESKTOP_CREATEMENU.0
        | DESKTOP_CREATEWINDOW.0
        | DESKTOP_ENUMERATE.0
        | DESKTOP_HOOKCONTROL.0
        | DESKTOP_JOURNALPLAYBACK.0
        | DESKTOP_JOURNALRECORD.0
        | DESKTOP_READOBJECTS.0
        | DESKTOP_SWITCHDESKTOP.0
        | DESKTOP_WRITEOBJECTS.0
        | STANDARD_RIGHTS_REQUIRED.0;

    let hdesk = unsafe {
        CreateDesktopW(
            w!("Eink Desktop"),
            None,
            None,
            DF_ALLOWOTHERACCOUNTHOOK,
            GENERIC_ALL,
            None,
        )
        .unwrap()
    };

    log::trace!("TRACE: Hello world");
    log::info!("INFO Hello world: hdesk: {:?}", hdesk);
    log::debug!("DEBUG: Hello world");
    log::error!("ERROR: Hello world");

    unsafe {
        let mut desktop_name = U16CString::from_str("Eink Desktop").unwrap();
        let mut si: STARTUPINFOW = zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        si.lpDesktop = PWSTR::from_raw(desktop_name.as_mut_ptr());
        // si.lpDesktop = PWSTR::from_raw(w!("winsta0\\default").as_ptr() as *mut u16);
        let mut pi: PROCESS_INFORMATION = zeroed();

        // let cmdline = "C:\\Program Files\\Lenovo\\Lenovo Reader\\net6.0-windows\\Lenovo.Reader.exe";
        let cmdline = "C:\\Program Files\\Lenovo\\ThinkBookEinkPlus\\EinkPlus.exe";

        let mut cmdline16 = U16CString::from_str(&cmdline).unwrap();

        let cmdline_path = PathBuf::from(&cmdline);
        let curr_dir = cmdline_path.parent().unwrap().to_str().unwrap();
        let curr_dir16 = U16CString::from_str(curr_dir).unwrap();

        let hwnds_before = find_all_windows();

        let ret = CreateProcessW(
            None,
            PWSTR::from_raw(cmdline16.as_mut_ptr()),
            None,
            None,
            false,
            NORMAL_PRIORITY_CLASS | CREATE_NEW_CONSOLE,
            None,
            Some(PCWSTR::from_raw(curr_dir16.as_ptr())),
            &si as *const STARTUPINFOW as *mut STARTUPINFOW,
            &mut pi,
        );

        println!("pi.dwProcessId = {}", pi.dwProcessId);
        println!("pi.dwThreadId = {}", pi.dwThreadId);

        let sys_time = std::time::SystemTime::now();
        let five_secs = std::time::Duration::from_secs(5);

        //     'outter: loop {
        //         let hwnds_after = find_all_windows();

        //         if sys_time.elapsed().unwrap() > five_secs {
        //             break;
        //         }

        //         if hwnds_after.len() > 0 {
        //             for hwnd in hwnds_after {
        //                 if hwnds_before.contains(&hwnd) {
        //                     continue;
        //                 }

        //                 let title = get_window_text(HWND(hwnd)).unwrap();
        //                 let class = get_window_class(HWND(hwnd)).unwrap();
        //                 let real_class = get_window_real_class(HWND(hwnd)).unwrap();

        //                 // let mut process_id: u32 = 0;
        //                 // GetWindowThreadProcessId(HWND(hwnd), Some(&mut process_id));
        //                 // info!(
        //                 //     "Window {}, ProcessId: {}, Title: {}, Class: {} / {}",
        //                 //     hwnd, process_id, &title, &class, &real_class
        //                 // );

        //                 let interop =
        //                     windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>(
        //                     )
        //                     .unwrap();

        //                 let result: Result<GraphicsCaptureItem, windows::core::Error> =
        //                     interop.CreateForWindow(HWND(hwnd));
        //                 if result.is_err() {
        //                     continue;
        //                 }

        //                 // DEBUG: 关闭窗口
        //                 // PostThreadMessageA(pi.dwThreadId, WM_QUIT, None, None);
        //                 break 'outter;
        //             }
        //         }
        //     }
    }

    std::thread::sleep(std::time::Duration::from_secs(1));

    unsafe {
        SwitchDesktop(hdesk);
    }

    std::thread::sleep(std::time::Duration::from_secs(20));

    unsafe {
        // let def_hdesk = OpenDesktopW(
        //     PCWSTR::from_raw(w!("winsta0\\default").as_ptr() as *mut u16),
        //     DF_ALLOWOTHERACCOUNTHOOK,
        //     false,
        //     GENERIC_ALL,
        // )
        // .unwrap();
        // GetThreadDesktop(dwthreadid)

        SwitchDesktop(orig_desk);
        // CloseDesktop(def_hdesk);
    }

    std::thread::sleep(std::time::Duration::from_secs(5));

    unsafe {
        CloseDesktop(hdesk);
    }

    Ok(())
}
