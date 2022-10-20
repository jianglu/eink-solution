use log::Level;
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
#[allow(non_snake_case)]
use log::{debug, info};
use std::mem::size_of;
use std::mem::zeroed;
use std::mem::MaybeUninit;
use widestring::U16CStr;
use widestring::U16CString;
use windows::Win32::System::SystemServices::MAXIMUM_ALLOWED;
use windows::Win32::System::Threading::CREATE_NEW_CONSOLE;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;
use windows::Win32::System::Threading::NORMAL_PRIORITY_CLASS;
use windows::Win32::System::Threading::PROCESS_INFORMATION;
use windows::Win32::System::Threading::STARTUPINFOW;

// use winapi::shared::minwindef::BOOL;
// use winapi::shared::minwindef::DWORD;
// use winapi::shared::minwindef::FALSE;
// use winapi::shared::minwindef::LPVOID;
// use winapi::shared::ntdef::HANDLE;
// use winapi::shared::ntdef::NULL;
// use winapi::shared::ntdef::PHANDLE;
// use winapi::shared::ntdef::PVOID;
// use winapi::shared::ntdef::ULONG;

// use winapi::um::errhandlingapi::GetLastError;
// use winapi::um::minwinbase::LPSECURITY_ATTRIBUTES;
// use winapi::um::processthreadsapi::CreateProcessAsUserW;
// use winapi::um::processthreadsapi::OpenProcess;
// use winapi::um::processthreadsapi::OpenProcessToken;
// use winapi::um::processthreadsapi::PROCESS_INFORMATION;
// use winapi::um::processthreadsapi::STARTUPINFOW;
// use winapi::um::securitybaseapi::DuplicateTokenEx;
// use winapi::um::securitybaseapi::GetTokenInformation;
// use winapi::um::securitybaseapi::SetTokenInformation;
// use winapi::um::tlhelp32::Process32First;
// use winapi::um::tlhelp32::PROCESSENTRY32;
// use winapi::um::userenv::DestroyEnvironmentBlock;
// use winapi::um::winbase::WTSGetActiveConsoleSessionId;
// use winapi::um::winbase::CREATE_NEW_CONSOLE;
// use winapi::um::winbase::CREATE_NO_WINDOW;
// use winapi::um::winbase::NORMAL_PRIORITY_CLASS;
// use winapi::um::winnt::SecurityIdentification;
// use winapi::um::winnt::TokenLinkedToken;
// use winapi::um::winnt::TokenPrimary;
// use winapi::um::winnt::TokenSessionId;
// use winapi::um::winnt::MAXIMUM_ALLOWED;
// use winapi::um::winnt::PROCESS_ALL_ACCESS;
// use winapi::um::winnt::TOKEN_ADJUST_PRIVILEGES;
// use winapi::um::winnt::TOKEN_ADJUST_SESSIONID;
// use winapi::um::winnt::TOKEN_ASSIGN_PRIMARY;
// use winapi::um::winnt::TOKEN_DUPLICATE;
// use winapi::um::winnt::TOKEN_QUERY;
// use winapi::um::winnt::TOKEN_READ;
// use winapi::um::winnt::TOKEN_WRITE;

use windows::core::PCWSTR;
use windows::core::PWSTR;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::{BOOL, HANDLE};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::Diagnostics::ToolHelp::CreateToolhelp32Snapshot;
use windows::Win32::System::Diagnostics::ToolHelp::PROCESSENTRY32W;
use windows::Win32::System::Diagnostics::ToolHelp::TH32CS_SNAPPROCESS;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, TerminateProcess, PROCESS_ALL_ACCESS, PROCESS_TERMINATE,
};
use windows::Win32::UI::Shell::SHAppBarMessage;
use windows::Win32::UI::Shell::SHGetFileInfoW;
use windows::Win32::UI::Shell::StrStrW;
use windows::Win32::UI::Shell::ABM_SETSTATE;
use windows::Win32::UI::Shell::ABS_ALWAYSONTOP;
use windows::Win32::UI::Shell::ABS_AUTOHIDE;
use windows::Win32::UI::Shell::SHFILEINFOW;
use windows::Win32::UI::Shell::SHGFI_ICON;
use windows::Win32::UI::Shell::SHGFI_SMALLICON;
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, StrStrW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON,
};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

