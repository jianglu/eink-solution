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

use serde_json::json;
use structopt::StructOpt;

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
        Subcommand::SetWindowTopmost { hwnd } => todo!(),
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
    }
}
