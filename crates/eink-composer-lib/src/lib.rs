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

use std::{io::Write, mem::size_of, sync::Arc};

use anyhow::bail;
use base64::encode_config;
use nng::options::Options;
// use interprocess::{os::windows::named_pipe::DuplexMsgPipeStream, ReliableReadMsg};
// use ipc_channel::ipc::IpcSender;
use serde::{Deserialize, Serialize};
use windows::Win32::System::Threading::GetCurrentProcessId;
// use zenoh::{
//     config::{whatami::WhatAmI, EndPoint, ListenConfig},
//     prelude::{Locator, Receiver, Sample, SplitBuffer},
//     queryable::Queryable,
//     sync::ZFuture,
// };

pub type AnyResult<T> = anyhow::Result<T>;

// #[derive(Debug, Deserialize, Serialize)]
// pub enum SurfaceComposerError {
//     Type(String),
//     Network,
//     NotFound,
//     NotSupported,
//     Security,
//     InvalidState,
// }

// pub type SurfaceComposerResponseResult = Result<SurfaceComposerResponse, SurfaceComposerError>;

// 请求消息
#[derive(Debug, Deserialize, Serialize)]
pub enum SurfaceComposerRequest {
    NewConnection {
        pid: u32,
        url: String,
    },
    CreateSurfaceRequest {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    // ReleaseSurfaceRequest {
    // },
    MoveSurfaceRequest {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
}

// 请求消息
#[derive(Debug, Deserialize, Serialize)]
pub enum SurfaceComposerResponse {
    NewConnectionResponse,
    CreateSurfaceResponse { texture_name: String },
    MoveSurfaceResponse,
}

// DWORD returnCode{};
// if (GetExitCodeProcess(handle, &returnCode)) {

// /// 创建表面
// #[derive(Debug, Deserialize, Serialize)]
// pub struct CreateSurfaceRequest {
//     pub x: i32,
//     pub y: i32,
//     pub width: i32,
//     pub height: i32,
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct CreateSurfaceResponse {
//     // 共享材质名称
//     pub shared_texture_name: String,
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub enum SurfaceComposerResponse {
//     CreateSurface(CreateSurfaceResponsePayload),
// }

/// 易于操作的表面对象
pub struct Surface {
    pub shared_texture_name: String,
}

impl Surface {
    /// 创建表面
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub(crate) fn new(shared_texture_name: String) -> anyhow::Result<Self> {
        Ok(Self {
            shared_texture_name,
        })
    }
}

pub struct SurfaceComposerClient {
    // session: zenoh::Session,
    // pipe_conn: DuplexMsgPipeStream,
    sock: nng::Socket,
    // peer_sock: nng::Socket,
}

impl SurfaceComposerClient {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn new() -> AnyResult<Self> {
        println!("Open connection ...");

        let server_sock = nng::Socket::new(nng::Protocol::Req0)?;
        server_sock.dial("ipc://surface-composer")?;

        let pid = unsafe { GetCurrentProcessId() };
        let url = format!("ipc://pid/{}", pid);

        let sock = nng::Socket::new(nng::Protocol::Pair0)?;
        sock.listen(&url)?;
        sock.set_opt::<nng::options::RecvTimeout>(Some(std::time::Duration::from_secs(5)))?;

        // 发送创建链接消息
        let req = SurfaceComposerRequest::NewConnection { pid, url };
        let req_bin = bincode::serialize(&req)?;
        server_sock.send(&req_bin).unwrap();
        // server_sock.close();

        let rep = match sock.recv() {
            Ok(m) => bincode::deserialize::<SurfaceComposerResponse>(&m)?,
            Err(e) => bail!(e),
        };

        println!("New connection: {:?}", req);

        Ok(Self { sock })
    }

    /// 通过 IPC 通知 SurfaceComposer 创建 Surface
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn create_surface(&mut self, x: i32, y: i32, w: i32, h: i32) -> anyhow::Result<Surface> {
        // Mutex::Autolock

        println!("Create surface ...");

        // 发送创建表面消息
        let req = SurfaceComposerRequest::CreateSurfaceRequest {
            x,
            y,
            width: w,
            height: h,
        };

        let req_bin = bincode::serialize(&req)?;

        match self.sock.send(&req_bin) {
            Ok(_) => (),
            Err(e) => bail!("{:?}: {:?}", e.0, e.1),
        };

        let rep = match self.sock.recv() {
            Ok(m) => bincode::deserialize::<SurfaceComposerResponse>(&m)?,
            Err(e) => bail!(e),
        };

        // let size = size_of::<SurfaceComposerResponse>();

        // // 从 SurfaceComposer 读取返回消息
        // let mut res_vec = Vec::with_capacity(size);
        // unsafe { res_vec.set_len(size) };
        // let ret = self.pipe_conn.read_msg(&mut res_vec)?;

        // match ret {
        //     Ok(len) => {
        //         println!("len: {}", len);
        //         res_vec.truncate(len)
        //     }
        //     Err(err) => {
        //         bail!(
        //             "Cannot read response message from SurfaceComposer: {:?}",
        //             err
        //         )
        //     }
        // }

        // let resp = bincode::deserialize::<SurfaceComposerResponse>(&res_vec)?;

        if let SurfaceComposerResponse::CreateSurfaceResponse { texture_name } = rep {
            Ok(Surface::new(texture_name)?)
        } else {
            bail!("Unknown SurfaceComposer Response")
        }

        // let target = zenoh::query::QueryTarget {
        //     kind: zenoh::queryable::EVAL,
        //     target: zenoh::query::Target::All,
        // };

        // let req = CreateSurfaceRequest {
        //     x,
        //     y,
        //     width: w,
        //     height: h,
        // };

        // let s = serde_json::to_string(&req).unwrap();
        // let s = base64::encode(s);

        // let replies = self
        //     .session
        //     .get(format!("/SurfaceComposer/CreateSurface?{}", &s))
        //     .target(target)
        //     .wait()
        //     .expect("Cannot get");

        // let result = replies.recv();

        // if let Ok(reply) = result {
        //     let val = String::from_utf8_lossy(&reply.sample.value.payload.contiguous()).to_string();

        //     let s = base64::decode(&val)?;
        //     let s = serde_json::from_slice::<CreateSurfaceResponse>(&s)?;

        //     println!(
        //         ">> Received ('{}': shared_texture_name: '{}')",
        //         reply.sample.key_expr.as_str(),
        //         s.shared_texture_name,
        //     );

        //     Ok(Surface::new(s.shared_texture_name)?)
        // } else {
        //     bail!("Error: {:?}", result.err().unwrap())
        // }
    }

    /// 缩放表面至新的大小
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn move_surface(
        &mut self,
        surface: &mut Surface,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> anyhow::Result<()> {
        println!("Move surface ...");

        // 发送创建表面消息
        let req = SurfaceComposerRequest::MoveSurfaceRequest {
            x,
            y,
            width,
            height,
        };

        let req_bin = bincode::serialize(&req)?;

        match self.sock.send(&req_bin) {
            Ok(_) => (),
            Err(e) => bail!("{:?}: {:?}", e.0, e.1),
        };

        let rep = match self.sock.recv() {
            Ok(m) => bincode::deserialize::<SurfaceComposerResponse>(&m)?,
            Err(e) => bail!(e),
        };

        Ok(())
    }
}

// struct IpcServer<'a> {
//     queryable: Queryable<'a>,
// }

// impl<'a> IpcServer<'a> {
//     /// .
//     ///
//     /// # Errors
//     ///
//     /// This function will return an error if .
//     pub fn new() -> anyhow::Result<Self> {
//         let config = zenoh::config::Config::default();
//         config.set_mode(Some(WhatAmI::Peer));

//         let session = Arc::new(zenoh::open(config).wait().expect("Cannot open ipc session"));

//         let key_expr = "/SurfaceFlinger/*".to_string();

//         println!("Creating Queryable on '{}'...", key_expr);

//         let queryable = session
//             .queryable(&key_expr)
//             .kind(zenoh::queryable::EVAL)
//             .wait()
//             .expect("Cannot create queryable");

//         ///// D2D
//         Ok(Self { queryable })
//     }

//     pub fn try_recv(&mut self) {
//         if let Ok(query) = self.queryable.try_recv() {
//             query.reply(Sample::new(key_expr, value))
//         }
//     }
// }

// pub struct IpcClient {
//     key: String,
// }

// impl IpcClient {
//     /// .
//     ///
//     /// # Errors
//     ///
//     /// This function will return an error if .
//     pub fn new(key: &str) -> anyhow::Result<Self> {
//         Ok(Self {})
//     }
// }
