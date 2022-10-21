use windows::{
    Devices::Display::Core::{DisplayManager, DisplayManagerOptions},
    Win32::Graphics::Dxgi::{CreateDXGIFactory2, IDXGIFactory6},
};

fn main() -> anyhow::Result<()> {
    let manager = DisplayManager::Create(DisplayManagerOptions::None)?;

    let factory: IDXGIFactory6 = unsafe { CreateDXGIFactory2(0).unwrap() };

    Ok(())
}
