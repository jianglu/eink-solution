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

use std::ops::Sub;

use anyhow::bail;
use serde_json::json;
use structopt::StructOpt;
use windows::{
    core::PCSTR,
    s,
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            FindWindowA, GetForegroundWindow, SendMessageA, WM_HOTKEY, WM_USER,
        },
    },
};

#[derive(structopt::StructOpt, Clone, Debug, PartialEq)]
enum Subcommand {
    #[structopt(about = "Set window topmost")]
    SetWindowTopmost {
        /// Window Handle
        #[structopt(long)]
        hwnd: u64,
    },
    #[structopt(about = "Hide taskbar")]
    HideTaskbar,
    #[structopt(about = "Eink set mipi mode")]
    EinkSetMipiMode {
        #[structopt(long)]
        mode: u32,
    },
    #[structopt(about = "Eink refresh")]
    EinkRefresh,
    #[structopt(about = "Disable alt-tab / win key")]
    DisableWinKey,
    #[structopt(about = "Enable alt-tab / win key")]
    EnableWinKey,
    #[structopt(about = "Test")]
    Test,
}

#[derive(structopt::StructOpt, Clone, Debug, PartialEq)]
#[structopt(
    name = "runner",
    about = "Wrap arbitrary commands as Windows services",
    set_term_width = 80,
    setting(structopt::clap::AppSettings::SubcommandsNegateReqs)
)]
struct Cli {
    #[structopt(subcommand)]
    sub: Subcommand,
}

const TCON_PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\tcon";

const KEYBOARD_PIPE_NAME: &str = r"\\.\pipe\lenovo\eink-service\keyboard";

fn main() {
    let cli = Cli::from_args();
    match cli.sub {
        Subcommand::SetWindowTopmost { hwnd } => {
            let hwnd = unsafe { GetForegroundWindow() };
            if let Ok(api_hwnd) = find_window_by_classname(s!("AlwaysOnTopWindow")) {
                log::info!("Send Topmost Message To AlwaysOnTopWindow");
                unsafe {
                    SendMessageA(api_hwnd, WM_USER, WPARAM::default(), LPARAM(hwnd.0));
                }
            }
        }
        Subcommand::HideTaskbar => todo!(),
        Subcommand::EinkSetMipiMode { mode } => {
            println!("EinkSetMipiMode mode: {mode}");
            let mut client = eink_pipe_io::blocking::connect(TCON_PIPE_NAME)
                .expect("Cannot connect to tcon service");
            let reply = client
                .call_with_params("set_mipi_mode", json!({ "mode": mode }))
                .expect("Cannot invoke remote method to tcon service");
            println!("reply: {reply:?}");
        }
        Subcommand::EinkRefresh => {
            println!("EinkRefresh");
            let mut client = eink_pipe_io::blocking::connect(TCON_PIPE_NAME)
                .expect("Cannot connect to tcon service");
            let reply = client
                .call_with_params("refresh", json!({}))
                .expect("Cannot invoke remote method to tcon service");
            println!("reply: {reply:?}");
        }

        Subcommand::DisableWinKey => {
            println!("DisableWinKey");
            let mut client = eink_pipe_io::blocking::connect(KEYBOARD_PIPE_NAME)
                .expect("Cannot connect to keyboard service");
            let reply = client
                .call_with_params("disable_win_key", json!({}))
                .expect("Cannot invoke remote method to tcon service");
            println!("reply: {reply:?}");
        }

        Subcommand::EnableWinKey => {
            println!("EnableWinKey");
            let mut client = eink_pipe_io::blocking::connect(KEYBOARD_PIPE_NAME)
                .expect("Cannot connect to keyboard service");
            let reply = client
                .call_with_params("enable_win_key", json!({}))
                .expect("Cannot invoke remote method to tcon service");
            println!("reply: {reply:?}");
        }

        Subcommand::Test => unsafe {
            // #[windows_dll::dll(User32)]
            // extern "system" {
            //     #[allow(non_snake_case)]
            //     pub fn GetWindowBand(hwnd: HWND, band: &mut u32) -> bool;
            // }

            std::thread::sleep(std::time::Duration::from_millis(2000));

            let foreground_hwnd = GetForegroundWindow();
            // let mut band: u32 = 0;
            // GetWindowBand(foreground_hwnd, &mut band);
            // println!("Foreground Band: {band}");

            if let Ok(api_hwnd) = find_window_by_classname(s!("AlwaysOnTopWindow")) {
                log::error!("Send Topmost Message To AlwaysOnTopWindow");
                SendMessageA(
                    api_hwnd,
                    WM_USER,
                    WPARAM::default(),
                    LPARAM(foreground_hwnd.0),
                );
            }
        },
    }
}

/// 查找窗口
fn find_window_by_classname<P>(name: P) -> anyhow::Result<HWND>
where
    P: Into<PCSTR>,
{
    match unsafe { FindWindowA(name, None) } {
        HWND(0) => {
            bail!("Cannot find window");
        }
        HWND(hwnd) => Ok(HWND(hwnd)),
    }
}
