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

use log::info;
use std::mem::size_of;
use std::mem::zeroed;
use widestring::U16CStr;
use widestring::U16CString;
use winapi::shared::minwindef::FALSE;
use winapi::shared::ntdef::HANDLE;
use winapi::shared::ntdef::NULL;
use winapi::shared::ntdef::PHANDLE;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::processthreadsapi::OpenProcessToken;
use winapi::um::processthreadsapi::STARTUPINFOW;
use winapi::um::securitybaseapi::DuplicateTokenEx;
use winapi::um::winbase::CREATE_NEW_CONSOLE;
use winapi::um::winbase::NORMAL_PRIORITY_CLASS;
use winapi::um::winnt::MAXIMUM_ALLOWED;
use winapi::um::winnt::TOKEN_ADJUST_PRIVILEGES;
use winapi::um::winnt::TOKEN_ADJUST_SESSIONID;
use winapi::um::winnt::TOKEN_ASSIGN_PRIMARY;
use winapi::um::winnt::TOKEN_DUPLICATE;
use winapi::um::winnt::TOKEN_QUERY;
use winapi::um::winnt::TOKEN_READ;
use winapi::um::winnt::TOKEN_WRITE;
use windows::w;

use anyhow::bail;
use anyhow::Result;
use winapi::um::handleapi::CloseHandle;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::tlhelp32::CreateToolhelp32Snapshot;
use winapi::um::tlhelp32::Process32FirstW;
use winapi::um::tlhelp32::Process32NextW;
use winapi::um::tlhelp32::PROCESSENTRY32W;
use winapi::um::tlhelp32::TH32CS_SNAPPROCESS;

/// 通过进程名获取进程 PID
pub fn get_process_pid(name: &str) -> Result<u32> {
    unsafe { get_process_pid_unsafe(name) }
}

/// 通过进程名获取进程 PID
pub unsafe fn get_process_pid_unsafe(name: &str) -> Result<u32> {
    let process_snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);

    if process_snap == INVALID_HANDLE_VALUE {
        bail!("CreateToolhelp32Snapshot Failed")
    }

    let mut pe32: PROCESSENTRY32W = zeroed();
    pe32.dwSize = size_of::<PROCESSENTRY32W>() as u32;

    let mut ret = Process32FirstW(process_snap, &mut pe32);

    // 创建临时宽字符
    let name16 = U16CString::from_str(name)?;

    let mut pid = 0;

    while ret != 0 {
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

pub fn run_system_privilege(proc_name: &str, proc_dir: &str, proc_cmd: &str) -> Result<()> {
    unsafe { run_system_privilege_unsafe(proc_name, proc_dir, proc_cmd) }
}

pub unsafe fn run_system_privilege_unsafe(
    proc_name: &str,
    proc_dir: &str,
    proc_cmd: &str,
) -> Result<()> {
    use winapi::shared::minwindef::DWORD;
    use winapi::shared::minwindef::LPVOID;
    use winapi::um::minwinbase::LPSECURITY_ATTRIBUTES;
    use winapi::um::processthreadsapi::CreateProcessAsUserW;
    use winapi::um::processthreadsapi::PROCESS_INFORMATION;
    use winapi::um::securitybaseapi::SetTokenInformation;
    use winapi::um::userenv::DestroyEnvironmentBlock;
    use winapi::um::winbase::WTSGetActiveConsoleSessionId;
    use winapi::um::winnt::SecurityIdentification;
    use winapi::um::winnt::TokenPrimary;
    use winapi::um::winnt::TokenSessionId;

    let mut creation_flags = NORMAL_PRIORITY_CLASS | CREATE_NEW_CONSOLE;
    let winlogon_pid = get_process_pid("winlogon.exe")?;

    let mut process: HANDLE = NULL;
    let mut token: HANDLE = NULL;
    let mut environment: LPVOID = NULL;
    let mut user_token_dup: HANDLE = NULL;

    let mut error_code = 0;

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

        let pi: PROCESS_INFORMATION = zeroed();

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

    Ok(())
}

#[test]
fn test_eink() {
    crate::logger::init();
    log::set_max_level(log::LevelFilter::Trace);
    let pid = unsafe { get_process_pid_unsafe("lsass.exe").unwrap() };
    info!("PID: {}", pid);

    unsafe { run_system_privilege_unsafe("a") };
}
