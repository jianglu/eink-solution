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

fn main() {}
