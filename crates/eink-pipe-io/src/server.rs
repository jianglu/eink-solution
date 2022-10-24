use std::{
    pin::Pin,
    sync::{Arc, Weak},
    time::Duration,
};

use anyhow::bail;
use jsonrpc_lite::{Id, JsonRpc, Params};
use parking_lot::Mutex;
use remoc::rch;
use signals2::{connect::ConnectionImpl, Connect2, Connect3, Connection, Emit2, Emit3, Signal};
use tokio::{
    net::windows::named_pipe::{ClientOptions, ServerOptions},
    time,
};
use windows::Win32::Foundation::ERROR_PIPE_BUSY;

use crate::msg::IpcMsg;

pub struct ServerHandlers {
    pub on_request: Signal<(i32, JsonRpc), JsonRpc>,
}

pub struct Server {
    pipe_name: String,
    // handlers: Arc<Mutex<ServerHandlers>>,
    on_connection: Signal<(Arc<Mutex<Socket>>, i32), i32>,
}

impl Server {
    pub fn new(pipe_name: &str) -> Self {
        Self {
            pipe_name: pipe_name.to_string(),
            on_connection: Signal::new(),
        }
    }

    /// 设置请求回调，使用 signals 接口
    pub fn on_connection<Callback>(&mut self, cb: Callback) -> Connection
    where
        Callback: Fn(Arc<Mutex<Socket>>, i32) -> i32 + Send + Sync + 'static,
    {
        self.on_connection.connect(cb)
    }

    pub async fn listen(&mut self) {
        // 创建底层 pipe 连接

        let pipe_name = self.pipe_name.clone();
        let on_connection_cloned = self.on_connection.clone();

        // The first server needs to be constructed early so that clients can
        // be correctly connected. Otherwise calling .wait will cause the client to
        // error.
        //
        // Here we also make use of `first_pipe_instance`, which will ensure that
        // there are no other servers up and running already.
        let mut pipe_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&self.pipe_name)
            .unwrap();

        // Spawn the server loop.
        loop {
            // Wait for a client to connect.
            let _connected = match pipe_server.connect().await {
                Ok(res) => res,
                Err(err) => panic!("err: {err}"),
            };

            /* use the connected client */
            let (pipe_rx, pipe_tx) = tokio::io::split(pipe_server);

            // Construct the next server to be connected before sending the one
            // we already have of onto a task. This ensures that the server
            // isn't closed (after it's done in the task) before a new one is
            // available. Otherwise the client might error with
            // `io::ErrorKind::NotFound`.
            pipe_server = ServerOptions::new().create(&pipe_name).unwrap();

            let on_connection_cloned2 = on_connection_cloned.clone();

            let _client = tokio::spawn(async move {
                // Establish Remoc connection over pipe connection.
                // The connection is always bidirectional, but we can just drop
                // the unneeded sender.
                let (conn, tx, rx): (_, rch::base::Sender<IpcMsg>, rch::base::Receiver<IpcMsg>) =
                    remoc::Connect::io(remoc::Cfg::default(), pipe_rx, pipe_tx)
                        .await
                        .unwrap();

                tokio::spawn(conn);

                let socket = Arc::new(Mutex::new(Socket {
                    tx,
                    rx: Some(rx),
                    on_request: Signal::new(),
                }));

                on_connection_cloned2.emit(socket.clone(), 0);

                // Run server.
                Socket::process_incoming(socket).await;

                ()
            });
        }
    }
}

pub struct Socket {
    pub tx: rch::base::Sender<IpcMsg>,
    pub rx: Option<rch::base::Receiver<IpcMsg>>,
    pub on_request: Signal<(Arc<Mutex<Socket>>, Id, JsonRpc), JsonRpc>,
}

impl Socket {
    /// 设置请求回调，使用 signals 接口
    pub fn on_request<Callback>(&mut self, cb: Callback) -> Connection
    where
        Callback: Fn(Arc<Mutex<Socket>>, Id, JsonRpc) -> JsonRpc + Send + Sync + 'static,
    {
        self.on_request.connect(cb)
    }

    pub async fn call_with_params<P: Into<Params>>(
        &mut self,
        method: &str,
        params: P,
    ) -> anyhow::Result<JsonRpc> {
        let id = uuid::Uuid::new_v4().to_string();
        let (reply_tx, mut reply_rx) = rch::mpsc::channel(1);
        self.tx
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

    /// 处理输入的请求
    pub async fn process_incoming(this: Arc<Mutex<Self>>) {
        let conn_id = uuid::Uuid::new_v4().as_u128();
        eprintln!("client[{conn_id}] was connected:");

        let on_request = this.lock().on_request.clone();
        // let shared_self = this.clone();

        let mut rx = this.lock().rx.take().unwrap();

        loop {
            match rx.recv().await {
                Ok(received) => match received {
                    Some(rpc_msg) => match &rpc_msg.payload {
                        JsonRpc::Request(_) => {
                            let id = rpc_msg.payload.get_id().unwrap();
                            let id2 = id.clone();

                            // Signal 的 clone 是轻量级操作

                            // 事件处理可能是耗时操作，分离到 blocking 线程进行

                            let self_cloned = this.clone();
                            let on_request_cloned = on_request.clone();

                            let blocking_res = tokio::task::spawn_blocking(move || {
                                Box::new(on_request_cloned.emit(self_cloned, id2, rpc_msg.payload))
                            })
                            .await
                            .unwrap();

                            match *blocking_res {
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
                    eprintln!("client[{conn_id}] disconnect: {err}");
                    break;
                }
            }
        }
    }
}
