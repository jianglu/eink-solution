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
use ntapi::winapi::um::handleapi::DuplicateHandle;
use win_desktop_duplication::*;
use win_desktop_duplication::{devices::*, tex_reader::*};

use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::Win32::Graphics::Dxgi::IDXGIKeyedMutex;
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessId, OpenProcess};
use zenoh::config::Config;
use zenoh::query::{QueryTarget, Target};
use zenoh::queryable::EVAL;
use zenoh::{prelude::*, queryable};

fn main() -> Result<()> {
    // this is required to be able to use desktop duplication api
    set_process_dpi_awareness();
    co_init();

    println!("Opening IPC Session...");
    let config = Config::default();
    let session = zenoh::open(config).wait().expect("Cannot open ipc session");

    let ps_id = unsafe { GetProcessId(GetCurrentProcess()) };
    println!("GetProcessId: {:?}", ps_id);

    // OpenProcess(__in DWORD dwDesiredAccess,__in BOOL bInheritHandle,__in DWORD dwProcessId);

    let selector = format!("/SurfaceFlinger/CreateBufferQueue?ps={}", ps_id);
    println!("Sending Query '{}'...", selector);

    let target = QueryTarget {
        kind: queryable::EVAL,
        target: Target::All,
    };

    let replies = session
        .get(&selector)
        .target(target)
        .wait()
        .expect("Cannot get");

    let result = replies.recv();

    if let Ok(reply) = result {
        let val = String::from_utf8_lossy(&reply.sample.value.payload.contiguous()).to_string();
        let handle = val.parse::<isize>()?;
        println!(
            ">> Received ('{}': '{}', {})",
            reply.sample.key_expr.as_str(),
            val,
            handle
        );

        Ok(HANDLE(handle))
    } else {
        bail!("Error: {:?}", result.err().unwrap())
    }
}

fn main() -> Result<()> {
    // this is required to be able to use desktop duplication api
    set_process_dpi_awareness();
    co_init();

    // let surface_handle = get_shared_handle()?;

    // select gpu and output you want to use.
    let adapter = AdapterFactory::new().get_adapter_by_idx(0).unwrap();
    let output = adapter.get_display_by_idx(0).unwrap();

    // get output duplication api
    let mut dupl = DesktopDuplicationApi::new(adapter, output.clone()).unwrap();

    // Optional: get TextureReader to read GPU textures into CPU.
    let (device, ctx) = dupl.get_device_and_ctx();
    // let mut texture_reader = TextureReader::new(device, ctx);

    let surface_name = HSTRING::from("Surface-0");
    let texture2d = unsafe {
        device.OpenSharedResourceByName::<&HSTRING, ID3D11Texture2D>(&surface_name, 0x80000001)?
    };
    println!("Shared Texture: {:?}", texture2d);

    let tex2d_keyed_mutex = texture2d.cast::<IDXGIKeyedMutex>()?;
    println!("tex2d_keyed_mutex({:?})", tex2d_keyed_mutex);

    // draw_ctx = DrawingContext::from_device(device);
    // draw_ctx.set_render_target(rtv);
    // draw_ctx.draw_texture2d(d3d_texture_2d, 0, 0);

    // create a vector to hold picture data;
    let mut pic_data: Vec<u8> = vec![0; 0];

    loop {
        // this api send one frame per vsync. the frame also has cursor pre drawn
        output.wait_for_vsync().unwrap();

        let tex = dupl.acquire_next_frame_now();

        if let Ok(tex) = tex {
            let d3d_texture2d = tex.as_raw_ref();

            // texture_reader.get_data(&mut pic_data, &tex);
            // use pic_data as necessary
            println!("d3d_texture2d: {:?}", d3d_texture2d);

            unsafe {
                tex2d_keyed_mutex.AcquireSync(0, 10).unwrap();

                ctx.CopyResource(&texture2d, d3d_texture2d);

                tex2d_keyed_mutex.ReleaseSync(0).unwrap();
            }
        }
    }

    // Ok(())
}
