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

use anyhow::{bail, Result};

use windows::Devices::Display::Core::DisplayDevice;
use winrt_ext::DisplayPathExt;

use crate::renderer::D3D11Renderer;
use crate::winrt::*;

// mod d3d;
// mod d3d11;
// mod d3dcompiler;
mod duplicator;
// mod dxgi;
mod frame_counter;
mod iterable;
// mod layer;
mod renderer;
mod specialized;
mod utility;
mod winrt;
mod winrt_ext;

mod shader;

const SURFACE_COUNT: usize = 2;

fn check_hresult(hr: HRESULT) -> Result<()> {
    Ok(())
}

fn main() -> Result<()> {
    // Create a DisplayManager instance for owning targets and managing the displays
    let manager = DisplayManager::Create(DisplayManagerOptions::None)?;

    let mut found_targets = Vec::<DisplayTarget>::default();

    let display_targets = manager.GetCurrentTargets()?;
    for display_target in display_targets {
        if let Ok(_display_monitor) = display_target.TryGetMonitor() {
            let stable_monitor_id = display_target.StableMonitorId()?;
            if stable_monitor_id == "GBR01560_21_07E3_EF" {
                found_targets.push(display_target);
                break;
            }
        }
    }

    // Create a state object for setting modes on the targets
    let iterable_targets: IIterable<DisplayTarget> =
        iterable::Iterable(found_targets.clone()).into();
    let state_result =
        manager.TryAcquireTargetsAndCreateEmptyState(InParam::owned(iterable_targets.clone()))?;
    check_hresult(state_result.ExtendedErrorCode()?)?;

    let state = state_result.State()?;

    for target in found_targets.iter() {
        let path = state.ConnectTarget(target)?;

        // Set some values that we know we want
        path.set_is_interlaced(false)?;
        path.SetScaling(DisplayPathScaling::Identity)?;

        // We only look at BGRA 8888 modes in this example
        path.SetSourcePixelFormat(DirectXPixelFormat::B8G8R8A8UIntNormalized)?;

        // Get a list of modes for only the preferred resolution
        let modes = path.FindModes(DisplayModeQueryOptions::OnlyPreferredResolution)?;

        // Find e.g. the mode with a refresh rate closest to 60 Hz
        let mut best_mode: Option<DisplayModeInfo> = None;
        let mut best_mode_diff = f32::MAX;

        for mode in modes {
            let v_sync = mode.PresentationRate()?.VerticalSyncRate;
            let v_sync_double = v_sync.Numerator as f32 / v_sync.Denominator as f32;

            let mode_diff = f32::abs(v_sync_double - 60f32);
            if mode_diff < best_mode_diff {
                best_mode = Some(mode);
                best_mode_diff = mode_diff;
            }
        }

        if best_mode.is_none() {
            // Failed to find a mode
            bail!("Failed to find a valid mode");
        }

        // Set the properties on the path
        let best_mode = best_mode.unwrap();
        path.ApplyPropertiesFromMode(InParam::owned(best_mode))?;
    }

    // Now that we've decided on modes to use for all of the targets, apply all the modes in one-shot
    let apply_result = state.TryApply(DisplayStateApplyOptions::None)?;
    check_hresult(apply_result.ExtendedErrorCode()?)?;

    // Re-read the current state to see the final state that was applied (with all properties)
    let state_result =
        manager.TryAcquireTargetsAndReadCurrentState(InParam::owned(iterable_targets))?;
    check_hresult(state_result.ExtendedErrorCode()?)?;

    let state = state_result.State()?;

    let target = found_targets.first().unwrap();
    let adapter = target.Adapter()?;
    let device = manager.CreateDisplayDevice(InParam::owned(adapter))?;
    let path = state.GetPathForTarget(target)?;

    render(device, target, path)?;

    Ok(())
}

fn render(device: DisplayDevice, target: &DisplayTarget, path: DisplayPath) -> Result<()> {
    let mut renderer = D3D11Renderer::new(target.Adapter()?)?;

    // Create a display source, which identifies where to render
    let source = device.CreateScanoutSource(target)?;

    // Create a task pool for queueing presents
    let task_pool = device.CreateTaskPool()?;

    let source_resolution = path.SourceResolution()?.Value()?;

    let mut multisample_desc = Direct3DMultisampleDescription::default();
    multisample_desc.Count = 1;

    // Create a surface format description for the primaries
    // 创建主表面描述符
    let primary_desc = DisplayPrimaryDescription::CreateWithProperties(
        None,
        source_resolution.Width as u32,
        source_resolution.Height as u32,
        path.SourcePixelFormat()?,
        DirectXColorSpace::RgbFullG22NoneP709,
        false,
        multisample_desc,
    )
    .unwrap();
    println!("DisplayPrimaryDescription: {:?}", primary_desc);

    // std::array<winrt::DisplaySurface, SurfaceCount> primaries = { nullptr, nullptr };
    // std::array<winrt::DisplayScanout, SurfaceCount> scanouts = { nullptr, nullptr };

    let mut primaries = Vec::<DisplaySurface>::new();
    let mut scanouts = Vec::<DisplayScanout>::new();

    for _surface_index in 0..SURFACE_COUNT {
        let primary_surface = device.CreatePrimary(target, &primary_desc)?;
        let scanout = device.CreateSimpleScanout(&source, &primary_surface, 0, 1)?;
        primaries.push(primary_surface);
        scanouts.push(scanout);
    }

    renderer.open_surfaces(&device, primaries)?;

    renderer.init_layers()?;

    // Get a fence to wait for render work to complete
    let fence = renderer.get_fence(&device)?;

    // // Render and present until termination is signalled
    let mut surface_index = 0usize;

    let mut last_frame_rate = 0;
    let mut frame_counter = frame_counter::FrameCounter::new(60f64);

    loop {
        let fence_value = renderer.render_and_get_fence_value(surface_index);

        let task = task_pool.CreateTask()?;
        task.SetScanout(&scanouts[surface_index])?;
        task.SetWait(&fence, fence_value)?;

        task_pool.TryExecuteTask(&task)?;

        device.WaitForVBlank(&source)?;

        surface_index = surface_index + 1;
        if surface_index >= SURFACE_COUNT {
            surface_index = 0;
        }

        frame_counter.tick();

        let frame_rate = frame_counter.frame_rate_round();

        if last_frame_rate != frame_rate {
            last_frame_rate = frame_rate;
            println!("fps: {}", frame_rate);
        }
    }
}
