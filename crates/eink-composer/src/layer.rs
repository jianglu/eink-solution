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

use std::{ffi::c_void, mem::size_of};

use crate::{pc_str, shader::CompiledShaders, winrt::*};
use anyhow::{bail, Result};
use uuid::Uuid;
use windows::Win32::{
    Foundation::{BOOL, DUPLICATE_HANDLE_OPTIONS, DUPLICATE_SAME_ACCESS},
    Graphics::{
        Direct3D::WKPDID_CommentStringW,
        Direct3D11::{
            ID3D11DepthStencilState, ID3D11Resource, D3D11_BOX, D3D11_COMPARISON_ALWAYS,
            D3D11_COMPARISON_LESS, D3D11_DEPTH_STENCILOP_DESC, D3D11_DEPTH_STENCIL_DESC,
            D3D11_DEPTH_WRITE_MASK_ALL, D3D11_RESOURCE_MISC_SHARED,
            D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_STENCIL_OP, D3D11_STENCIL_OP_KEEP,
        },
        Dxgi::{IDXGIResource1, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE},
    },
    System::Threading::GetCurrentProcess,
};

#[repr(C)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    fn new(position: [f32; 3], uv: [f32; 2]) -> Self {
        Self { position, uv }
    }
}

pub struct Layer {
    pub pid: u32,

    dev: ID3D11Device5,
    ctx: ID3D11DeviceContext,
    // dxgi_device: IDXGIDevice,
    // dxgi_adapter: IDXGIAdapter,
    tex2d: ID3D11Texture2D,
    tex2d_keyed_mutex: IDXGIKeyedMutex,
    input_layout: ID3D11InputLayout,
    vs: ID3D11VertexShader,
    ps: ID3D11PixelShader,
    samplers: [Option<ID3D11SamplerState>; 1],
    vertex_buffer: ID3D11Buffer,
    indices_buffer: ID3D11Buffer,
    tex2d_srv: ID3D11ShaderResourceView,
    tex2d_shared_handle: HANDLE,
    pub tex2d_shared_name: HSTRING,

    // 内容尺寸
    x: i32,
    y: i32,
    width: i32,
    height: i32,

    // Surface 尺寸
    display_width: i32,
    display_height: i32,

    // 材质尺寸
    tex_width: i32,
    tex_height: i32,
    depth_stencil_state: ID3D11DepthStencilState,
}

