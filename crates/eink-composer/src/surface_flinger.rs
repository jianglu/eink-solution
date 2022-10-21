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

use std::collections::HashMap;
use std::hash::Hash;
use std::io::Write;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{ffi::OsStr, sync::mpsc::Receiver};

use anyhow::Result;
use log::info;

use ntapi::winapi::um::errhandlingapi::GetLastError;
use ntapi::winapi::um::minwinbase::STILL_ACTIVE;
use parking_lot::Once;
use windows::Win32::Graphics::{
    Direct3D11::{
        ID3D11Device, ID3D11DeviceContext, ID3D11InputLayout, D3D11_APPEND_ALIGNED_ELEMENT,
        D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD,
        D3D11_BLEND_SRC_ALPHA, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_ALWAYS,
        D3D11_CULL_NONE, D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
        D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_RASTERIZER_DESC,
        D3D11_RENDER_TARGET_BLEND_DESC, D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_BORDER,
        D3D11_VIEWPORT,
    },
    Dxgi::Common::{DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_R32G32_FLOAT},
};
use windows::Win32::System::Threading::GetExitCodeProcess;

use eink_composer_lib::{SurfaceComposerRequest, SurfaceComposerResponse};

use crate::layer::Layer;
use crate::utility::pid_to_handle;
use crate::{expect, pc_str, shader::CompiledShaders, swap_chain::SwapChain};

static START: Once = Once::new();

pub struct SurfaceFlinger {
    swap_chain: SwapChain,
    input_layout: ID3D11InputLayout,

    // IPC
    sock: nng::Socket,
    conn_socks: HashMap<u32, nng::Socket>,
    // queryable: Queryable<'a>,
    // pipe: PipeListener<DuplexMsgPipeStream>,
    // pipe_rx: Receiver<DuplexMsgPipeStream>,
    // pipe_conns: Vec<DuplexMsgPipeStream>,
    // pipe_thread: JoinHandle<()>,
    layers: Vec<Layer>,
    test_background: bool,
    // rx_set: IpcReceiverSet,
    // tx: Sender<SurfaceComposerRequest>,
    // rx: Receiver<SurfaceComposerRequest>,
}

