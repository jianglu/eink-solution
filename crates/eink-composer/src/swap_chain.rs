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

use anyhow::{bail, Ok, Result};

use log::info;

use windows::Devices::Display::Core::{DisplayFence, DisplayManagerResult, DisplayScanout};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D11::ID3D11DeviceContext4;
use windows::Win32::Graphics::Dxgi::IDXGIAdapter;

use windows::core::Interface;
use windows::{
    core::InParam,
    Devices::Display::{
        Core::{
            DisplayAdapter, DisplayDevice, DisplayManager, DisplayManagerOptions, DisplayModeInfo,
            DisplayModeQueryOptions, DisplayPathScaling, DisplaySource, DisplayState,
            DisplayStateApplyOptions, DisplaySurface, DisplayTarget, DisplayTaskPool,
        },
        DisplayMonitorUsageKind,
    },
    Foundation::Collections::IIterable,
    Graphics::SizeInt32,
    Win32::Graphics::Direct3D11::{
        ID3D11Device, ID3D11DeviceContext, ID3D11Fence, ID3D11RenderTargetView, ID3D11Texture2D,
    },
};

use crate::{
    iterable,
    specialized::set_monitor_specialized,
    winrt_ext::{
        DisplayAdapterExt, DisplayDeviceExt, DisplayManagerExt, DisplayPathExt, DisplaySurfaceExt,
        ID3D11DeviceExt, IDXGIAdapterExt,
    },
};

const SURFACE_COUNT: usize = 2;

pub struct SwapChain {
    // DisplayCore
    adapter: DisplayAdapter,
    device: DisplayDevice,
    source: DisplaySource,
    resolution: SizeInt32,
    task_pool: DisplayTaskPool,
    primaries: Vec<DisplaySurface>,
    scanouts: Vec<DisplayScanout>,
    fence: DisplayFence,

    // DXGI
    dxgi_adapter: IDXGIAdapter,

    // D3D11
    d3d_device: ID3D11Device,
    d3d_context: ID3D11DeviceContext,
    d3d_fence: ID3D11Fence,
    textures: Vec<ID3D11Texture2D>,
    render_targets: Vec<ID3D11RenderTargetView>,

    fence_value: u64,
    frame_count: u64,
    surface_index: usize,
}

impl SwapChain {
    /// .
    pub fn new(monitor_id: &str) -> Result<Self> {
        // Create a DisplayManager instance for owning targets and managing the displays
        let manager = DisplayManager::Create(DisplayManagerOptions::None)?;

        let mut target = manager.find_target_by_stable_id(monitor_id)?;

        let usage_kind = target.UsageKind()?;

        if usage_kind != DisplayMonitorUsageKind::SpecialPurpose {
            info!("usage_kind: {:?}", usage_kind);
            set_monitor_specialized(&target, true)?;

            // 再次尝试获取
            target = manager.find_target_by_stable_id(monitor_id)?;
        }

        let mut target_vec = Vec::<DisplayTarget>::with_capacity(1);
        target_vec.push(target.clone());

        // Create a state object for setting modes on the targets
        let targets_iter: IIterable<DisplayTarget> = iterable::Iterable(target_vec).into();

        info!("TryAcquireTargetsAndCreateEmptyState !");
        let result = manager
            .TryAcquireTargetsAndCreateEmptyState(&targets_iter)
            .unwrap();

        let errcode = result.ErrorCode()?;

        if errcode == DisplayManagerResult::TargetAccessDenied {
            panic!("TryAcquireTargetsAndCreateEmptyState: TargetAccessDenied");
        }

        info!("result.ErrorCode: {:?}", errcode);
        info!("result.ExtendedErrorCode: {:?}", result.ExtendedErrorCode());

        info!("result.State !");
        let state = result.State().expect("XXX");

        config_best_display_mode(&state, &target)?;

        // Now that we've decided on modes to use for all of the targets, apply all the modes in one-shot
        let _result = state.TryApply(DisplayStateApplyOptions::None)?;

        // Re-read the current state to see the final state that was applied (with all properties)
        let result = manager.TryAcquireTargetsAndReadCurrentState(&targets_iter)?;
        let state = result.State()?;

        let adapter = target.Adapter()?;
        let device = manager.CreateDisplayDevice(&adapter)?;
        let path = state.GetPathForTarget(&target)?;

        // Create a display source, which identifies where to render
        let source = device.CreateScanoutSource(&target)?;
        let resolution = path.SourceResolution()?.Value()?;
        let pixel_fmt = path.SourcePixelFormat()?;

        // Create a task pool for queueing presents
        let task_pool = device.CreateTaskPool()?;
        info!("task_pool: {:?}", task_pool);

        // Create DXGI Adapter
        let dxgi_adapter = adapter.create_dxgi_adapter()?;

        // Create D3D11 Stuff
        let (d3d_device, d3d_context) = dxgi_adapter.create_d3d11_device()?;
        info!("d3d_device: {:?}", d3d_device);

        let d3d_fence = d3d_device.create_shared_fence()?;
        let fence = device.create_shared_fence(&d3d_fence)?;
        info!("fence: {:?}", fence);

        let mut primaries = Vec::<DisplaySurface>::new();
        let mut scanouts = Vec::<DisplayScanout>::new();
        let mut textures = Vec::<ID3D11Texture2D>::new();
        let mut render_targets = Vec::<ID3D11RenderTargetView>::new();

        for _surface_index in 0..SURFACE_COUNT {
            let primary_surface = device.create_primary_surface(&target, resolution, pixel_fmt)?;
            let scanout = device.create_simple_scanout(&source, &primary_surface)?;

            let texture = primary_surface.create_shared_texture2d(&d3d_device, &device)?;
            let render_target = d3d_device.create_render_target_view(&texture)?;

            primaries.push(primary_surface);
            scanouts.push(scanout);
            textures.push(texture);
            render_targets.push(render_target);
        }

        Ok(Self {
            adapter,
            device,
            source,
            resolution,
            task_pool,
            primaries,
            scanouts,
            fence,
            dxgi_adapter,
            d3d_device,
            d3d_context,
            d3d_fence,
            textures,
            render_targets,
            fence_value: 0,
            frame_count: 0,
            surface_index: 0,
        })
    }

