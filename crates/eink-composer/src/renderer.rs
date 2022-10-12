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

use crate::winrt::*;
use anyhow::Result;

const SURFACE_COUNT: usize = 2;

pub struct D3D11Renderer {
    d3d11_device: ID3D11Device5,
    d3d11_device_context: ID3D11DeviceContext,
    d3d11_surfaces: Vec<ID3D11Texture2D>, // , SurfaceCount> ;
    d3d11_render_targets: Vec<ID3D11RenderTargetView>, // SurfaceCount>;
    d3d11_fence: ID3D11Fence,

    fence_value: u64,
    frame_count: u64,
    // layers: Vec<Layer>,
}

const GENERIC_ALL: u32 = 0x10000000;

/// 由 Shared Surface 创建 Texture2D
///
/// # Errors
///
/// This function will return an error if .
fn create_texture2d_from_shared_surface(
    d3d11_device5: &ID3D11Device5,
    display_device: &DisplayDevice,
    display_surface: &DisplaySurface,
) -> Result<ID3D11Texture2D> {
    let display_device_interop = display_device.cast::<IDisplayDeviceInterop>()?;
    let display_surface_inspectable = display_surface.cast::<IInspectable>()?;

    // Share the DisplaySurface across devices using a handle
    let display_surface_shared_handle = unsafe {
        display_device_interop.CreateSharedHandle(
            InParam::owned(display_surface_inspectable),
            std::ptr::null(), // pSecurityAttributes
            GENERIC_ALL,      // Access
            &HSTRING::default(),
        )?
    };

    // Call OpenSharedResource1 on the D3D device to get the ID3D11Texture2D
    let d3d_surface: ID3D11Texture2D =
        unsafe { d3d11_device5.OpenSharedResource1(display_surface_shared_handle)? };

    Ok(d3d_surface)
}

/// 创建 Texture2D 对应的 RenderTargetView
///
/// # Errors
///
/// This function will return an error if .
fn create_render_target_view(
    d3d11_device5: &ID3D11Device5,
    texture2d: &ID3D11Texture2D,
) -> Result<ID3D11RenderTargetView> {
    let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture2d.GetDesc(&mut tex_desc) };

    let mut view_desc = D3D11_RENDER_TARGET_VIEW_DESC::default();
    view_desc.ViewDimension = D3D11_RTV_DIMENSION_TEXTURE2D;
    view_desc.Anonymous.Texture2D.MipSlice = 0;
    view_desc.Format = tex_desc.Format;

    // Create a render target view for the surface
    let render_target_view =
        unsafe { d3d11_device5.CreateRenderTargetView(texture2d, &view_desc)? };
    Ok(render_target_view)
}

/// 创建 ID3D11Fence 的跨进程访问句柄
///
/// # Errors
///
/// This function will return an error if .
fn create_shared_display_fence(
    display_device: &DisplayDevice,
    d3d11_fence: &ID3D11Fence,
) -> Result<DisplayFence> {
    let device_interop = display_device.cast::<IDisplayDeviceInterop>()?;

    // Share the ID3D11Fence across devices using a handle
    let fence_handle =
        unsafe { d3d11_fence.CreateSharedHandle(std::ptr::null(), GENERIC_ALL, None)? };

    // Call OpenSharedHandle on the DisplayDevice to get a DisplayFence
    let display_fence =
        unsafe { device_interop.OpenSharedHandle(fence_handle, DisplayFence::IID)? };

    let display_fence: IUnknown = unsafe { std::mem::transmute(display_fence) };
    let display_fence = display_fence.cast::<DisplayFence>()?;

    Ok(display_fence)
}

/// 创建 Display 同步栅栏
///
/// D3D11_FENCE_FLAG_SHARED - 同适配器不同上下文
/// D3D11_FENCE_FLAG_SHARED_CROSS_ADAPTER - 不同适配器
///
/// # Panics
///
/// Panics if .
///
/// # Errors
///
/// This function will return an error if .
fn create_shared_d3d11_fence(d3d11_device_5: &ID3D11Device5) -> Result<ID3D11Fence> {
    unsafe {
        let mut d3d11_fence_opt: Option<ID3D11Fence> = None;
        d3d11_device_5.CreateFence(0, D3D11_FENCE_FLAG_SHARED, &mut d3d11_fence_opt)?;
        let d3d11_fence = d3d11_fence_opt.unwrap();
        Ok(d3d11_fence)
    }
}

/// 创建 D3D11 设备
///
/// # Errors
///
/// This function will return an error if .
fn create_d3d11_device(
    dxgi_adapter: &IDXGIAdapter4,
) -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    // Create the D3D device and context from the adapter
    let mut d3d11_device_opt: Option<ID3D11Device> = None;
    let mut d3d11_device_context_opt: Option<ID3D11DeviceContext> = None;
    let feature_levels = [D3D_FEATURE_LEVEL(0); 0];
    let mut feature_level = D3D_FEATURE_LEVEL(0);

    unsafe {
        D3D11CreateDevice(
            dxgi_adapter,                // pAdapter
            D3D_DRIVER_TYPE_UNKNOWN,     // DriverType
            HINSTANCE(0),                // Software
            D3D11_CREATE_DEVICE_FLAG(0), // Flags
            &feature_levels,             // [in] pFeatureLevels
            D3D11_SDK_VERSION,           // SDKVersion
            &mut d3d11_device_opt,       // ppDevice
            &mut feature_level,          // [out] pFeatureLevel
            &mut d3d11_device_context_opt,
        )?
    };

    let d3d11_device = d3d11_device_opt.unwrap();
    let d3d11_device_context = d3d11_device_context_opt.unwrap();

    Ok((d3d11_device, d3d11_device_context))
}

