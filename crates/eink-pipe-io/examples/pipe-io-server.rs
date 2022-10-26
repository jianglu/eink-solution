use std::time::Duration;

use jsonrpc_lite::JsonRpc;
use remoc::rch;
use serde_json::json;
use tokio::{
    io::ReadHalf,
    net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions},
    time,
};
use windows::Win32::Foundation::ERROR_PIPE_BUSY;

const PIPE_NAME: &str = r"\\.\pipe\lenovo\thinbook-eink-plus\eink-service";

#[tokio::main]
async fn main() {
    let mut server = eink_pipe_io::server::Server::new(PIPE_NAME);
    let _on_request_conn = server.on_connection(|socket, req| {
        println!("On connection");
        socket.lock().on_request(|socket, id, req| {
            // 在当前线程上下文执行异步方法
            let ret = tokio::runtime::Handle::current().block_on(async move {
                socket
                    .lock()
                    .call_with_params("client-method", serde_json::json!({}))
                    .await
            });

            println!("client-method: {ret:?}");

            JsonRpc::success(id, &json!({"request": req.get_params().unwrap()}))
        });
        0
    });
    server.listen().await;
}
