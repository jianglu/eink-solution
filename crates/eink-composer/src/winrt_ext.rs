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

use std::ptr::null;

use anyhow::{bail, Result};

use ntapi::winapi::um::winnt::GENERIC_ALL;
use windows::{
    core::{IInspectable, IUnknown, InParam, Interface, HSTRING},
    Devices::Display::Core::{
        DisplayAdapter, DisplayDevice, DisplayFence, DisplayManager, DisplayPath,
        DisplayPrimaryDescription, DisplayScanout, DisplaySource, DisplaySurface, DisplayTarget,
    },
    Foundation::{IReference, PropertyValue},
    Graphics::{
        DirectX::{
            Direct3D11::Direct3DMultisampleDescription, DirectXColorSpace, DirectXPixelFormat,
        },
        SizeInt32,
    },
    Win32::{
        Foundation::{HINSTANCE, LUID},
        Graphics::{
            Direct3D::{
                D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL,
                D3D_FEATURE_LEVEL_11_1,
            },
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11Device5, ID3D11DeviceContext, ID3D11Fence,
                ID3D11RenderTargetView, ID3D11Texture2D, D3D11_CREATE_DEVICE_DEBUG,
                D3D11_CREATE_DEVICE_FLAG, D3D11_FENCE_FLAG_SHARED, D3D11_RENDER_TARGET_VIEW_DESC,
                D3D11_RTV_DIMENSION_TEXTURE2D, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
            },
            Dxgi::{
                CreateDXGIFactory2, IDXGIAdapter, IDXGIAdapter4, IDXGIFactory6,
                DXGI_CREATE_FACTORY_DEBUG,
            },
        },
        System::WinRT::Display::IDisplayDeviceInterop,
    },
};

pub trait DisplayManagerExt {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn find_target_by_stable_id(&self, monitor_id: &str) -> Result<DisplayTarget>;
}

impl DisplayManagerExt for DisplayManager {
    fn find_target_by_stable_id(&self, monitor_id: &str) -> Result<DisplayTarget> {
        let targets = self.GetCurrentTargets()?;
        for target in targets {
            let id = target.StableMonitorId()?;
            println!("MonitorId: {}", id);
            if id == monitor_id {
                return Ok(target);
            }
        }
        bail!("Cannot found {}", monitor_id);
    }
}

pub trait DisplayPathExt {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn set_is_interlaced(&self, is_interlaced: bool) -> ::windows::core::Result<()>;
}

impl DisplayPathExt for DisplayPath {
    fn set_is_interlaced(&self, is_interlaced: bool) -> ::windows::core::Result<()> {
        let value = PropertyValue::CreateBoolean(is_interlaced)
            .unwrap()
            .cast::<IReference<bool>>()
            .unwrap();
        self.SetIsInterlaced(InParam::owned(value))
    }
}

pub trait DisplayAdapterExt {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn create_dxgi_adapter(&self) -> Result<IDXGIAdapter>;
}

impl DisplayAdapterExt for DisplayAdapter {
    fn create_dxgi_adapter(&self) -> Result<IDXGIAdapter> {
        let factory: IDXGIFactory6 = unsafe { CreateDXGIFactory2(0)? };

        // Find the GPU that the target is connected to
        let adapter_id = self.Id()?;

        let adapter_luid = LUID {
            LowPart: adapter_id.LowPart,
            HighPart: adapter_id.HighPart,
        };

        let dxgi_adapter = unsafe { factory.EnumAdapterByLuid(adapter_luid)? };

        Ok(dxgi_adapter)
    }
}

pub trait DisplayDeviceExt {
    fn create_primary_surface(
        &self,
        target: &DisplayTarget,
        resolution: SizeInt32,
        pixel_fmt: DirectXPixelFormat,
    ) -> Result<DisplaySurface>;

    fn create_simple_scanout(
        &self,
        source: &DisplaySource,
        surface: &DisplaySurface,
    ) -> Result<DisplayScanout>;

    fn create_shared_fence(&self, d3d_fence: &ID3D11Fence) -> Result<DisplayFence>;
}

impl DisplayDeviceExt for DisplayDevice {
    fn create_primary_surface(
        &self,
        target: &DisplayTarget,
        resolution: SizeInt32,
        pixel_fmt: DirectXPixelFormat,
    ) -> Result<DisplaySurface> {
        let multisample_desc = Direct3DMultisampleDescription {
            Count: 1,
            Quality: 0,
        };

        // Create a surface format description for the primaries
        // 创建主表面描述符
        let primary_desc = DisplayPrimaryDescription::CreateWithProperties(
            None,
            resolution.Width as u32,
            resolution.Height as u32,
            pixel_fmt,
            DirectXColorSpace::RgbFullG22NoneP709,
            false,
            multisample_desc,
        )?;
        // println!("DisplayPrimaryDescription: {:?}", primary_desc);
        Ok(self.CreatePrimary(target, &primary_desc)?)
    }