    pub fn get_render_target(&self) -> Result<&ID3D11RenderTargetView> {
        let rtv = &self.render_targets[self.surface_index];
        Ok(rtv)
    }

    pub fn get_device_and_context(&self) -> Result<(&ID3D11Device, &ID3D11DeviceContext)> {
        Ok((&self.d3d_device, &self.d3d_context))
    }

    pub fn get_resolution(&self) -> Result<SizeInt32> {
        Ok(self.resolution)
    }

    pub fn pre_present(&mut self, test_background: bool) -> Result<()> {
        let rtv = &self.render_targets[self.surface_index];

        // For the sample, we simply render a color pattern using a frame counter. This code is not interesting.
        unsafe {
            self.frame_count = self.frame_count + 1;

            let amount = f32::abs(f32::sin(self.frame_count as f32 / 30.0 * 3.141592));

            let frame_count_div = self.frame_count / 30;

            let mut clear_color = [0f32, 0f32, 0f32, 1f32];

            if test_background {
                clear_color[0] = amount * if frame_count_div % 3 == 0 { 1.0 } else { 0.0 };
                clear_color[1] = amount * if frame_count_div % 3 == 1 { 1.0 } else { 0.0 };
                clear_color[2] = amount * if frame_count_div % 3 == 2 { 1.0 } else { 0.0 };
            }

            self.d3d_context
                .ClearRenderTargetView(rtv, &clear_color as *const f32);
        }

        Ok(())
    }

    /// Presents a rendered image to the user.
    pub fn present(&mut self) -> Result<()> {
        // DO OTHER

        self.fence_value = self.fence_value + 1;

        // Signal D3D
        let d3d_context_4 = self.d3d_context.cast::<ID3D11DeviceContext4>()?;
        unsafe { d3d_context_4.Signal(&self.d3d_fence, self.fence_value)? };

        // DisplayTask 处理
        let task = self.task_pool.CreateTask()?;
        task.SetScanout(&self.scanouts[self.surface_index])?;
        task.SetWait(&self.fence, self.fence_value)?;
        self.task_pool.TryExecuteTask(&task)?;

        self.device.WaitForVBlank(&self.source)?;

        // swap_buffer
        self.surface_index = self.surface_index + 1;
        if self.surface_index >= SURFACE_COUNT {
            self.surface_index = 0;
        }
        Ok(())
    }

    /// Returns the get screen size of this [`SwapChain`].
    pub fn get_screen_size(&self) -> (f32, f32) {
        (self.resolution.Width as f32, self.resolution.Height as f32)
    }
}

fn config_best_display_mode(state: &DisplayState, target: &DisplayTarget) -> Result<()> {
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

    info!("modes.Size() : {}", modes.Size()?);

    for mode in modes {
        let v_sync = mode.PresentationRate()?.VerticalSyncRate;
        let v_sync_double = v_sync.Numerator as f32 / v_sync.Denominator as f32;

        let tr = mode.TargetResolution().unwrap();
        info!("TargetResolution : {}x{}", tr.Width, tr.Height);

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

    Ok(())
}