impl Layer {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn new(
        pid: u32,
        dev: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        display_width: i32,
        display_height: i32,
        test: bool,
    ) -> Result<Self> {
        let dev = dev.cast::<ID3D11Device5>()?;
        let ctx = ctx.clone();

        // let dxgi_dev = dev.cast::<IDXGIDevice>()?;
        // let dxgi_adapter = unsafe { dxgi_dev.GetParent::<IDXGIAdapter>()? };

        let tex_width = i32::max(width, display_width);
        let tex_height = i32::max(height, display_height);

        println!("tex_size: {}x{}", tex_width, tex_height);

        let tex2d = if test {
            let img = image::open("background.png").unwrap().to_rgba8();
            println!("img({}x{})", img.width(), img.height());

            let subresource_data = D3D11_SUBRESOURCE_DATA {
                pSysMem: img.as_ptr() as *const c_void,
                SysMemPitch: 4 * img.width(),
                SysMemSlicePitch: 0,
            };

            // 创建跨进程共享材质
            let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
            tex_desc.Width = img.width() as u32;
            tex_desc.Height = img.height() as u32;
            tex_desc.MipLevels = 1;
            tex_desc.ArraySize = 1;
            tex_desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
            tex_desc.SampleDesc.Count = 1;
            tex_desc.Usage = D3D11_USAGE_DEFAULT;
            tex_desc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
            tex_desc.CPUAccessFlags = D3D11_CPU_ACCESS_FLAG::default();
            tex_desc.MiscFlags =
                D3D11_RESOURCE_MISC_SHARED_NTHANDLE | D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX;

            let tex2d = unsafe {
                // 创建空材质
                dev.CreateTexture2D(&tex_desc, &subresource_data)?
            };

            tex2d
        } else {
            // 创建跨进程共享材质
            let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
            tex_desc.Width = tex_width as u32;
            tex_desc.Height = tex_height as u32;
            tex_desc.MipLevels = 1;
            tex_desc.ArraySize = 1;
            tex_desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
            tex_desc.SampleDesc.Count = 1;
            tex_desc.Usage = D3D11_USAGE_DEFAULT;
            tex_desc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
            tex_desc.CPUAccessFlags = D3D11_CPU_ACCESS_FLAG::default();
            tex_desc.MiscFlags =
                D3D11_RESOURCE_MISC_SHARED_NTHANDLE | D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX;

            let tex2d = unsafe {
                // 创建空材质
                dev.CreateTexture2D(&tex_desc, std::ptr::null())?
            };

            tex2d
        };

        println!("tex2d({:?})", tex2d);

        let tex2d_keyed_mutex = tex2d.cast::<IDXGIKeyedMutex>()?;
        println!("tex2d_keyed_mutex({:?})", tex2d_keyed_mutex);

        let tex2d_res = tex2d.cast::<IDXGIResource1>()?;
        println!("tex2d_res({:?})", tex2d_res);

        let tex2d_shared_name = HSTRING::from(format!("Surface-{:?}", Uuid::new_v4()));
        let tex2d_shared_handle = unsafe {
            tex2d_res.CreateSharedHandle(
                std::ptr::null(),
                DXGI_SHARED_RESOURCE_READ + DXGI_SHARED_RESOURCE_WRITE,
                &tex2d_shared_name,
            )?
        };
        println!("tex2d_shared_name({:?})", tex2d_shared_name);

        // Create the sample state
        let mut samp_desc = D3D11_SAMPLER_DESC::default();
        samp_desc.Filter = D3D11_FILTER_MIN_MAG_MIP_LINEAR;
        samp_desc.AddressU = D3D11_TEXTURE_ADDRESS_CLAMP;
        samp_desc.AddressV = D3D11_TEXTURE_ADDRESS_CLAMP;
        samp_desc.AddressW = D3D11_TEXTURE_ADDRESS_CLAMP;
        samp_desc.ComparisonFunc = D3D11_COMPARISON_NEVER;
        samp_desc.MinLOD = 0f32;
        samp_desc.MaxLOD = D3D11_FLOAT32_MAX;

        let sampler = unsafe { dev.CreateSamplerState(&samp_desc).unwrap() };

        let (input_layout, vs, ps) = Layer::init_shaders(&dev, &ctx)?;

        // let sampler_desc = D3D11_SAMPLER_DESC::default();
        // let sampler = unsafe { d3d_device.CreateSamplerState(&sampler_desc)? };

        let w = 1f32; // img.width() as f32 / 3840f32;
        let h = 1f32; // img.height() as f32 / 2160f32;
        println!("img({}x{})", w, h);

        // 顶点缓冲
        #[rustfmt::skip]
        let vertices = [
            Vertex::new([-w, -h, 0.0], [0.0, 1.0]),
            Vertex::new([-w,  h, 0.0], [0.0, 0.0]),
            Vertex::new([ w,  h, 0.0], [1.0, 0.0]),
            Vertex::new([ w, -h, 0.0], [1.0, 1.0]),
        ];

        let vertex_buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: (size_of::<Vertex>() * vertices.len()) as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0,
            CPUAccessFlags: 0, // D3D11_CPU_ACCESS_FLAG,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: vertices.as_ptr() as *const c_void,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        let vertex_buffer = unsafe { dev.CreateBuffer(&vertex_buffer_desc, &subresource_data)? };

        // 索引缓冲
        let indices = [0, 1, 2, 2, 3, 0];

        let indices_buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: (size_of::<Vertex>() * vertices.len()) as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_INDEX_BUFFER.0,
            CPUAccessFlags: 0, // D3D11_CPU_ACCESS_FLAG
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: indices.as_ptr() as *const c_void,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        let indices_buffer = unsafe {
            // 创建索引缓冲
            dev.CreateBuffer(&indices_buffer_desc, &subresource_data)?
        };

        // // 创建 Texture
        // let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
        // tex_desc.Width = img.width();
        // tex_desc.Height = img.height();
        // tex_desc.MipLevels = 1;
        // tex_desc.ArraySize = 1;
        // tex_desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        // tex_desc.SampleDesc.Count = 1;
        // tex_desc.Usage = D3D11_USAGE_DEFAULT;
        // tex_desc.BindFlags = D3D11_BIND_SHADER_RESOURCE; // D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
        // tex_desc.CPUAccessFlags = D3D11_CPU_ACCESS_FLAG::default();
        // tex_desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG::default(); // D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX;

        // let tex2d = unsafe { dev.CreateTexture2D(&tex_desc, &subresource_data)? };
        // println!("tex2d: {:?}", tex2d);

        // let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
        //     Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        //     ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
        //     Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
        //         Texture2D: D3D11_TEX2D_SRV {
        //             MostDetailedMip: 1,
        //             MipLevels: 1,
        //         },
        //     },
        // };
        let tex2d_srv = unsafe { dev.CreateShaderResourceView(&tex2d, std::ptr::null())? };
        println!("tex2d_srv: {:?}", tex2d_srv);

        let depth_stencil_desc = D3D11_DEPTH_STENCIL_DESC {
            DepthEnable: false.into(),
            DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ALL,
            DepthFunc: D3D11_COMPARISON_LESS,
            StencilEnable: false.into(),
            StencilReadMask: 0xff,
            StencilWriteMask: 0xff,
            FrontFace: D3D11_DEPTH_STENCILOP_DESC {
                StencilFailOp: D3D11_STENCIL_OP_KEEP,
                StencilDepthFailOp: D3D11_STENCIL_OP_KEEP,
                StencilPassOp: D3D11_STENCIL_OP_KEEP,
                StencilFunc: D3D11_COMPARISON_ALWAYS,
            },
            BackFace: D3D11_DEPTH_STENCILOP_DESC {
                StencilFailOp: D3D11_STENCIL_OP_KEEP,
                StencilDepthFailOp: D3D11_STENCIL_OP_KEEP,
                StencilPassOp: D3D11_STENCIL_OP_KEEP,
                StencilFunc: D3D11_COMPARISON_ALWAYS,
            },
        };

        let depth_stencil_state = unsafe { dev.CreateDepthStencilState(&depth_stencil_desc)? };

        Ok(Self {
            pid,
            dev,
            ctx,
            // dxgi_device: dxgi_dev,
            // dxgi_adapter,
            tex2d,
            tex2d_srv,
            tex2d_keyed_mutex,
            tex2d_shared_handle,
            tex2d_shared_name,
            input_layout,
            vs,
            ps,
            samplers: [Some(sampler)],
            vertex_buffer,
            indices_buffer,
            depth_stencil_state,
            x,
            y,
            width,
            height,
            display_width,
            display_height,
            tex_width,
            tex_height,
        })
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn init_shaders(
        dev: &ID3D11Device5,
        ctx: &ID3D11DeviceContext,
    ) -> Result<(ID3D11InputLayout, ID3D11VertexShader, ID3D11PixelShader)> {
        let layout: [D3D11_INPUT_ELEMENT_DESC; 2] = [
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR("POSITION\0".as_ptr()),
                SemanticIndex: 0u32,
                Format: DXGI_FORMAT_R32G32B32_FLOAT,
                InputSlot: 0u32,
                AlignedByteOffset: 0u32,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0u32,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR("TEXCOORD\0".as_ptr()),
                SemanticIndex: 0u32,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0u32,
                AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0u32,
            },
        ];

        unsafe {
            let shaders = CompiledShaders::new(&dev.cast::<ID3D11Device>()?);

            let input_layout = dev.CreateInputLayout(&layout, &shaders.bytecode())?;

            Ok((input_layout, shaders.vertex, shaders.pixel))
        }
    }

