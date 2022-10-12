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

// draw_ctx = DrawingContext::from_device(device);
// draw_ctx.set_render_target(rtv);
// draw_ctx.draw_texture2d(d3d_texture_2d, 0, 0);

use anyhow::{bail, Result};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Texture2D,
};

pub fn draw_texture2d(
    device: &ID3D11Device,
    ctx: &ID3D11DeviceContext,
    render_target: &ID3D11RenderTargetView,
    tex2d: &ID3D11Texture2D,
) -> Result<()> {
    bail!("Cannot draw texture2d")
}

pub fn load_texture2d(
    device: &ID3D11Device,
    ctx: &ID3D11DeviceContext,
    path: String,
) -> Result<ID3D11Texture2D> {
    bail!("Cannot load image")
}
