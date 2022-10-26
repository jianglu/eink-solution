use std::mem::{size_of, MaybeUninit};

use anyhow::Result;
use log::info;

use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{BOOL, HANDLE},
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32,
                PROCESSENTRY32W, TH32CS_SNAPPROCESS,
            },
            Threading::{OpenProcess, TerminateProcess, PROCESS_ALL_ACCESS, PROCESS_TERMINATE},
        },
        UI::Shell::{SHGetFileInfoW, StrStrW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON},
    },
};

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

fn main() -> Result<()> {
    // 设置当前的活动日志系统为 OutputDebugString 输出
    eink_logger::init_with_level(log::Level::Trace);

    kill_process_by_name("TabTip.exe", 0);

    Ok(())
}
