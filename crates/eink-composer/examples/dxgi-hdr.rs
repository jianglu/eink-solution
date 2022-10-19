use windows::Devices::Display::Core::{DisplayManager, DisplayManagerOptions};


fn main() -> anyhow::Result<()> {
    let manager = DisplayManager::Create(DisplayManagerOptions::None)?;
    manager.
    Ok(())
}

// monitor_set_specliazed
// monitor_set_hdr HHHH ture

// monitor_cli specialize monitor_id false
// monitor_cli hdr monitor_id false
