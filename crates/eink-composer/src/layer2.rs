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

use crate::{shader::CompiledShaders, winrt::*};
use anyhow::{bail, Result};

use windows::{
    s,
    Win32::Graphics::{
        Direct3D::{
            Dxc::{DxcCreateInstance, IDxcCompiler2, IDxcCompiler3},
            Fxc::{D3DCompile, D3DCOMPILE_PREFER_FLOW_CONTROL},
            ID3DBlob, D3D_PRIMITIVE_TOPOLOGY, D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            D3D_SHADER_MACRO, D3D_SRV_DIMENSION_TEXTURE2D,
        },
        Direct3D11::{
            ID3D11Buffer, ID3D11InputLayout, ID3D11ShaderResourceView,
            D3D11_APPEND_ALIGNED_ELEMENT, D3D11_BIND_FLAG, D3D11_BIND_INDEX_BUFFER,
            D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC, D3D11_RESOURCE_MISC_FLAG,
            D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV,
            D3D11_VIEWPORT,
        },
        Dxgi::Common::{DXGI_FORMAT_R32_UINT, DXGI_FORMAT_R8G8B8A8_UNORM},
    },
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

// struct Mesh {
//     vertex_buffer: ID3D11Buffer,
//     index_buffer: ID3D11Buffer,
//     _texture: ID3D11Texture2D,
//     srv_tex: ID3D11ShaderResourceView,
// }

// impl Mesh {
//     fn new(width: u32, height: u32, device: &ID3D11Device5) -> Self {
//         let img = image::open("sample.png").unwrap().to_rgba8();

//         device
//             .create_buffer(
//                 &d3d11::BufferDesc::new()
//                     .byte_width((std::mem::size_of::<Vertex>() * vertices.len()) as u32)
//                     .usage(d3d11::Usage::Default)
//                     .bind_flags(d3d11::BindFlags::VertexBuffer),
//                 Some(&d3d11::SubresourceData::new().sys_mem(vertices.as_ptr())),
//             )
//             .unwrap();

//         let vertex_buffer = {
//             let w = (img.width() as f32) / (width as f32);
//             let h = (img.height() as f32) / (height as f32);
//             #[rustfmt::skip]
//             let vertices = [
//                 Vertex::new([-w, -h, 0.0], [0.0, 1.0]),
//                 Vertex::new([-w,  h, 0.0], [0.0, 0.0]),
//                 Vertex::new([ w,  h, 0.0], [1.0, 0.0]),
//                 Vertex::new([ w, -h, 0.0], [1.0, 1.0]),
//             ];
//         };

//         let index_buffer = {
//             let indices = [0, 1, 2, 2, 3, 0];
//             device
//                 .create_buffer(
//                     &d3d11::BufferDesc::new()
//                         .byte_width((std::mem::size_of::<u32>() * indices.len()) as u32)
//                         .usage(d3d11::Usage::Default)
//                         .bind_flags(d3d11::BindFlags::IndexBuffer),
//                     Some(&d3d11::SubresourceData::new().sys_mem(indices.as_ptr())),
//                 )
//                 .unwrap()
//         };

//         let texture = device
//             .create_texture2d(
//                 &d3d11::Texture2DDesc::new()
//                     .width(img.width())
//                     .height(img.height())
//                     .format(dxgi::Format::R8G8B8A8Unorm)
//                     .usage(d3d11::Usage::Default),
//                 Some(
//                     &d3d11::SubresourceData::new()
//                         .sys_mem(img.as_ptr())
//                         .sys_mem_pitch(4 * img.width()),
//                 ),
//             )
//             .unwrap();

//         let srv_tex = device.create_shader_resource_view(&texture, None).unwrap();

//         Self {
//             vertex_buffer,
//             index_buffer,
//             _texture: texture,
//             srv_tex,
//         }
//     }
// }

pub struct Layer {
    dev: ID3D11Device5,
    ctx: ID3D11DeviceContext,
    // dxgi_device: IDXGIDevice,
    // dxgi_adapter: IDXGIAdapter,
    tex2d: ID3D11Texture2D,
    // dxgi_keyed_mutex: IDXGIKeyedMutex,
    input_layout: ID3D11InputLayout,
    vs: ID3D11VertexShader,
    ps: ID3D11PixelShader,
    samplers: [Option<ID3D11SamplerState>; 1],
    vertex_buffer: ID3D11Buffer,
    indices_buffer: ID3D11Buffer,
    tex2d_srv: ID3D11ShaderResourceView,
    dxgi_keyed_mutex: IDXGIKeyedMutex,
    //
    // layer_width: u32,
    // layer_height: u32,
}

impl Layer {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn new(dev: &ID3D11Device, ctx: &ID3D11DeviceContext) -> Result<Self> {
        let dev = dev.cast::<ID3D11Device5>()?;
        let ctx = ctx.clone();

        let dxgi_dev = dev.cast::<IDXGIDevice>()?;
        let dxgi_adapter = unsafe { dxgi_dev.GetParent::<IDXGIAdapter>()? };

        // Create shared texture for all duplication threads to draw into
        let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
        tex_desc.Width = 100;
        tex_desc.Height = 100;
        tex_desc.MipLevels = 1;
        tex_desc.ArraySize = 1;
        tex_desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
        tex_desc.SampleDesc.Count = 1;
        tex_desc.Usage = D3D11_USAGE_DEFAULT;
        tex_desc.BindFlags = D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
        tex_desc.CPUAccessFlags = D3D11_CPU_ACCESS_FLAG::default();
        tex_desc.MiscFlags = D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX;

        let tex2d = unsafe { dev.CreateTexture2D(&tex_desc, std::ptr::null())? };
        let dxgi_keyed_mutex = tex2d.cast::<IDXGIKeyedMutex>()?;

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

        // let img = image::open("barkeep.png").unwrap().to_rgba8();
        // println!("img({}x{})", img.width(), img.height());

        let w = 1f32; // img.width() as f32 / 3840f32;
        let h = 1f32; // img.height() as f32 / 2160f32;
                      // println!("img({}x{})", w, h);

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
            CPUAccessFlags: D3D11_CPU_ACCESS_FLAG::default().0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: indices.as_ptr() as *const c_void,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        let indices_buffer = unsafe { dev.CreateBuffer(&indices_buffer_desc, &subresource_data)? };

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

        // let subresource_data = D3D11_SUBRESOURCE_DATA {
        //     pSysMem: img.as_ptr() as *const c_void,
        //     SysMemPitch: 4 * img.width(),
        //     SysMemSlicePitch: 0,
        // };

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

        Ok(Self {
            dev,
            ctx,
            // dxgi_device: dxgi_dev,
            // dxgi_adapter,
            tex2d,
            tex2d_srv,
            dxgi_keyed_mutex,
            input_layout,
            vs,
            ps,
            samplers: [Some(sampler)],
            vertex_buffer,
            indices_buffer,
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
            //     // self.dxgi_keyed_mutex.AcquireSync(1, 100)?;

            self.ctx.OMSetRenderTargets(&[Some(rtv.clone())], None);

            // let colors = [1.0, 0.0, 0.0, 0.0].as_ptr();
            // self.ctx.ClearRenderTargetView(&rtv.clone(), colors);

            let mut viewport = D3D11_VIEWPORT::default();
            viewport.TopLeftX = 100f32;
            viewport.TopLeftY = 100f32;
            viewport.Width = 760f32;
            viewport.Height = 822f32;

            self.ctx.RSSetViewports(&[viewport]);

            self.ctx.IASetVertexBuffers(
                0,
                1,
                &Some(self.vertex_buffer.clone()),
                (&[size_of::<Vertex>() as u32]).as_ptr(),
                (&[0]).as_ptr(),
            );

            self.ctx
                .IASetIndexBuffer(&self.indices_buffer, DXGI_FORMAT_R32_UINT, 0);

            self.ctx.IASetInputLayout(&self.input_layout);

            self.ctx
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

            self.ctx.VSSetShader(&self.vs, &[]);

            // [39856] D3D11 CORRUPTION: ID3D11DeviceContext::VSSetShader: Second parameter (ppClassInstances), array index 0 corrupt or unexpectedly NULL. [ MISCELLANEOUS CORRUPTION #14: CORRUPTED_PARAMETER2]

            self.ctx.PSSetShader(&self.ps, &[]);

            self.ctx
                .PSSetShaderResources(0, &[Some(self.tex2d_srv.clone())]);

            self.ctx.PSSetSamplers(0, &self.samplers);

            self.ctx.DrawIndexed(6, 0, 0);

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
            // self.dxgi_keyed_mutex.ReleaseSync(0)?;
        }

        Ok(())
    }
}
