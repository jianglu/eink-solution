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

use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    Arc, Mutex, RwLock,
};

use anyhow::{bail, Result};
use eink_eventbus::Event;
use event_listener_primitives::{Bag, BagOnce, HandlerId};
use futures::{SinkExt, StreamExt};
use jsonrpc_lite::{ErrorCode, JsonRpc, Params};
use log::{error, info};
use pipe_ipc::{blocking::BlockingIpcConnection, Endpoint, SecurityAttributes};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::Sender,
};
use tokio_util::codec::Decoder;

use tokio::sync::mpsc::channel as tokio_channel;

use crate::global::{CaptureWindowMessage, RegModeUpdateMessage, EVENTBUS, GENERIC_TOPIC_KEY};
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
        let running = Arc::new(AtomicBool::new(true));

        std::thread::spawn(move || {
            let server =
                pipe_ipc::blocking::BlockingIpcServer::new("\\\\.\\pipe\\eink-service").unwrap();

            server
                .on_connection(|conn| {
                    info!("IpcServer: On New Connection");

                    conn.on_request(|conn, request| {
                        let id = request.get_id().unwrap();
                        info!(
                            "IpcServer: On New Request: Id: {:?}, request: {:?}",
                            id, request
                        );

                        let method = request.get_method().unwrap();

                        Self::handle_request(conn, request);

                        // conn.reply_success(id, &serde_json::Value::Bool(true));
                        // conn.reply_error(id, jsonrpc_lite::Error::invalid_params());
                    })
                    .detach();

                    conn.on_notify(|conn, request| {
                        let id = request.get_id().unwrap();
                        info!(
                            "IpcServer: On New Request: Id: {:?}, request: {:?}",
                            id, request
                        );

                        let method = request.get_method().unwrap();

                        Self::handle_request(conn, request);

                        // conn.reply_success(id, &serde_json::Value::Bool(true));
                        // conn.reply_error(id, jsonrpc_lite::Error::invalid_params());
                    })
                    .detach();
                })
                .detach();

            info!("IpcServer: Start Listening");
            server.listen().unwrap();
            info!("IpcServer: End Listening");
        });

        Ok(Self { running })
    }

    pub fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn handle_request(conn: &BlockingIpcConnection, req: &JsonRpc) {
        info!("EinkService Received Request: {:?}", req);

        let id = req.get_id().unwrap();
        info!("id: {:?}", id);

        let method = req.get_method().unwrap();
        info!("method: {:?}", method);

        if method == "capture_window" {
            let params = req.get_params().unwrap();
            info!("params: {:?}", params);

            if let Params::Map(map) = params {
                if map.contains_key("hwnd") {
                    let hwnd = map.get("hwnd").unwrap().as_i64().unwrap();

                    info!("capture_window: hwnd: {:?}", hwnd);

                    conn.reply_success(id, &serde_json::Value::Bool(true));

                    // 将捕获消息发送至消息总线
                    EVENTBUS.post(&Event::new(
                        GENERIC_TOPIC_KEY.clone(),
                        CaptureWindowMessage {
                            hwnd: Some(HWND(hwnd as isize)),
                            cmdline: None,
                        },
                    ));
                } else if map.contains_key("cmdline") {
                    let cmdline = map.get("cmdline").unwrap().as_str().unwrap();

                    info!("capture_window: cmdline: {:?}", cmdline);

                    conn.reply_success(id, &serde_json::Value::Bool(true));

                    // 将捕获消息发送至消息总线
                    EVENTBUS.post(&Event::new(
                        GENERIC_TOPIC_KEY.clone(),
                        CaptureWindowMessage {
                            hwnd: None,
                            cmdline: Some(cmdline.to_string()),
                        },
                    ));
                }
            } else {
                info!("invalid_params");
                conn.reply_error(id, jsonrpc_lite::Error::invalid_params())
            }
        } else if method == "switch_mode" {
            let params = req.get_params().unwrap();
            info!("params: {:?}", params);

            if let Params::Map(map) = params {
                let mode = map.get("mode").unwrap().as_u64().unwrap() as u32;

                info!("switch_mode: mode: {:?}", mode);

                conn.reply_success(id, &serde_json::Value::Bool(true));

                std::thread::sleep(std::time::Duration::from_millis(100));

                // 将捕获消息发送至消息总线
                EVENTBUS.post(&Event::new(
                    GENERIC_TOPIC_KEY.clone(),
                    RegModeUpdateMessage { mode },
                ));
            } else {
                info!("invalid_params");
                conn.reply_error(id, jsonrpc_lite::Error::invalid_params())
            }
        } else {
            info!("invalid_request");
            conn.reply_error(id, jsonrpc_lite::Error::invalid_request())
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