    pub fn set_bounds(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn draw(&mut self, rtv: &ID3D11RenderTargetView) -> Result<()> {
        // Try and acquire sync on common display buffer
        unsafe {
            const INFINITE: u32 = 0xFFFFFFFF; // Infinite timeout
            self.tex2d_keyed_mutex.AcquireSync(0, INFINITE)?;

            self.ctx
                .OMSetDepthStencilState(&self.depth_stencil_state, 0);

            self.ctx.OMSetRenderTargets(&[Some(rtv.clone())], None);

            let mut dst_resource: Option<ID3D11Resource> = None;
            rtv.GetResource(
                &dst_resource as *const Option<ID3D11Resource> as *mut Option<ID3D11Resource>,
            );
            let dst_resource = dst_resource.unwrap();

            // let mut s = String::with_capacity(1024);
            // let mut size = 0u32;

            // dst_resource.GetPrivateData(
            //     &WKPDID_CommentStringW,
            //     &size as *const u32 as *mut u32,
            //     s.as_mut_ptr() as *mut c_void,
            // );

            // s.shrink_to(size as usize);

            // println!(
            //     "Layer:draw: {},{},{},{}",
            //     self.x, self.y, self.width, self.height
            // );

            let srcbox = D3D11_BOX {
                left: 0,
                top: 0,
                front: 0,
                right: i32::min(2560, self.width) as u32,
                bottom: i32::min(1600, self.height) as u32,
                back: 1,
            };

            self.ctx.CopySubresourceRegion(
                &dst_resource,
                0,
                self.x as u32,
                self.y as u32,
                0,
                &self.tex2d,
                0,
                &srcbox, // std::ptr::null(),
            );

            // let colors = [1.0, 0.0, 0.0, 0.0].as_ptr();
            // self.ctx.ClearRenderTargetView(&rtv.clone(), colors);

            // let mut viewport = D3D11_VIEWPORT::default();
            // viewport.TopLeftX = self.x as f32;
            // viewport.TopLeftY = self.y as f32;
            // viewport.Width = self.tex_width as f32;
            // viewport.Height = self.tex_height as f32;

            // self.ctx.RSSetViewports(&[viewport]);

            // let mut scissor_rect = RECT::default();

            // scissor_rect.left = 0;
            // scissor_rect.top = 0;
            // scissor_rect.right = 0.5;
            // scissor_rect.bottom = 0.5;
            // self.ctx.RSSetScissorRects(&[scissor_rect]);

            // self.ctx.IASetVertexBuffers(
            //     0,
            //     1,
            //     &Some(self.vertex_buffer.clone()),
            //     (&[size_of::<Vertex>() as u32]).as_ptr(),
            //     (&[0]).as_ptr(),
            // );

            // self.ctx
            //     .IASetIndexBuffer(&self.indices_buffer, DXGI_FORMAT_R32_UINT, 0);

            // self.ctx.IASetInputLayout(&self.input_layout);

            // self.ctx
            //     .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

            // self.ctx.VSSetShader(&self.vs, &[]);
            // self.ctx.PSSetShader(&self.ps, &[]);

            // self.ctx
            //     .PSSetShaderResources(0, &[Some(self.tex2d_srv.clone())]);

            // self.ctx.PSSetSamplers(0, &self.samplers);

            // self.ctx.DrawIndexed(6, 0, 0);

            // 清除状态
            self.ctx.ClearState();

            // self.d3d_device_context.DrawIndexed(indexcount, startindexlocation, basevertexlocation)

            // self.d3d_device_context.IASetVertexBuffers(
            //     0,
            //     1,
            //     &Some(vertex_buffer),
            //     &stride,
            //     &offset,
            // );

            // Draw textured quad onto render target
            // self.d3d_device_context.Draw(NUMVERTICES as u32, 0);

            // Release keyed mutex
            self.tex2d_keyed_mutex.ReleaseSync(0)?;
        }

        Ok(())
    }

    // /// 将材质句柄复制到远程进程
    // pub fn dup_handle(&self, target_proc_handle: HANDLE) -> Result<HANDLE> {
    //     unsafe {
    //         let dxgi_res = self.tex2d.cast::<IDXGIResource1>()?;

    //         let dxgi_shared_handle = dxgi_res.CreateSharedHandle(
    //             std::ptr::null(),
    //             DXGI_SHARED_RESOURCE_READ + DXGI_SHARED_RESOURCE_WRITE,
    //             w!("Surface1"),
    //         )?;

    //         // println!("DXGI Shared Handle: {:?}", dxgi_shared_handle);
    //         // let source_handle = GetCurrentProcess();
    //         // println!("Target Process Handle: {:?}", target_proc_handle);
    //         // let target_handle = HANDLE::default();
    //         // println!("Source Handle: {:?}", source_handle);
    //         // let target_handle = HANDLE::default();
    //         // DuplicateHandle(
    //         //     source_handle,
    //         //     dxgi_shared_handle,
    //         //     target_proc_handle,
    //         //     &target_handle as *const HANDLE as *mut HANDLE,
    //         //     DUPLICATE_SAME_ACCESS.0,
    //         //     BOOL(0),
    //         //     DUPLICATE_HANDLE_OPTIONS::default(),
    //         // );
    //         // println!("Target Handle: {:?}", target_handle);
    //         // Ok(target_handle)
    //     }
    // }
}