impl SurfaceFlinger {
    const INPUT_ELEMENTS_DESC: [D3D11_INPUT_ELEMENT_DESC; 3] = [
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: pc_str!("POSITION"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: pc_str!("TEXCOORD"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: pc_str!("COLOR"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
    ];

    pub fn new(test_background: bool, test_layer: bool, monitor_id: &str) -> Result<Self> {
        // "MS_00022_00_07D2_69"
        // GBR01560_21_07E3_EF
        // TSB88882290649088_1C_07DB_69
        // MS_00011_00_07D2_D6
        // MS_00011_00_07D2_D6

        info!("monitor_id: {}", monitor_id);

        let swap_chain = SwapChain::new(monitor_id)?;
        let (dev, ctx) = swap_chain.get_device_and_context()?;

        let shaders = CompiledShaders::new(dev);
        let input_layout = expect!(
            unsafe { dev.CreateInputLayout(&Self::INPUT_ELEMENTS_DESC, shaders.bytecode()) },
            "Failed to create input layout"
        );

        info!("Opening IPC Session...");

        let sock = nng::Socket::new(nng::Protocol::Rep0)?;
        // sock.listen("ipc://surface-composer")?;

        let conn_socks = HashMap::new();

        // let pipe = PipeListenerOptions::new()
        //     .name(OsStr::new("SurfaceComposer"))
        //     .accept_remote(false)
        //     .mode(PipeMode::Messages)
        //     .instance_limit(instance_limit)
        //     .create::<DuplexMsgPipeStream>()?;

        // let (pipe_tx, pipe_rx) = channel::<DuplexMsgPipeStream>();

        // let pipe_thread = std::thread::spawn(move || loop {
        //     info!("Try accept");
        //     match pipe.accept() {
        //         Ok(conn) => {
        //             info!("New connection !!!");
        //             conn.set_nonblocking(true).unwrap();
        //             pipe_tx.send(conn).unwrap();
        //         }
        //         Err(_) => {
        //             std::thread::sleep(std::time::Duration::from_millis(10));
        //         }
        //     }
        // });

        // let pipe_conns = Vec::new();

        // let endpoint = EndPoint::try_from("tcp/127.0.0.1:7447".to_string()).unwrap();
        // let mut config = zenoh::config::Config::default();
        // config
        //     .set_listen(ListenConfig {
        //         endpoints: vec![endpoint],
        //     })
        //     .unwrap();
        // config.set_mode(Some(WhatAmI::Peer)).unwrap();
        // let session = Arc::new(zenoh::open(config).wait().expect("Cannot open ipc session"));

        // let key_expr = "/SurfaceComposer/*".to_string();
        // info!("Creating Queryable on '{}'...", key_expr);
        // let queryable = session
        //     .queryable(&key_expr)
        //     .kind(EVAL)
        //     .wait()
        //     .expect("Cannot create queryable");

        ///// D2D
        // let dxgi_device = dev.cast::<IDXGIDevice>()?;
        // info!("IDXGIDevice: {:?}", dxgi_device);

        // let mut ppsurface: [Option<IDXGISurface>; 3] = Default::default();
        // let desc = DXGI_SURFACE_DESC {
        // };

        // dxgi_device.CreateSurface(&desc, DXGI_USAGE, null(), &mut ppsurface)

        let mut layers = Vec::<Layer>::with_capacity(4);

        if test_layer {
            info!("Create test_layer");
            // 2560,1600
            // 1941,1600
            let layer = Layer::new(0, dev, ctx, 0, 0, 2941, 1600, 2941, 1600, true)?;
            layers.push(layer);
        }

        // let (tx, rx) = std::sync::mpsc::channel();

        Ok(Self {
            swap_chain,
            input_layout,
            sock,
            conn_socks,
            // pipe,
            // pipe_rx,
            // pipe_conns,
            // pipe_thread,
            layers,
            // rx_set,
            // tx,
            // rx,
            test_background,
        })
    }

    // pub fn get_tx(&self) -> Result<Sender<SurfaceComposerRequest>> {
    //     Ok(self.tx.clone())
    // }

    pub fn run(&mut self) -> Result<()> {
        loop {
            self.do_events()?;
            self.do_render()?;
        }
    }

    pub fn do_events(&mut self) -> Result<()> {
        // 接受 NewConnection 消息
        match self.sock.try_recv() {
            Ok(msg) => {
                let req = bincode::deserialize::<SurfaceComposerRequest>(&msg)?;
                info!("try_recv: req: {:?}", req);

                if let SurfaceComposerRequest::NewConnection { pid, url } = req {
                    info!("NewConnection: {}, {}", pid, &url);
                    let conn_sock = nng::Socket::new(nng::Protocol::Pair0)?;
                    conn_sock.dial(&url)?;

                    let req = SurfaceComposerResponse::NewConnectionResponse;
                    let req_bin = bincode::serialize(&req)?;
                    conn_sock.send(&req_bin).unwrap();

                    self.conn_socks.insert(pid, conn_sock);
                }
            }
            Err(nng::Error::TryAgain) => {
                // self.layers.retain(|l| !l.name.eq_ignore_ascii_case(url));
            }
            Err(err) => {
                info!("self.sock.try_recv({:?})", err);
            }
        }

        // let mut drop_urls: Vec<String> = Vec::new();
        let msg = (|| {
            for (pid, sock) in self.conn_socks.iter() {
                let ret = sock.try_recv();
                match ret.as_ref() {
                    Ok(_) => return Some((*pid, ret)),
                    Err(&nng::Error::TryAgain) => {
                        let hprocess = pid_to_handle(*pid).unwrap();
                        let mut exit_code: u32 = 0;
                        let result = unsafe {
                            GetExitCodeProcess(hprocess, &mut exit_code as *const u32 as *mut u32)
                        };
                        if exit_code == STILL_ACTIVE {
                            // info!("TryAgain 1, pid: {:?} STILL_ACTIVE", *pid);
                            continue;
                        } else {
                            // let last_error = unsafe { GetLastError() };
                            info!(
                                "TryAgain 2, pid: {:?} BOOL: {:?}, ExitCode: {}",
                                *pid, result, exit_code
                            );
                            return Some((*pid, Err(nng::Error::Closed)));
                        }
                    }
                    Err(_) => {
                        info!("conn try_recv: err: {:?}", &ret.as_ref().err());
                        return Some((*pid, ret));
                    }
                }
            }
            None
        })();

        // 没有任何消息需要处理
        if msg.is_none() {
            return Ok(());
        }

        // PID 和 Result
        let (pid, ret) = msg.unwrap();

        // 发生错误，关闭对应的 Layer
        if ret.is_err() {
            info!("ERROR: pid: {}, err: {:?}", pid, ret.as_ref().err());
            self.conn_socks.retain(|s, _| *s != pid);
            self.layers.retain(|l| l.pid != pid);
            return Ok(());
        }

        // 处理不同的消息
        match ret {
            Ok(msg) => {
                let req = bincode::deserialize::<SurfaceComposerRequest>(&msg).unwrap();
                match req {
                    SurfaceComposerRequest::CreateSurfaceRequest {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        info!(
                            "SurfaceComposerRequest::CreateSurfaceRequest({}, {}, {}, {})",
                            x, y, width, height
                        );

                        let h = self.create_surface(pid, x, y, width, height).unwrap();

                        let rep =
                            SurfaceComposerResponse::CreateSurfaceResponse { texture_name: h };
                        let rep_bin = bincode::serialize(&rep).unwrap();

                        let sock = self.conn_socks.get(&pid).unwrap();
                        sock.send(&rep_bin).unwrap();
                    }
                    // SurfaceComposerRequest::ReleaseSurfaceRequest {
                    // } => {

                    // }
                    SurfaceComposerRequest::MoveSurfaceRequest {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        info!(
                            "SurfaceComposerRequest::MoveSurfaceRequest({}, {}, {}, {})",
                            x, y, width, height
                        );

                        if let Some(layer) = self.layers.iter_mut().find(|l| l.pid == pid) {
                            layer.set_bounds(x, y, width, height);
                        }

                        let rep = SurfaceComposerResponse::MoveSurfaceResponse;
                        let rep_bin = bincode::serialize(&rep).unwrap();

                        let sock = self.conn_socks.get(&pid).unwrap();
                        sock.send(&rep_bin).unwrap();
                    }
                    _ => (),
                }
            }
            Err(nng::Error::TryAgain) => {
                // info!("conn try_recv: TryAgain");
                // self.layers.retain(|l| !l.name.eq_ignore_ascii_case(url));
            }
            Err(nng::Error::Closed) => {
                // 连接关闭，释放表面
                info!("Conn closed !!, Release Surface !!!!!!!!!!");
                // self.layers.retain(|l| !l.name.eq_ignore_ascii_case(url));
            }
            Err(err) => {
                info!("conn try_recv: err: {:?}", err);
                //
            }
        }

        // for url in  {
        // }

        // // 接收新连接
        // if let Ok(conn) = self.pipe_rx.try_recv() {
        //     let client_pid = conn.client_process_id()?;
        //     info!("New connection client_pid: {}", client_pid);
        //     self.pipe_conns.push(conn);
        // }
        // // info!("do_events START");

        // let mut msg_vec = [0u8; 1024];

        // self.pipe_conns
        //     .retain_mut(|conn| match conn.try_read_msg(&mut msg_vec) {
        //         Ok(ret) => match ret {
        //             Ok(size) => {
        //                 info!("read_msg: size: {:?}", size);
        //                 let reply = SurfaceComposerResponse::CreateSurfaceResponse {
        //                     texture_name: "11111".to_string(),
        //                 };
        //                 let reply_buf = bincode::serialize(&reply).unwrap();
        //                 conn.write_all(&reply_buf).unwrap();
        //                 conn.flush().unwrap();
        //                 true
        //             }
        //             Err(os_errcode) => {
        //                 info!("read_msg: err_code: {:?}", os_errcode);
        //                 false
        //             }
        //         },
        //         Err(err) => {
        //             info!("read_msg: err: {:?}", err);
        //             false
        //         }
        //     });

        // for (i, conn) in self.pipe_conns.iter_mut().enumerate() {
        //     let mut msg_vec = [0u8; 1024];

        //     match conn.try_read_msg(&mut msg_vec) {
        //         Ok(ret) => match ret {
        //             Ok(size) => {
        //                 info!("read_msg: size: {:?}", size);
        //             }
        //             Err(os_errcode) => {
        //                 info!("read_msg: err_code: {:?}", os_errcode);
        //             }
        //         },
        //         Err(err) => {
        //             info!("read_msg: err: {:?}", err);
        //             self.pipe_conns.remove(i);
        //         }
        //     }
        // }

        // info!("do_events END");

        // if let Ok(query) = self.queryable.try_recv() {
        //     let selector = query.selector();

        //     let key_selector = selector.key_selector.as_str();
        //     let value_selector = selector.parse_value_selector().unwrap();

        //     info!("key_selector: {}", key_selector);
        //     info!("filter: {}", value_selector.filter);
        //     info!("properties: {}", value_selector.properties);
        //     info!("fragment: {:?}", value_selector.fragment);

        //     if key_selector == "/SurfaceComposer/CreateSurface" {
        //         let s = base64::decode(value_selector.filter.as_bytes())?;
        //         let s = serde_json::from_slice::<CreateSurfaceRequest>(&s)?;
        //         info!("{},{},{},{}", s.x, s.y, s.width, s.height);

        //         let h = self.create_surface(s.x, s.y, s.width, s.height)?;

        //         let r = CreateSurfaceResponse {
        //             shared_texture_name: h,
        //         };
        //         let s = serde_json::to_string(&r)?;
        //         let s = base64::encode(&s);
        //         query.reply(Sample::new("Reply", s));
        //     }
        // }
        Ok(())
    }

    // 雅黑字体
    pub fn do_render(&mut self) -> Result<()> {
        self.swap_chain.pre_present(self.test_background)?;

        // let (dev, ctx) = self.swap_chain.get_device_and_context()?;
        let rtv = self.swap_chain.get_render_target()?;

        // ctx.ClearRenderTargetView(rtv, [0.39, 0.58, 0.92, 1.].as_ptr());

        // self.set_blend_state(dev, ctx);
        // self.set_raster_options(dev, ctx);
        // self.set_sampler_state(dev, ctx);

        // ctx.RSSetViewports(&[self.get_viewport()]);
        // ctx.OMSetRenderTargets(&[Some(rtv.clone())], None);
        // ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        // ctx.IASetInputLayout(&self.input_layout);

        // for mesh in primitives {
        //     let idx = create_index_buffer(dev, &mesh);
        //     let vtx = create_vertex_buffer(dev, &mesh);

        //     let texture = this.tex_alloc.get_by_id(mesh.texture_id);

        //     ctx.RSSetScissorRects(&[RECT {
        //         left: mesh.clip.left() as _,
        //         top: mesh.clip.top() as _,
        //         right: mesh.clip.right() as _,
        //         bottom: mesh.clip.bottom() as _,
        //     }]);

        //     if texture.is_some() {
        //         ctx.PSSetShaderResources(0, &[texture]);
        //     }

        //     ctx.IASetVertexBuffers(0, 1, &Some(vtx), &(size_of::<GpuVertex>() as _), &0);
        //     ctx.IASetIndexBuffer(idx, DXGI_FORMAT_R32_UINT, 0);
        //     ctx.VSSetShader(&this.shaders.vertex, &[]);
        //     ctx.PSSetShader(&this.shaders.pixel, &[]);

        //     ctx.DrawIndexed(mesh.indices.len() as _, 0, 0);
        // }

        {
            for (_i, layer) in self.layers.iter_mut().enumerate() {
                layer.draw(rtv)?;
            }
        }

        self.swap_chain.present()?;

        Ok(())
    }

    #[inline]
    fn get_viewport(&self) -> D3D11_VIEWPORT {
        let (w, h) = self.swap_chain.get_screen_size();
        D3D11_VIEWPORT {
            TopLeftX: 0.,
            TopLeftY: 0.,
            Width: w,
            Height: h,
            MinDepth: 0.,
            MaxDepth: 1.,
        }
    }

    /// Sets the blend state of this [`SurfaceFlinger`].
    fn set_blend_state(&self, dev: &ID3D11Device, ctx: &ID3D11DeviceContext) {
        let mut targets: [D3D11_RENDER_TARGET_BLEND_DESC; 8] = Default::default();
        targets[0].BlendEnable = true.into();
        targets[0].SrcBlend = D3D11_BLEND_SRC_ALPHA;
        targets[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
        targets[0].BlendOp = D3D11_BLEND_OP_ADD;
        targets[0].SrcBlendAlpha = D3D11_BLEND_ONE;
        targets[0].DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;
        targets[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
        targets[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL.0 as _;

        let blend_desc = D3D11_BLEND_DESC {
            AlphaToCoverageEnable: false.into(),
            IndependentBlendEnable: false.into(),
            RenderTarget: targets,
        };

        unsafe {
            let blend_state = expect!(
                dev.CreateBlendState(&blend_desc),
                "Failed to create blend state"
            );
            let blend_factor = [0., 0., 0., 0.].as_ptr();
            ctx.OMSetBlendState(&blend_state, blend_factor, 0xffffffff);
        }
    }

    /// Sets the raster options of this [`SurfaceFlinger`].
    fn set_raster_options(&self, dev: &ID3D11Device, ctx: &ID3D11DeviceContext) {
        let raster_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            FrontCounterClockwise: false.into(),
            DepthBias: false.into(),
            DepthBiasClamp: 0.,
            SlopeScaledDepthBias: 0.,
            DepthClipEnable: false.into(),
            ScissorEnable: true.into(),
            MultisampleEnable: false.into(),
            AntialiasedLineEnable: false.into(),
        };

        unsafe {
            let options = expect!(
                dev.CreateRasterizerState(&raster_desc),
                "Failed to create rasterizer state"
            );
            ctx.RSSetState(&options);
        }
    }

    /// Sets the sampler state of this [`SurfaceFlinger`].
    fn set_sampler_state(&self, dev: &ID3D11Device, ctx: &ID3D11DeviceContext) {
        let desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_BORDER,
            AddressV: D3D11_TEXTURE_ADDRESS_BORDER,
            AddressW: D3D11_TEXTURE_ADDRESS_BORDER,
            MipLODBias: 0.,
            ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            MinLOD: 0.,
            MaxLOD: 0.,
            BorderColor: [1., 1., 1., 1.],
            ..Default::default()
        };

        unsafe {
            let sampler = expect!(dev.CreateSamplerState(&desc), "Failed to create sampler");
            ctx.PSSetSamplers(0, &[Some(sampler)]);
        }
    }

    pub(crate) fn init(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn create_surface(
        &mut self,
        pid: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<String> {
        let (dev, ctx) = self.swap_chain.get_device_and_context()?;
        let resolution = self.swap_chain.get_resolution()?;
        let layer = Layer::new(
            pid,
            dev,
            ctx,
            x,
            y,
            width,
            height,
            resolution.Width,
            resolution.Height,
            false,
        )?;

        let shared_name = layer.tex2d_shared_name.to_string();

        self.layers.push(layer);

        Ok(shared_name)
    }
}
