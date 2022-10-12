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
use directx_math::{XMFLOAT2, XMFLOAT3};

//
// A vertex with a position and texture coordinate
//
struct VERTEX {
    pos: XMFLOAT3,
    tex_coord: XMFLOAT2,
}

const NUMVERTICES: usize = 6;

pub struct Layer {
    d3d_device: ID3D11Device5,
    d3d_device_context: ID3D11DeviceContext,
    dxgi_device: IDXGIDevice,
    dxgi_adapter: IDXGIAdapter,
    shared_surface: ID3D11Texture2D,
    dxgi_keyed_mutex: IDXGIKeyedMutex,
    vs_shader: ID3D11VertexShader,
    ps_shader: ID3D11PixelShader,
    d3d_samplers: [Option<ID3D11SamplerState>; 1],
}

use indoc::indoc;

use hassle_rs::compile_hlsl;
use windows::{
    s,
    Win32::Graphics::{
        Direct3D::{
            Dxc::{DxcCreateInstance, IDxcCompiler2, IDxcCompiler3},
            Fxc::{D3DCompile, D3DCOMPILE_PREFER_FLOW_CONTROL},
            ID3DBlob, D3D_SHADER_MACRO,
        },
        Direct3D11::{D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA},
    },
};

impl Layer {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn new(
        d3d_device: &ID3D11Device5,
        d3d_device_context: &ID3D11DeviceContext,
    ) -> Result<Self> {
        let d3d_device = d3d_device.clone();
        let d3d_device_context = d3d_device_context.clone();
        let dxgi_device = d3d_device.cast::<IDXGIDevice>()?;
        let dxgi_adapter = unsafe { dxgi_device.GetParent::<IDXGIAdapter>()? };

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

        let shared_surface = unsafe { d3d_device.CreateTexture2D(&tex_desc, std::ptr::null())? };
        let dxgi_keyed_mutex = shared_surface.cast::<IDXGIKeyedMutex>()?;

        // Create the sample state
        let mut samp_desc = D3D11_SAMPLER_DESC::default();
        samp_desc.Filter = D3D11_FILTER_MIN_MAG_MIP_LINEAR;
        samp_desc.AddressU = D3D11_TEXTURE_ADDRESS_CLAMP;
        samp_desc.AddressV = D3D11_TEXTURE_ADDRESS_CLAMP;
        samp_desc.AddressW = D3D11_TEXTURE_ADDRESS_CLAMP;
        samp_desc.ComparisonFunc = D3D11_COMPARISON_NEVER;
        samp_desc.MinLOD = 0f32;
        samp_desc.MaxLOD = D3D11_FLOAT32_MAX;

        let d3d_sampler_linear = unsafe { d3d_device.CreateSamplerState(&samp_desc).unwrap() };

        let (vs_shader, ps_shader) = Layer::init_shaders(&d3d_device, &d3d_device_context)?;

        Ok(Self {
            d3d_device,
            d3d_device_context,
            dxgi_device,
            dxgi_adapter,
            shared_surface,
            dxgi_keyed_mutex,
            vs_shader,
            ps_shader,
            d3d_samplers: [Some(d3d_sampler_linear)],
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
        d3d_device: &ID3D11Device5,
        d3d_device_context: &ID3D11DeviceContext,
    ) -> Result<(ID3D11VertexShader, ID3D11PixelShader)> {
        let shader_text = indoc! {"
            struct vs_input_t
            {
                float3 pos: POSITION;
                float2 uv: TEXCOORD;
            };

            struct vs_output_t
            {
                float4 pos: SV_POSITION;
                float2 uv: TEXCOORD;
            };

            Texture2D tex: register(t0);
            SamplerState samp: register(s0);

            vs_output_t vs_main(vs_input_t input)
            {
                vs_output_t output;
                output.pos = float4(input.pos, 1.0f);
                output.uv = input.uv;
                return output;
            }

            float4 ps_main(vs_output_t vs): SV_TARGET
            {
                return tex.Sample(samp, vs.uv);
            }
        "};
        println!("shader_text:\n\n{}", shader_text);

        let vs_bytecode = unsafe {
            hassle_rs::compile_hlsl("shader.hlsl", shader_text, "vs_main", "vs_6_1", &[], &[])?

            // let dxc_compiler = DxcCreateInstance::<IDxcCompiler2>(&IDxcCompiler2::IID)?;
            // println!("dxc_compiler: {:?}", dxc_compiler);

            // let mut pcode = Option::<ID3DBlob>::default();
            // let mut perrormsgs = Option::<ID3DBlob>::default();
            // D3DCompile(
            //     shader_text.as_ptr() as *const std::ffi::c_void,
            //     shader_text.len(),
            //     s!("shader.hlsl"),
            //     &D3D_SHADER_MACRO::default(),
            //     None,
            //     s!("vs_main"),
            //     s!("vs_4_0_level_9_3"),
            //     0,
            //     0,
            //     &mut pcode,
            //     &mut perrormsgs,
            // )?;
            // let pcode = pcode.unwrap();
            // core::slice::from_raw_parts(
            //     pcode.GetBufferPointer() as *const u8,
            //     pcode.GetBufferSize(),
            // )
        };
        println!("vs_bytecode: {:?}", vs_bytecode);
        let result = hassle_rs::validate_dxil(&vs_bytecode)?; // Only a Windows machine in Developer Mode can run non-validated DXIL
        println!("result: {:?}", result);

        let ps_bytecode = unsafe {
            let mut pcode = Option::<ID3DBlob>::default();
            let mut perrormsgs = Option::<ID3DBlob>::default();
            D3DCompile(
                shader_text.as_ptr() as *const std::ffi::c_void,
                shader_text.len(),
                s!("shader.hlsl"),
                &D3D_SHADER_MACRO::default(),
                None,
                s!("ps_main"),
                s!("ps_4_0_level_9_3"),
                0,
                0,
                &mut pcode,
                &mut perrormsgs,
            )?;
            let pcode = pcode.unwrap();
            core::slice::from_raw_parts(
                pcode.GetBufferPointer() as *const u8,
                pcode.GetBufferSize(),
            )
        };
        println!("ps_bytecode: {:?}", ps_bytecode);

        let vertex_shader = unsafe { d3d_device.CreateVertexShader(vs_bytecode.as_slice(), None)? };
        println!("vertex_shader: {:?}", vertex_shader);

        let pixel_shader = unsafe { d3d_device.CreatePixelShader(ps_bytecode, None)? };
        println!("pixel_shader: {:?}", pixel_shader);

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
                AlignedByteOffset: 12u32,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0u32,
            },
        ];

        let d3d_input_layout = unsafe { d3d_device.CreateInputLayout(&layout, &vs_bytecode)? };

        unsafe { d3d_device_context.IASetInputLayout(&d3d_input_layout) };

        Ok((vertex_shader, pixel_shader))
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
    pub fn draw(&mut self, rtv: ID3D11RenderTargetView) -> Result<()> {
        // Try and acquire sync on common display buffer
        unsafe { self.dxgi_keyed_mutex.AcquireSync(1, 100)? };

        let mut frame_desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { self.shared_surface.GetDesc(&mut frame_desc) };

        let mut shader_desc = D3D11_SHADER_RESOURCE_VIEW_DESC::default();
        shader_desc.Format = frame_desc.Format;
        shader_desc.ViewDimension = D3D11_SRV_DIMENSION_TEXTURE2D;
        shader_desc.Anonymous.Texture2D.MostDetailedMip = frame_desc.MipLevels - 1;
        shader_desc.Anonymous.Texture2D.MipLevels = frame_desc.MipLevels;

        // Create new shader resource view
        let shader_resource = unsafe {
            self.d3d_device
                .CreateShaderResourceView(&self.shared_surface, &shader_desc)?
        };

        // Set resources
        let blend_factor: [f32; 4] = [0f32, 0f32, 0f32, 0f32];
        unsafe {
            self.d3d_device_context.OMSetBlendState(
                None,
                &blend_factor as *const f32,
                0xffffffffu32,
            );
            self.d3d_device_context
                .OMSetRenderTargets(&[Some(rtv)], None);
            self.d3d_device_context
                .VSSetShader(&self.vs_shader, &[None]);
            self.d3d_device_context
                .PSSetShader(&self.ps_shader, &[None]);
            self.d3d_device_context
                .PSSetShaderResources(0, &[Some(shader_resource)]);
            self.d3d_device_context.PSSetSamplers(0, &self.d3d_samplers);
            self.d3d_device_context
                .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        }

        let mut buffer_desc = D3D11_BUFFER_DESC::default();
        buffer_desc.Usage = D3D11_USAGE_DEFAULT;
        buffer_desc.ByteWidth = (std::mem::size_of::<VERTEX>() * NUMVERTICES) as u32;
        buffer_desc.BindFlags = D3D11_BIND_VERTEX_BUFFER.0;
        buffer_desc.CPUAccessFlags = 0;

        // Vertices for drawing whole texture
        let vertices: [VERTEX; NUMVERTICES] = [
            VERTEX {
                pos: [-1f32, -1f32, 0f32].into(),
                tex_coord: [0f32, 1f32].into(),
            },
            VERTEX {
                pos: [-1f32, 1f32, 0f32].into(),
                tex_coord: [0f32, 0f32].into(),
            },
            VERTEX {
                pos: [1f32, -1f32, 0f32].into(),
                tex_coord: [1f32, 1f32].into(),
            },
            VERTEX {
                pos: [1f32, -1f32, 0f32].into(),
                tex_coord: [1f32, 1f32].into(),
            },
            VERTEX {
                pos: [-1f32, 1f32, 0f32].into(),
                tex_coord: [0f32, 0f32].into(),
            },
            VERTEX {
                pos: [1f32, 1f32, 0f32].into(),
                tex_coord: [1f32, 0f32].into(),
            },
        ];

        let mut init_data = D3D11_SUBRESOURCE_DATA::default();
        init_data.pSysMem = vertices.as_ptr() as *const std::ffi::c_void;

        // Create vertex buffer
        let vertex_buffer = unsafe {
            self.d3d_device
                .CreateBuffer(&buffer_desc, &init_data)
                .unwrap()
        };

        let stride = std::mem::size_of::<VERTEX>() as u32;
        let offset = 0;

        unsafe {
            self.d3d_device_context.IASetVertexBuffers(
                0,
                1,
                &Some(vertex_buffer),
                &stride,
                &offset,
            );

            // Draw textured quad onto render target
            self.d3d_device_context.Draw(NUMVERTICES as u32, 0);

            // Release keyed mutex
            self.dxgi_keyed_mutex.ReleaseSync(0)?;
        }

        Ok(())
    }
}
