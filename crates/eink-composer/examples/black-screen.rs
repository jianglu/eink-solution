use windows::Devices::Display::Core::{DisplayManager, DisplayManagerOptions};


fn main() -> anyhow::Result<()> {
    let manager = DisplayManager::Create(DisplayManagerOptions::None)?;
    Ok(())
}