use winapi::ENUM;
use winapi::STRUCT;

use windows::w;

use anyhow::bail;
use anyhow::Result;

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

pub fn run_system_privilege(proc_name: &str, proc_dir: &str, proc_cmd: &str) -> Result<DWORD> {
    unsafe { run_system_privilege_unsafe(proc_name, proc_dir, proc_cmd) }
}

pub unsafe fn run_system_privilege_unsafe(
    proc_name: &str,
    proc_dir: &str,
    proc_cmd: &str,
) -> Result<DWORD> {
    let mut creation_flags = NORMAL_PRIORITY_CLASS | CREATE_NEW_CONSOLE | CREATE_NO_WINDOW;
    let winlogon_pid = get_process_pid("winlogon.exe")?;

    let mut process: HANDLE = NULL;
    let mut token: HANDLE = NULL;
    let mut environment: LPVOID = NULL;
    let mut user_token_dup: HANDLE = NULL;

    let mut error_code = 0;

    let mut pi: PROCESS_INFORMATION = zeroed();

    loop {
        if winlogon_pid == 0 {
            break;
        }

        let mut si: STARTUPINFOW = zeroed();
        si.lpDesktop = w!("winsta0\\default").as_ptr() as *mut u16;

        process = OpenProcess(MAXIMUM_ALLOWED, FALSE, winlogon_pid);
        info!("OpenProcess: process: {:?}", process);

        if process == NULL {
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
            &token as *const HANDLE as PHANDLE,
        );
        info!(
            "OpenProcessToken: token_handle: {:?}, ret: {:?}",
            token, ret
        );

        if FALSE == ret {
            error_code = GetLastError();
            info!("OpenProcessToken: error_code: {:?}", error_code);
            break;
        }

        let ret = DuplicateTokenEx(
            token,
            MAXIMUM_ALLOWED,
            NULL as LPSECURITY_ATTRIBUTES,
            SecurityIdentification,
            TokenPrimary,
            &user_token_dup as *const HANDLE as PHANDLE,
        );
        info!(
            "DuplicateTokenEx: user_token_dup: {:?}, ret: {:?}",
            user_token_dup, ret
        );

        if FALSE == ret {
            error_code = GetLastError();
            break;
        }

        let session_id = WTSGetActiveConsoleSessionId();
        info!("WTSGetActiveConsoleSessionId: {:?}", session_id);

        let ret = SetTokenInformation(
            user_token_dup,
            TokenSessionId,
            &session_id as *const u32 as LPVOID,
            size_of::<DWORD>() as u32,
        );
        info!("SetTokenInformation: ret: {:?}", ret);

        // TODO: Check Result

        use winapi::shared::minwindef::TRUE;
        use winapi::um::userenv::CreateEnvironmentBlock;
        use winapi::um::winbase::CREATE_UNICODE_ENVIRONMENT;

        let ret = CreateEnvironmentBlock(
            &environment as *const LPVOID as *mut LPVOID,
            user_token_dup,
            TRUE,
        );
        info!("CreateEnvironmentBlock: ret: {:?}", ret);

        error_code = GetLastError();

        if ret != 0 {
            creation_flags |= CREATE_UNICODE_ENVIRONMENT;
        } else {
            environment = NULL;
        }

        let proc_name16 = U16CString::from_str(proc_name)?;
        let proc_dir16 = U16CString::from_str(proc_dir)?;
        let mut proc_cmd16 = U16CString::from_str(proc_cmd)?;

        let ret = CreateProcessAsUserW(
            user_token_dup,
            proc_name16.as_ptr(),
            proc_cmd16.as_mut_ptr(),
            NULL as LPSECURITY_ATTRIBUTES,
            NULL as LPSECURITY_ATTRIBUTES,
            FALSE,
            creation_flags,
            environment,
            proc_dir16.as_ptr(),
            &si as *const STARTUPINFOW as *mut STARTUPINFOW,
            &pi as *const PROCESS_INFORMATION as *mut PROCESS_INFORMATION,
        );

        info!("CreateProcessAsUserW: ret: {:?}", ret);
        info!("pi.dwProcessId: {:?}", pi.dwProcessId);
        info!("pi.dwThreadId: {:?}", pi.dwThreadId);
        info!("pi.hProcess: {:?}", pi.hProcess);
        info!("pi.hThread: {:?}", pi.hThread);

        break;
    }

    if error_code != 0 {
        info!("ErrorCode: {:?}", error_code);
    }

    if environment != NULL {
        DestroyEnvironmentBlock(environment);
    }

    if user_token_dup != NULL {
        CloseHandle(user_token_dup);
    }

    if token != NULL {
        CloseHandle(token);
    }

    if process != NULL {
        CloseHandle(process);
    }

    Ok(pi.dwProcessId)
}