    fn create_simple_scanout(
        &self,
        source: &DisplaySource,
        surface: &DisplaySurface,
    ) -> Result<DisplayScanout> {
        Ok(self.CreateSimpleScanout(source, surface, 0, 1)?)
    }

    fn create_shared_fence(&self, d3d_fence: &ID3D11Fence) -> Result<DisplayFence> {
        let device_interop = self.cast::<IDisplayDeviceInterop>()?;

        // Share the ID3D11Fence across devices using a handle
        let fence_handle = unsafe { d3d_fence.CreateSharedHandle(null(), GENERIC_ALL, None)? };

        // Call OpenSharedHandle on the DisplayDevice to get a DisplayFence
        let fence = unsafe { device_interop.OpenSharedHandle(fence_handle, DisplayFence::IID)? };

        let fence: IUnknown = unsafe { std::mem::transmute(fence) };
        let fence = fence.cast::<DisplayFence>()?;

        Ok(fence)
    }
}

pub trait DisplaySurfaceExt {
    fn create_shared_texture2d(
        &self,
        d3d_device: &ID3D11Device,
        device: &DisplayDevice,
    ) -> Result<ID3D11Texture2D>;
}

impl DisplaySurfaceExt for DisplaySurface {
    fn create_shared_texture2d(
        &self,
        d3d_device: &ID3D11Device,
        device: &DisplayDevice,
    ) -> Result<ID3D11Texture2D> {
        let device_interop = device.cast::<IDisplayDeviceInterop>()?;
        let surface = self.cast::<IInspectable>()?;

        // Share the DisplaySurface across devices using a handle
        let surface_shared_handle = unsafe {
            device_interop.CreateSharedHandle(&surface, null(), GENERIC_ALL, &HSTRING::default())?
        };

        // Call OpenSharedResource1 on the D3D device to get the ID3D11Texture2D
        let d3d_device5 = d3d_device.cast::<ID3D11Device5>()?;
        let texture: ID3D11Texture2D =
            unsafe { d3d_device5.OpenSharedResource1(surface_shared_handle)? };

        Ok(texture)
    }
}
pub trait IDXGIAdapterExt {
    fn create_d3d11_device(&self) -> Result<(ID3D11Device, ID3D11DeviceContext)>;
}

impl IDXGIAdapterExt for IDXGIAdapter {
    /// 创建 D3D11 设备
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn create_d3d11_device(&self) -> Result<(ID3D11Device, ID3D11DeviceContext)> {
        // Create the D3D device and context from the adapter
        let mut device_opt: Option<ID3D11Device> = None;
        let mut device_context_opt: Option<ID3D11DeviceContext> = None;
        let feature_levels = [D3D_FEATURE_LEVEL_11_1; 1];
        let mut feature_level = D3D_FEATURE_LEVEL(0);

        unsafe {
            D3D11CreateDevice(
                self,
                D3D_DRIVER_TYPE_UNKNOWN,
                HINSTANCE(0),
                D3D11_CREATE_DEVICE_DEBUG,
                &feature_levels,
                D3D11_SDK_VERSION,
                &mut device_opt,
                &mut feature_level,
                &mut device_context_opt,
            )?
        };

        let d3d11_device = device_opt.unwrap();
        let d3d11_device_context = device_context_opt.unwrap();

        Ok((d3d11_device, d3d11_device_context))
    }
}

pub trait ID3D11DeviceExt {
    fn create_shared_fence(&self) -> Result<ID3D11Fence>;

    fn create_render_target_view(
        &self,
        texture2d: &ID3D11Texture2D,
    ) -> Result<ID3D11RenderTargetView>;
}

impl ID3D11DeviceExt for ID3D11Device {
    fn create_shared_fence(&self) -> Result<ID3D11Fence> {
        let this = self.cast::<ID3D11Device5>()?;
        let mut fence_opt: Option<ID3D11Fence> = None;
        unsafe { this.CreateFence(0, D3D11_FENCE_FLAG_SHARED, &mut fence_opt)? };
        Ok(fence_opt.unwrap())
    }

    /// 创建 Texture2D 对应的 RenderTargetView
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn create_render_target_view(
        &self,
        texture2d: &ID3D11Texture2D,
    ) -> Result<ID3D11RenderTargetView> {
        let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture2d.GetDesc(&mut tex_desc) };

        let mut view_desc = D3D11_RENDER_TARGET_VIEW_DESC::default();
        view_desc.ViewDimension = D3D11_RTV_DIMENSION_TEXTURE2D;
        view_desc.Anonymous.Texture2D.MipSlice = 0;
        view_desc.Format = tex_desc.Format;

        // Create a render target view for the surface
        let render_target_view = unsafe { self.CreateRenderTargetView(texture2d, &view_desc)? };
        Ok(render_target_view)
    }
}
