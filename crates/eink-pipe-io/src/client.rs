use std::{sync::Arc, time::Duration};

use anyhow::bail;
use jsonrpc_lite::{JsonRpc, Params};
use remoc::rch;
use signals2::{connect::ConnectionImpl, Connect2, Connection, Emit2, Signal};
use tokio::{net::windows::named_pipe::ClientOptions, sync::Mutex, time};
use windows::Win32::Foundation::ERROR_PIPE_BUSY;

use crate::msg::IpcMsg;

pub struct ClientHandlers {
    pub on_request: Signal<(i32, JsonRpc), JsonRpc>,
}

pub struct Client {
    pipe_name: String,
    handlers: Arc<Mutex<ClientHandlers>>,
    tx: Option<rch::base::Sender<IpcMsg>>,
}

impl Client {
    pub fn new(pipe_name: &str) -> Self {
        Self {
            pipe_name: pipe_name.to_string(),
            handlers: Arc::new(Mutex::new(ClientHandlers {
                on_request: Signal::new(),
            })),
            tx: None,
        }
    }

    /// 设置请求回调，使用 signals 接口
    pub async fn on_request<Callback>(&mut self, cb: Callback) -> Connection
    where
        Callback: Fn(i32, JsonRpc) -> JsonRpc + Send + Sync + 'static,
    {
        self.handlers.lock().await.on_request.connect(cb)
    }

    pub async fn call_with_params<P: Into<Params>>(
        &mut self,
        method: &str,
        params: P,
    ) -> anyhow::Result<JsonRpc> {
        let id = uuid::Uuid::new_v4().to_string();
        let (reply_tx, mut reply_rx) = rch::mpsc::channel(1);
        self.tx
            .as_mut()
            .unwrap()
            .send(IpcMsg {
                payload: JsonRpc::request_with_params(id, method, params),
                reply_tx: Some(reply_tx),
            })
            .await
            .unwrap();
        match reply_rx.recv().await {
            Ok(reply) => return Ok(reply.unwrap()),
            Err(err) => bail!(err),
        }
    }

    pub async fn connect(&mut self) {
        // 创建底层 pipe 连接
        let pipe_client = loop {
            match ClientOptions::new().open(&self.pipe_name) {
                Ok(client) => break client,
                Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32) => (),
                Err(e) => return,
            }

            time::sleep(Duration::from_millis(50)).await;
        };

        // 将 pipe 连接分离为 rx, tx
        let (pipe_rx, pipe_tx) = tokio::io::split(pipe_client);

        // 创建 Remoc 双向链路
        let (conn, tx, mut rx): (_, rch::base::Sender<IpcMsg>, rch::base::Receiver<IpcMsg>) =
            remoc::Connect::io(remoc::Cfg::default(), pipe_rx, pipe_tx)
                .await
                .unwrap();

        tokio::spawn(conn);

        self.tx = Some(tx);

        let handlers = self.handlers.clone();

        // 接收对端请求
        tokio::spawn(async move {
            Self::process_incoming(handlers, &mut rx).await;
        });
    }

    /// 处理输入的请求
    async fn process_incoming(
        handlers: Arc<Mutex<ClientHandlers>>,
        rx: &mut rch::base::Receiver<IpcMsg>,
    ) {
        loop {
            match rx.recv().await {
                Ok(received) => match received {
                    Some(rpc_msg) => match &rpc_msg.payload {
                        JsonRpc::Request(_) => {
                            let id = rpc_msg.payload.get_id().unwrap();

                            // Signal 的 clone 是轻量级操作
                            let on_request = { handlers.lock().await.on_request.clone() };

                            // 事件处理可能是耗时操作，分离到 blocking 线程进行
                            let blocking_res = tokio::task::spawn_blocking(move || {
                                on_request.emit(0, rpc_msg.payload)
                            })
                            .await
                            .unwrap();

                            match blocking_res {
                                Some(reply) => {
                                    if let Some(tx) = rpc_msg.reply_tx {
                                        tx.send(reply).await.unwrap();
                                    }
                                }
                                None => {
                                    // internal_error
                                    if let Some(tx) = rpc_msg.reply_tx {
                                        tx.send(jsonrpc_lite::JsonRpc::error(
                                            id,
                                            jsonrpc_lite::Error::internal_error(),
                                        ))
                                        .await
                                        .unwrap();
                                    }
                                }
                            }
                        }
                        JsonRpc::Notification(_) => {
                            if let Some(tx) = rpc_msg.reply_tx {
                                tx.send(jsonrpc_lite::JsonRpc::error(
                                    0,
                                    jsonrpc_lite::Error::invalid_request(),
                                ))
                                .await
                                .unwrap();
                            }
                        }
                        JsonRpc::Success(_) | JsonRpc::Error(_) => {
                            panic!("单向链路只应该收到 Reuquest 和 Notification");
                        }
                    },
                    _ => {
                        // ignore
                        // eprintln!(
                        //     "[{conn_id}] Received Empty Message, tx.is_closed(): {}",
                        //     tx.is_closed()
                        // );
                        // rx.close().await;
                        break;
                    }
                },
                Err(err) => {
                    // client disconnect : multiplexer terminate
                    eprintln!("{err}");
                    break;
                }
            }
        }
    }
}

pub async fn connect(pipe_name: &str) -> anyhow::Result<Client> {
    let mut client = Client::new(pipe_name);
    client.connect().await;
    Ok(client)
}