use winapi::shared::ntdef::LPWSTR;

STRUCT! {
struct WTS_SESSION_INFOW {
    SessionId: DWORD,
    pWinStationName: LPWSTR,
    State: WTS_CONNECTSTATE_CLASS,
}}

#[allow(non_snake_case)]
type WTS_CONNECTSTATE_CLASS = u32;

const WTSActive: WTS_CONNECTSTATE_CLASS = 0;
const WTSConnected: WTS_CONNECTSTATE_CLASS = 1;
const WTSConnectQuery: WTS_CONNECTSTATE_CLASS = 2;
const WTSShadow: WTS_CONNECTSTATE_CLASS = 3;
const WTSDisconnected: WTS_CONNECTSTATE_CLASS = 4;
const WTSIdle: WTS_CONNECTSTATE_CLASS = 5;
const WTSListen: WTS_CONNECTSTATE_CLASS = 6;
const WTSReset: WTS_CONNECTSTATE_CLASS = 7;
const WTSDown: WTS_CONNECTSTATE_CLASS = 8;
const WTSInit: WTS_CONNECTSTATE_CLASS = 9;

#[windows_dll::dll(Wtsapi32)]
extern "system" {
    #[allow(non_snake_case)]
    pub fn WTSEnumerateSessionsW(
        hServer: HANDLE,
        Reserved: DWORD,
        Version: DWORD,
        ppSessionInfo: *mut *mut WTS_SESSION_INFOW,
        pCount: *mut DWORD,
    ) -> u32;

    #[allow(non_snake_case)]
    pub fn WTSFreeMemory(pMemory: PVOID);

    #[allow(non_snake_case)]
    pub fn WTSQueryUserToken(SessionId: ULONG, phToken: PHANDLE) -> BOOL;
}

#[test]
fn test_get_current_user_token() {
    eink_logger::init();
    log::set_max_level(log::LevelFilter::Trace);
    let token = unsafe { get_current_user_token().unwrap() };
    info!("get_current_user_token: {:?}", token);
}

