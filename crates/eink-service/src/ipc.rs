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

use std::{
    collections::HashMap,
    default,
    io::{BufRead, BufReader},
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex, RwLock,
    },
};

use anyhow::{bail, Result};
use eink_eventbus::Event;
use event_listener_primitives::{Bag, BagOnce, HandlerId};
use futures::{SinkExt, StreamExt};
use jsonrpc_lite::{ErrorCode, JsonRpc, Params};
use log::{error, info};
use pipe_ipc::{Endpoint, SecurityAttributes};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::Sender,
};
use tokio_util::codec::Decoder;

use tokio::sync::mpsc::channel as tokio_channel;

use crate::global::{CaptureWindowMessage, EVENTBUS, GENERIC_TOPIC_KEY};
use crate::winrt::HWND;
// #[derive(Deserialize, Debug)]
// struct IpcRequest {
//     method: String,
//     params: serde_json::Value,
//     id: u32,
// }

// #[derive(Serialize, Debug)]
// struct IpcResponse {
//     result: serde_json::Value,
//     id: u32,
// }

// type HandlerFn = dyn Fn(JsonRpc, JsonRpc) + Sync + Send;
// type HandlerMap = cht::map::HashMap<String, Arc<RwLock<HandlerFn>>>;

struct IpcServiceImpl {
    running: Arc<AtomicBool>,
    // handlers: HandlerMap,
}

impl IpcServiceImpl {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn new() -> Result<Self> {
        // let pipe = Pipe::with_name("eink-service")?;

        let running = Arc::new(AtomicBool::new(true));
        // let running_cloned = running.clone();

        // std::thread::spawn(move || {
        //     let mut reader = BufReader::new(pipe);
        //     let mut buf = String::new();
        //     while running_cloned.load(Relaxed) {
        //         match reader.read_line(&mut buf) {
        //             Ok(cnt) => {
        //                 info!("IPC Received: {} bytes", cnt);
        //                 match serde_json::from_slice::<IpcRequest>(buf.as_bytes()) {
        //                     Ok(req) => {
        //                         info!("req.method: {}", &req.method);
        //                         info!("req.params: {}", &req.params);
        //                     }
        //                     Err(err) => {
        //                         error!("IPC SERDE JSON ERR: {:?}", err);
        //                     }
        //                 }
        //             }
        //             Err(err) => {
        //                 error!("IPC ERR: {:?}", err);
        //             }
        //         }
        //     }
        // });

        // let handlers = HandlerMap::default();
        // let handlers_clone = handlers.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(16)
                .enable_all()
                .build()
                .expect("Cannot create tokio runtime for eink ipc service");
            rt.block_on(async { Self::run_server("\\\\.\\pipe\\eink-service").await });
        });

        Ok(Self { running })
    }

    // type HandlerFn = dyn Fn() + Send + Sync + 'static;

    // pub fn add_ipc_handler<F: Fn(JsonRpc, JsonRpc) + Send + Sync + 'static>(
    //     &mut self,
    //     method: String,
    //     callback: F,
    // ) -> Result<()> {
    //     if !self.handlers.contains_key(&method) {
    //         self.handlers.insert(method.clone(), Arc::new(callback));
    //         Ok(())
    //     } else {
    //         bail!("{} was already registered", method)
    //     }
    // }

    /// 在 Tokio 运行时上启动异步服务
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn start(&mut self) -> Result<()> {
        Ok(())
    }

    /// 处理每一个 IpcClient 的连接
    /// 连接的读写是复用的，需要支持多个请求异步返回的情况
    ///
    /// IPC1 Req --------------------------- IPC1 Reply
    /// ---------- IPC2 Req -- IPC2 Reply -------------
    ///
    /// TODO: 细化处理 IpcClient 的断开
    async fn new_connection(stream: impl AsyncRead + AsyncWrite) -> ! {
        // PIN 到栈上
        futures::pin_mut!(stream);

        // 连接转换为 Frame 帧结构处理模型
        let codec = pipe_ipc::Codec {};
        let mut conn = codec.framed(stream);

        let (tx, mut rx) = tokio_channel::<JsonRpc>(8);

        loop {
            tokio::select! {
                // 来自 IpcClient 连接请求
                Some(req) = conn.next() => {
                    Self::handle_request(req.unwrap(), &tx).await
                }

                // 来自系统其他模块的异步返回值请求
                Some(res) = rx.recv() => {
                    let msg = res;
                    info!("Response already: {:?}", &msg);
                    conn.send(msg).await.unwrap();
                }
            };
        }
    }

    async fn handle_request(req: JsonRpc, tx: &Sender<JsonRpc>) {
        info!("EinkService Received Request: {:?}", req);
        let id = req.get_id().unwrap();
        let method = req.get_method().unwrap();

        if method == "capture_window" {
            let params = req.get_params().unwrap();

            if let Params::Map(map) = params {
                let hwnd = map.get("hwnd").unwrap().as_i64().unwrap();

                // 将捕获消息发送至消息总线
                EVENTBUS.post(&Event::new(
                    GENERIC_TOPIC_KEY.clone(),
                    CaptureWindowMessage {
                        hwnd: HWND(hwnd as isize),
                    },
                ));

                let msg = JsonRpc::success(id, &serde_json::Value::Bool(true));
                tx.send(msg).await.unwrap();
            } else {
                let msg = JsonRpc::error(id, jsonrpc_lite::Error::invalid_params());
                tx.send(msg).await.unwrap();
            }
        } else {
            let msg = JsonRpc::error(id, jsonrpc_lite::Error::invalid_request());
            tx.send(msg).await.unwrap();
        }
    }

    async fn run_server(path: &str) {
        let mut endpoint = Endpoint::new(path);

        // 当前设置允许任意程序与 EinkSrv 通讯
        // TODO: 增加签名校验
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        let incoming = endpoint.incoming().expect("failed to open new socket");
        futures::pin_mut!(incoming);

        // 对于每一个新连接，创建独立的通讯任务
        while let Some(result) = incoming.next().await {
            match result {
                Ok(stream) => {
                    tokio::spawn(async move { Self::new_connection(stream).await });
                }
                _ => unreachable!("ideally"),
            }
        }
    }
}

impl Drop for IpcServiceImpl {
    fn drop(&mut self) {
        self.running.store(false, Relaxed)
    }
}

/// EINK 服务
/// 1. EINK 保活
/// 2. EINK 模式管理和切换
pub struct IpcService {
    inner: Arc<Mutex<IpcServiceImpl>>,
}

impl IpcService {
    /// 创建 EINK IPC 服务
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(IpcServiceImpl::new()?)),
        })
    }

    pub fn start(&self) -> Result<()> {
        let mut guard = self.inner.lock().unwrap();
        guard.start()
    }
}
