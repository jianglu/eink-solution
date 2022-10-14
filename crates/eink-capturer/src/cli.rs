use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(long)]
    command_line: Option<String>,

    /// Capture a window who's title contains the provided input.
    #[clap(
        long,
        // conflicts_with = "monitor",
        // conflicts_with = "window_title",
        // conflicts_with = "primary"
    )]
    window_id: Option<isize>,

    /// Capture a window who's title contains the provided input.
    #[clap(
        long,
        // conflicts_with = "monitor",
        // conflicts_with = "window_id",
        // conflicts_with = "primary"
    )]
    window_title: Option<String>,

    /// The index of the monitor to screenshot.
    #[clap(
        short,
        long,
        // conflicts_with = "window_id",
        // conflicts_with = "window_title",
        // conflicts_with = "primary"
    )]
    monitor: Option<usize>,

    // Capture the primary monitor (default if no params are specified).
    #[clap(
        short,
        long,
        // conflicts_with = "window_id",
        // conflicts_with = "window_title",
        // conflicts_with = "monitor"
    )]
    primary: bool,

    /// The target position band
    #[clap(long)]
    pub band: Option<i32>,

    /// The target position x
    #[clap(short)]
    pub x: Option<i32>,

    /// The target position y
    #[clap(short)]
    pub y: Option<i32>,
}

pub enum CaptureMode {
    CommandLine(String),
    WindowId(isize),
    WindowTitle(String),
    Monitor(usize),
    Primary,
}

impl CaptureMode {
    pub fn from_args(args: &Args) -> Self {
        if let Some(cmdline) = &args.command_line {
            CaptureMode::CommandLine(cmdline.clone())
        } else if let Some(window_id) = &args.window_id {
            CaptureMode::WindowId(*window_id)
        } else if let Some(window_query) = &args.window_title {
            CaptureMode::WindowTitle(window_query.clone())
        } else if let Some(index) = &args.monitor {
            CaptureMode::Monitor(index.clone())
        } else {
            CaptureMode::Primary
        }
    }
}