/// 由 DisplayAdapter 创建 IDXGIAdapter
///
/// # Panics
///
/// Panics if .
///
/// # Errors
///
/// This function will return an error if .
fn create_dxgi_adapter_from_display_adapter(adapter: &DisplayAdapter) -> Result<IDXGIAdapter4> {
    // let flags = Some(CreateFactoryFlag::Debug);
    // let dxgi_factory = dxgi::create_dxgi_factory2::<dxgi::Factory4>(flags)?;

    let factory: IDXGIFactory6 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_DEBUG)? };

    // Find the GPU that the target is connected to
    let display_adapter_id = adapter.Id().unwrap();

    let adapter_luid = LUID {
        LowPart: display_adapter_id.LowPart,
        HighPart: display_adapter_id.HighPart,
    };

    let dxgi_adapter: IDXGIAdapter4 = unsafe { factory.EnumAdapterByLuid(adapter_luid)? };
    // let dxgi_adapter = dxgi_factory.enum_adapter_by_luid(adapter_luid)?;

    Ok(dxgi_adapter)
}

impl D3D11Renderer {
    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn new(adapter: DisplayAdapter) -> Result<Self> {
        // DisplayAdapter -> IDXGIAdapter4
        let dxgi_adapter = create_dxgi_adapter_from_display_adapter(&adapter)?;

        // Create ID3D11Device
        let tuple = create_d3d11_device(&dxgi_adapter)?;
        let (d3d11_device, d3d11_device_context) = tuple;
        let d3d11_device: ID3D11Device5 = d3d11_device.cast()?;

        // Create Shared ID3D11Fence
        let d3d11_fence = create_shared_d3d11_fence(&d3d11_device)?;

        Ok(Self {
            d3d11_device,
            d3d11_device_context,
            d3d11_surfaces: Default::default(),
            d3d11_render_targets: Default::default(),
            d3d11_fence,
            fence_value: 0,
            frame_count: 0,
            // layers: Vec::default(),
        })
    }

    pub fn init_layers(&mut self) -> Result<()> {
        // self.layers
        //     .push(Layer::new(&self.d3d11_device, &self.d3d11_device_context)?);
        Ok(())
    }

    pub fn open_surfaces(
        &mut self,
        display_device: &DisplayDevice,
        surfaces: Vec<DisplaySurface>,
    ) -> Result<()> {
        for surface_index in 0..SURFACE_COUNT {
            // DisplaySurface -> ID3D11Texture2D
            let d3d11_texture2d = create_texture2d_from_shared_surface(
                &self.d3d11_device,
                &display_device,
                &surfaces[surface_index],
            )?;
            println!("d3d11_texture2d: {:?}", d3d11_texture2d);

            // Create a render target view for the surface
            // ID3D11Texture2D -> ID3D11RenderTargetView
            let d3d11_render_target =
                create_render_target_view(&self.d3d11_device, &d3d11_texture2d)?;
            println!("d3d11_render_target: {:?}", d3d11_render_target);

            self.d3d11_surfaces.push(d3d11_texture2d);
            self.d3d11_render_targets.push(d3d11_render_target);
        }
        Ok(())
    }

    // 创建 ID3D11Fence 的跨进程访问句柄
    pub fn get_fence(&mut self, display_device: &DisplayDevice) -> Result<DisplayFence> {
        let display_fence = create_shared_display_fence(&display_device, &self.d3d11_fence)?;
        println!("display_fence: {:?}", display_fence);
        return Ok(display_fence);
    }

    pub fn render_and_get_fence_value(&mut self, surface_index: usize) -> u64 {
        // TODO: Perform rendering here with D3D11

        let rtv = &self.d3d11_render_targets[surface_index];

        // For the sample, we simply render a color pattern using a frame counter. This code is not interesting.
        unsafe {
            self.frame_count = self.frame_count + 1;

            let amount = f32::abs(f32::sin(self.frame_count as f32 / 30.0 * 3.141592));

            let frame_count_div = self.frame_count / 30;

            let mut clear_color = [1f32, 1f32, 1f32, 1f32];
            clear_color[0] = amount * if frame_count_div % 3 == 0 { 1.0 } else { 0.0 };
            clear_color[1] = amount * if frame_count_div % 3 == 1 { 1.0 } else { 0.0 };
            clear_color[2] = amount * if frame_count_div % 3 == 2 { 1.0 } else { 0.0 };

            self.d3d11_device_context
                .ClearRenderTargetView(rtv, &clear_color as *const f32);
        }

        // for layer in self.layers.iter_mut() {
        //     layer.draw(rtv.clone()).unwrap();
        // }

        let d3d_context_4 = self
            .d3d11_device_context
            .cast::<ID3D11DeviceContext4>()
            .unwrap();

        self.fence_value = self.fence_value + 1;

        unsafe {
            d3d_context_4
                .Signal(&self.d3d11_fence, self.fence_value)
                .unwrap()
        };

        return self.fence_value;
    }
}

// from_winrt_display_fence();
// from_winrt_display_adapter();
