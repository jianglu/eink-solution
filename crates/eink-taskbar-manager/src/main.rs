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

const HELP: &str = "\
App
USAGE:
  app [OPTIONS] --number NUMBER [INPUT]
FLAGS:
  -h, --help            Prints help information
OPTIONS:
  --number NUMBER       Sets a number
  --opt-number NUMBER   Sets an optional number
  --width WIDTH         Sets width [default: 10]
  --output PATH         Sets an output path
ARGS:
  <INPUT>
";

#[derive(Debug)]
struct AppArgs {
    number: u32,
    opt_number: Option<u32>,
    width: u32,
    input: std::path::PathBuf,
    output: Option<std::path::PathBuf>,
}

fn main() -> Result<()> {
    let mut pargs = pico_args::Arguments::from_env();

    // Help has a higher priority and should be handled separately.
    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let pid: u64 = pargs.value_from_str("--pid")?,
    let settings_file: String = pargs.value_from_str("--settings-file")?,

    println!("{}, {}", pid, settings_file);
}
