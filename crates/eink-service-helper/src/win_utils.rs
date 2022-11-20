use std::ffi::CStr;
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
use std::mem::{size_of, zeroed, MaybeUninit};

use anyhow::{bail, Result};
use libc::c_void;
#[allow(non_snake_case)]
use log::Level;
use log::{debug, info};
use widestring::{U16CStr, U16CString};
use windows::core::{PCSTR, PCWSTR, PWSTR};
use windows::w;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, BOOL, HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM, NO_ERROR,
    WIN32_ERROR,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL};
use windows::Win32::Security::{
    DuplicateTokenEx, GetTokenInformation, SecurityIdentification, SecurityImpersonation,
    SetTokenInformation, TokenLinkedToken, TokenPrimary, TokenSessionId, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_ADJUST_SESSIONID, TOKEN_ALL_ACCESS, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY,
    TOKEN_READ, TOKEN_WRITE,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
use windows::Win32::System::RemoteDesktop::{
    WTSActive, WTSEnumerateSessionsW, WTSFreeMemory, WTSGetActiveConsoleSessionId,
    WTSQueryUserToken, WTS_SESSION_INFOW,
};
use windows::Win32::System::SystemServices::MAXIMUM_ALLOWED;
use windows::Win32::System::Threading::{
    CreateProcessAsUserW, OpenProcess, OpenProcessToken, TerminateProcess, CREATE_NEW_CONSOLE,
    CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, NORMAL_PRIORITY_CLASS, PROCESS_ALL_ACCESS,
    PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, PROCESS_TERMINATE, STARTUPINFOW,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, SHGetFileInfoW, StrStrW, ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE,
    SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowA, FindWindowW, GetAncestor, GetClassNameA, GetClassNameW, GetDesktopWindow,
    GetForegroundWindow, GetWindowLongW, ShowWindow, GA_ROOT, GWL_EXSTYLE, GWL_STYLE, SW_HIDE,
    SW_MAXIMIZE, SW_MINIMIZE, SW_SHOW, WS_DISABLED, WS_EX_TOOLWINDOW,
};

/// 通过进程名获取进程 PID
pub fn get_process_pid(name: &str) -> Result<u32> {
    unsafe { get_process_pid_unsafe(name) }
}

/// 通过进程名获取进程 PID
pub unsafe fn get_process_pid_unsafe(name: &str) -> Result<u32> {
    let process_snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;

    if process_snap == INVALID_HANDLE_VALUE {
        bail!("CreateToolhelp32Snapshot Failed")
    }

    let mut pe32: PROCESSENTRY32W = zeroed();
    pe32.dwSize = size_of::<PROCESSENTRY32W>() as u32;

    let mut ret = Process32FirstW(process_snap, &mut pe32);

    // 创建临时宽字符
    let name16 = U16CString::from_str(name)?;

    let mut pid = 0;

    while ret.as_bool() {
        // 使用 CStr 避免内存分配
        let exe = U16CStr::from_ptr_str(pe32.szExeFile.as_ptr());

        info!("{:?}", exe);

        if exe == name16 {
            pid = pe32.th32ProcessID;
            break;
        }

        ret = Process32NextW(process_snap, &mut pe32);
    }

    if process_snap != INVALID_HANDLE_VALUE {
        CloseHandle(process_snap);
    }

    Ok(pid)
}

pub fn run_system_privilege(proc_name: &str, proc_dir: &str, proc_cmd: &str) -> Result<u32> {
    unsafe { run_system_privilege_unsafe(proc_name, proc_dir, proc_cmd) }
}

pub unsafe fn run_system_privilege_unsafe(
    proc_name: &str,
    proc_dir: &str,
    proc_cmd: &str,
) -> Result<u32> {
    let mut creation_flags = NORMAL_PRIORITY_CLASS | CREATE_NEW_CONSOLE | CREATE_NO_WINDOW;
    let winlogon_pid = get_process_pid("winlogon.exe")?;

    let mut process: HANDLE = HANDLE::default();
    let mut token: HANDLE = HANDLE::default();
    let mut environment: *mut c_void = zeroed();
    let mut user_token_dup: HANDLE = HANDLE::default();

    let mut error_code: WIN32_ERROR = NO_ERROR;

    let mut pi: PROCESS_INFORMATION = zeroed();

    loop {
        if winlogon_pid == 0 {
            break;
        }

        let mut desktop_name = widestring::U16CString::from_str("winsta0\\default").unwrap();

        let mut si: STARTUPINFOW = zeroed();
        si.lpDesktop = PWSTR::from_raw(desktop_name.as_mut_ptr());

        process = OpenProcess(PROCESS_ALL_ACCESS, false, winlogon_pid).unwrap();
        info!("OpenProcess: process: {:?}", process);

        if process.is_invalid() {
            break;
        }

        let ret = OpenProcessToken(
            process,
            TOKEN_ADJUST_PRIVILEGES
                | TOKEN_QUERY
                | TOKEN_DUPLICATE
                | TOKEN_ASSIGN_PRIMARY
                | TOKEN_ADJUST_SESSIONID
                | TOKEN_READ
                | TOKEN_WRITE,
            &mut token,
        );
        info!(
            "OpenProcessToken: token_handle: {:?}, ret: {:?}",
            token, ret
        );

        if !ret.as_bool() {
            error_code = GetLastError();
            info!("OpenProcessToken: error_code: {:?}", error_code);
            break;
        }

        let ret = DuplicateTokenEx(
            token,
            TOKEN_ALL_ACCESS,
            None,
            SecurityIdentification,
            TokenPrimary,
            &mut user_token_dup,
        );
        info!(
            "DuplicateTokenEx: user_token_dup: {:?}, ret: {:?}",
            user_token_dup, ret
        );

        if !ret.as_bool() {
            error_code = GetLastError();
            break;
        }

        let session_id = WTSGetActiveConsoleSessionId();
        info!("WTSGetActiveConsoleSessionId: {:?}", session_id);

        let ret = SetTokenInformation(
            user_token_dup,
            TokenSessionId,
            &session_id as *const u32 as *const c_void,
            size_of::<u32>() as u32,
        );
        info!("SetTokenInformation: ret: {:?}", ret);

        // TODO: Check Result
        let ret = CreateEnvironmentBlock(&mut environment, user_token_dup, true);
        info!("CreateEnvironmentBlock: ret: {:?}", ret);

        error_code = GetLastError();

        if ret.as_bool() {
            creation_flags |= CREATE_UNICODE_ENVIRONMENT;
        } else {
            environment = zeroed();
        }

        let proc_name16 = U16CString::from_str(proc_name)?;
        let proc_dir16 = U16CString::from_str(proc_dir)?;
        let mut proc_cmd16 = U16CString::from_str(proc_cmd)?;

        let ret = CreateProcessAsUserW(
            user_token_dup,
            PCWSTR::from_raw(proc_name16.as_ptr()),
            PWSTR::from_raw(proc_cmd16.as_mut_ptr()),
            None,
            None,
            false,
            creation_flags.0,
            Some(environment),
            PCWSTR::from_raw(proc_dir16.as_ptr()),
            &mut si,
            &mut pi,
        );

        info!("CreateProcessAsUserW: ret: {:?}", ret);
        info!("pi.dwProcessId: {:?}", pi.dwProcessId);
        info!("pi.dwThreadId: {:?}", pi.dwThreadId);
        info!("pi.hProcess: {:?}", pi.hProcess);
        info!("pi.hThread: {:?}", pi.hThread);

        break;
    }

    if error_code != NO_ERROR {
        info!("ErrorCode: {:?}", error_code);
    }

    if !environment.is_null() {
        DestroyEnvironmentBlock(environment);
    }

    if !user_token_dup.is_invalid() {
        CloseHandle(user_token_dup);
    }

    if !token.is_invalid() {
        CloseHandle(token);
    }

    if !process.is_invalid() {
        CloseHandle(process);
    }

    Ok(pi.dwProcessId)
}

// use winapi::shared::ntdef::LPWSTR;

// STRUCT! {
// struct WTS_SESSION_INFOW {
//     SessionId: u32,
//     pWinStationName: LPWSTR,
//     State: WTS_CONNECTSTATE_CLASS,
// }}

// #[allow(non_snake_case)]
// type WTS_CONNECTSTATE_CLASS = u32;

// const WTSActive: WTS_CONNECTSTATE_CLASS = 0;
// const WTSConnected: WTS_CONNECTSTATE_CLASS = 1;
// const WTSConnectQuery: WTS_CONNECTSTATE_CLASS = 2;
// const WTSShadow: WTS_CONNECTSTATE_CLASS = 3;
// const WTSDisconnected: WTS_CONNECTSTATE_CLASS = 4;
// const WTSIdle: WTS_CONNECTSTATE_CLASS = 5;
// const WTSListen: WTS_CONNECTSTATE_CLASS = 6;
// const WTSReset: WTS_CONNECTSTATE_CLASS = 7;
// const WTSDown: WTS_CONNECTSTATE_CLASS = 8;
// const WTSInit: WTS_CONNECTSTATE_CLASS = 9;

// #[windows_dll::dll(Wtsapi32)]
// extern "system" {
//     #[allow(non_snake_case)]
//     pub fn WTSEnumerateSessionsW(
//         hServer: HANDLE,
//         Reserved: u32,
//         Version: u32,
//         ppSessionInfo: *mut *mut WTS_SESSION_INFOW,
//         pCount: *mut u32,
//     ) -> u32;

//     #[allow(non_snake_case)]
//     pub fn WTSFreeMemory(pMemory: PVOID);

//     #[allow(non_snake_case)]
//     pub fn WTSQueryUserToken(SessionId: ULONG, phToken: PHANDLE) -> BOOL;
// }

#[test]
fn test_get_current_user_token() {
    log::set_max_level(log::LevelFilter::Trace);
    let token = unsafe { get_current_user_token().unwrap() };
    info!("get_current_user_token: {:?}", token);
}

unsafe fn get_current_user_token() -> Result<HANDLE> {
    const WTS_CURRENT_SERVER_HANDLE: HANDLE = HANDLE(0);

    let mut session_info: *mut WTS_SESSION_INFOW = zeroed();
    let mut count: u32 = 0;

    WTSEnumerateSessionsW(
        WTS_CURRENT_SERVER_HANDLE,
        0,
        1,
        &mut session_info,
        &mut count,
    );

    let mut session_id = 0;
    for i in 0..count {
        let si = session_info.add(i as usize);
        if WTSActive == (*si).State {
            session_id = (*si).SessionId;
            break;
        }
    }
    WTSFreeMemory(session_info as *mut c_void);

    let mut current_token: HANDLE = HANDLE::default();
    let ret = WTSQueryUserToken(session_id, &mut current_token);

    if !ret.as_bool() {
        let error_code = GetLastError();
        bail!("WTSQueryUserToken  error_code: {:?}", error_code);
    }

    let mut primary_token: HANDLE = HANDLE::default();
    let ret = DuplicateTokenEx(
        current_token,
        TOKEN_ASSIGN_PRIMARY | TOKEN_ALL_ACCESS,
        None,
        SecurityImpersonation,
        TokenPrimary,
        &mut primary_token,
    );
    let error_code = GetLastError();

    CloseHandle(current_token);

    if !ret.as_bool() {
        bail!("DuplicateTokenEx error_code: {:?}", error_code);
    }

    Ok(primary_token)
}

pub fn run_as_admin(proc_dir: &str, proc_cmd: &str) -> Result<u32> {
    unsafe { run_admin_privilege_unsafe(proc_dir, proc_cmd) }
}

pub unsafe fn run_admin_privilege_unsafe(proc_dir: &str, proc_cmd: &str) -> Result<u32> {
    let primary_token = get_current_user_token().unwrap_or(HANDLE::default());

    let mut unfiltered_token: HANDLE = HANDLE::default();
    let mut size: u32 = 0;

    let ret = GetTokenInformation(
        primary_token,
        TokenLinkedToken,
        Some(&mut unfiltered_token as *const HANDLE as *mut c_void),
        size_of::<HANDLE>() as u32,
        &mut size,
    );

    let mut environment: *mut c_void = zeroed();
    let ret = CreateEnvironmentBlock(&mut environment, unfiltered_token, false);
    info!("CreateEnvironmentBlock: ret: {:?}", ret);

    let mut si: STARTUPINFOW = zeroed();
    let mut pi: PROCESS_INFORMATION = zeroed();

    // let proc_name16 = U16CString::from_str(proc_name)?;
    let proc_dir16 = U16CString::from_str(proc_dir)?;
    let mut proc_cmd16 = U16CString::from_str(proc_cmd)?;

    let creation_flags =
        CREATE_NEW_CONSOLE | NORMAL_PRIORITY_CLASS | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW; //

    let ret = CreateProcessAsUserW(
        unfiltered_token,
        None,
        PWSTR::from_raw(proc_cmd16.as_mut_ptr()),
        None,
        None,
        false,
        creation_flags.0,
        Some(environment),
        PCWSTR::from_raw(proc_dir16.as_ptr()),
        &mut si,
        &mut pi,
    );
    info!("CreateProcessAsUserW: ret: {:?}", ret);

    if !environment.is_null() {
        DestroyEnvironmentBlock(environment);
    }

    if !primary_token.is_invalid() {
        CloseHandle(primary_token);
    }

    Ok(pi.dwProcessId)
}

#[test]
fn test_eink() {
    eink_logger::init_with_level(Level::Trace).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    let pid = unsafe { get_process_pid_unsafe("lsass.exe").unwrap() };
    info!("PID: {}", pid);

    unsafe { run_system_privilege_unsafe("a", "", "") };
}

#[test]
fn test_kill_process_by_name() {
    eink_logger::init_with_level(Level::Trace).unwrap();
    info!("TaskBar.exe");
    kill_process_by_name("TaskBar.exe", 0);
}

/// 根据 PID 杀进程
pub fn kill_process_by_pid(pid: u32, exit_code: u32) -> bool {
    unsafe {
        let hprocess = OpenProcess(PROCESS_ALL_ACCESS, false, pid).unwrap();
        kill_process(hprocess, exit_code)
    }
}

/// 根据 HANDLE 杀进程
pub fn kill_process(hprocess: HANDLE, exit_code: u32) -> bool {
    unsafe { TerminateProcess(hprocess, exit_code).as_bool() }
}

/// 根据 NAME 杀进程
pub fn kill_process_by_name(name: &str, exit_code: u32) -> bool {
    unsafe {
        let pid = get_process_id_by_name(name).unwrap();
        let hprocess = OpenProcess(PROCESS_TERMINATE, false, pid).unwrap();
        kill_process(hprocess, exit_code)
    }
}

pub fn get_process_id_by_name(name: &str) -> anyhow::Result<u32> {
    unsafe {
        let name16 = widestring::U16CString::from_str(name)?;
        info!("name16: {:?}", name16);

        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;

        if snapshot.is_invalid() {
            anyhow::bail!("XXX");
        }

        let mut pinfo: PROCESSENTRY32W = MaybeUninit::uninit().assume_init();
        let mut shfile_info: SHFILEINFOW = MaybeUninit::uninit().assume_init();

        // let mut pid = 0;

        pinfo.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        let mut status: BOOL = Process32FirstW(snapshot, &mut pinfo);

        info!("pinfo.szExeFile: {:?}", pinfo.szExeFile);
        info!("name16: {:?}", name16);

        while status.as_bool() {
            SHGetFileInfoW(
                PCWSTR::from_raw(pinfo.szExeFile.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut shfile_info),
                size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_SMALLICON,
            );

            info!("pinfo.szExeFile: {:?}", pinfo.szExeFile);
            info!("name16: {:?}", name16);

            if StrStrW(
                PCWSTR::from_raw(pinfo.szExeFile.as_ptr()),
                PCWSTR::from_raw(name16.as_ptr()),
            ) != PWSTR::null()
            {
                return Ok(pinfo.th32ProcessID);
            }

            status = Process32NextW(snapshot, &mut pinfo);
        }
    }
    anyhow::bail!("XXX");
}

pub fn reparent_window_to_desktop(hwnd: HWND) {
    let desktop_hwnd = unsafe { GetDesktopWindow() };
}

/// 查找窗口
pub fn find_window_by_title<P>(name: P) -> anyhow::Result<HWND>
where
    P: Into<PCSTR>,
{
    let hwnd = unsafe { FindWindowA(None, name) };
    if hwnd == HWND(0) {
        bail!("Cannot find window");
    } else {
        Ok(hwnd)
    }
}

/// 查找窗口
pub fn find_window_by_classname<P>(name: P) -> anyhow::Result<HWND>
where
    P: Into<PCSTR>,
{
    let hwnd = unsafe { FindWindowA(name, None) };
    if hwnd == HWND(0) {
        bail!("Cannot find window");
    } else {
        Ok(hwnd)
    }
}

/// 设置窗口最大化
pub fn set_window_maximize(hwnd: HWND) {
    if unsafe { ShowWindow(hwnd, SW_MAXIMIZE).as_bool() } {
        // ignore
    } else {
        log::error!("Cannot maximize window {hwnd:?}");
    }
}

// 设置窗口隐藏
pub fn set_window_hidden(hwnd: HWND) {
    if unsafe { ShowWindow(hwnd, SW_HIDE).as_bool() } {
        // ignore
    } else {
        log::error!("Cannot hide window {hwnd:?}");
    }
}

/// 设置窗口显示
pub fn set_window_shown(hwnd: HWND) {
    if unsafe { ShowWindow(hwnd, SW_SHOW).as_bool() } {
        // ignore
    } else {
        log::error!("Cannot hide window {hwnd:?}");
    }
}

/// 设置窗口最小化
pub fn set_window_minimize(hwnd: HWND) {
    if unsafe { ShowWindow(hwnd, SW_MINIMIZE).as_bool() } {
        // ignore
    } else {
        log::error!("Cannot hide launcher window");
    }
}

/// 查找当前前端窗口
pub fn get_foreground_window() -> anyhow::Result<HWND> {
    // Retrieves a handle to the foreground window
    // The return value is a handle to the foreground window.
    // The foreground window can be NULL in certain circumstances,
    // such as when a window is losing activation.
    let hwnd = unsafe { GetForegroundWindow() };

    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) };
    if style & (WS_DISABLED.0 as i32) == 1 {
        bail!("Foreground window is WS_DISABLED");
    }

    // No tooltips
    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
    if ex_style & (WS_EX_TOOLWINDOW.0 as i32) == 1 {
        bail!("Foreground window is WS_EX_TOOLWINDOW");
    }

    let ancestor = unsafe { GetAncestor(hwnd, GA_ROOT) };
    if ancestor != hwnd {
        let mut ancestor_class: [u8; 256] = unsafe { std::mem::zeroed() };
        unsafe { GetClassNameA(ancestor, &mut ancestor_class) };

        let class_name = unsafe { CStr::from_ptr(ancestor_class.as_ptr() as *const _) };

        // println!("class_name.to_bytes(): {:?}", class_name.to_bytes());

        if class_name.to_bytes() != "ApplicationFrameWindow".as_bytes() {
            bail!("Ancestor classname is not ApplicationFrameWindow");
        }
    }

    let mut class_name = [0u16; 512];
    unsafe { GetClassNameW(hwnd, &mut class_name) };
    let mut class_name = String::from_utf16_lossy(&class_name);

    // Truncate to first null char
    if let Some(index) = class_name.find('\0') {
        class_name.truncate(index);
    }

    // Check to see if the self is cloaked if it's a UWP
    if class_name == "Windows.UI.Core.CoreWindow" || class_name == "ApplicationFrameWindow" {
        let mut cloaked: u32 = 0;
        let ret = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut _ as *mut _,
                std::mem::size_of::<u32>() as u32,
            )
        };
        if ret.is_ok() && cloaked == DWM_CLOAKED_SHELL {
            bail!("UWP window is not DWM_CLOAKED_SHELL");
        }
    }

    Ok(hwnd)
}

#[cfg(test)]
mod test {
    use eink_winkits::get_window_text;

    use crate::win_utils::get_foreground_window;

    #[test]
    fn test_get_foreground_window() {
        if let Ok(hwnd) = get_foreground_window() {
            println!("get_foreground_window: {:?}", hwnd);
            let title = get_window_text(hwnd);
            println!("window title: {:?}", title);
        }
    }
}