unsafe fn get_current_user_token() -> Result<HANDLE> {
    // PWTS_SESSION_INFO pSessionInfo = 0;

    const WTS_CURRENT_SERVER_HANDLE: HANDLE = NULL;

    let mut session_info: *mut WTS_SESSION_INFOW = zeroed();
    let mut count: DWORD = 0;

    WTSEnumerateSessionsW(
        WTS_CURRENT_SERVER_HANDLE,
        0,
        1,
        &mut session_info as *mut *mut WTS_SESSION_INFOW,
        &count as *const DWORD as *mut DWORD,
    );

    info!("count: {}", count);

    let mut session_id = 0;
    for i in 0..count {
        let si = session_info.add(i as usize);
        if WTSActive == (*si).State {
            session_id = (*si).SessionId;
            break;
        }
    }

    WTSFreeMemory(session_info as PVOID);

    let mut current_token: HANDLE = NULL;
    let ret = WTSQueryUserToken(session_id, &current_token as *const HANDLE as PHANDLE);
    let error_code = GetLastError();
    if ret == FALSE {
        bail!("WTSQueryUserToken  error_code: {}", error_code);
    }

    use winapi::um::winnt::SecurityImpersonation;
    use winapi::um::winnt::TOKEN_ALL_ACCESS;

    let mut primary_token: HANDLE = NULL;
    let ret = DuplicateTokenEx(
        current_token,
        TOKEN_ASSIGN_PRIMARY | TOKEN_ALL_ACCESS,
        NULL as LPSECURITY_ATTRIBUTES,
        SecurityImpersonation,
        TokenPrimary,
        &primary_token as *const HANDLE as PHANDLE,
    );
    let error_code = GetLastError();

    CloseHandle(current_token);

    if ret == FALSE {
        bail!("DuplicateTokenEx error_code: {}", error_code);
    }

    Ok(primary_token)
}

pub fn run_as_admin(proc_dir: &str, proc_cmd: &str) -> Result<DWORD> {
    unsafe { run_admin_privilege_unsafe(proc_dir, proc_cmd) }
}

pub unsafe fn run_admin_privilege_unsafe(
    // proc_name: &str,
    proc_dir: &str,
    proc_cmd: &str,
) -> Result<DWORD> {
    let primary_token = get_current_user_token().unwrap_or(NULL);

    let mut unfiltered_token: HANDLE = NULL;
    let mut size: DWORD = 0;
    let ret = GetTokenInformation(
        primary_token,
        TokenLinkedToken,
        &mut unfiltered_token as *const HANDLE as LPVOID,
        size_of::<HANDLE>() as u32,
        &mut size as *const u32 as *mut u32,
    );

    use winapi::um::userenv::CreateEnvironmentBlock;
    use winapi::um::winbase::CREATE_UNICODE_ENVIRONMENT;

    let mut environment: LPVOID = NULL;
    let ret = CreateEnvironmentBlock(
        &environment as *const LPVOID as *mut LPVOID,
        unfiltered_token,
        FALSE,
    );
    info!("CreateEnvironmentBlock: ret: {:?}", ret);

    let mut si: STARTUPINFOW = zeroed();
    let mut pi: PROCESS_INFORMATION = zeroed();

    // let proc_name16 = U16CString::from_str(proc_name)?;
    let proc_dir16 = U16CString::from_str(proc_dir)?;
    let mut proc_cmd16 = U16CString::from_str(proc_cmd)?;

    let mut creation_flags =
        CREATE_NEW_CONSOLE | NORMAL_PRIORITY_CLASS | CREATE_UNICODE_ENVIRONMENT; // CREATE_NO_WINDOW; //

    let ret = CreateProcessAsUserW(
        unfiltered_token,
        NULL as *mut u16, // proc_name16.as_ptr(),
        proc_cmd16.as_mut_ptr(),
        NULL as LPSECURITY_ATTRIBUTES,
        NULL as LPSECURITY_ATTRIBUTES,
        FALSE,
        creation_flags,
        environment,
        proc_dir16.as_ptr(),
        &si as *const STARTUPINFOW as *mut STARTUPINFOW,
        &pi as *const PROCESS_INFORMATION as *mut PROCESS_INFORMATION,
    );
    info!("CreateProcessAsUserW: ret: {:?}", ret);

    if environment != NULL {
        DestroyEnvironmentBlock(environment);
    }

    if primary_token != NULL {
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
pub fn kill_process_by_pid(pid: DWORD, exit_code: u32) -> bool {
    unsafe {
        let hprocess = OpenProcess(PROCESS_ALL_ACCESS, FALSE, pid);
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
