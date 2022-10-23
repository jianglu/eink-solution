use std::time::Duration;

use jsonrpc_lite::{JsonRpc, Params};
use remoc::rch;
use serde_json::json;
use tokio::{net::windows::named_pipe::ClientOptions, time};
use windows::Win32::Foundation::ERROR_PIPE_BUSY;

const PIPE_NAME: &str = r"\\.\pipe\pipe-io-idiomatic-server";

#[tokio::main]
async fn main() {
    let mut client = eink_pipe_io::client::connect(PIPE_NAME).await.unwrap();
    {
        let _on_request_conn = client
            .on_request(|_id, _req| JsonRpc::error(0, jsonrpc_lite::Error::internal_error()))
            .await
            .scoped();
    }

    for i in 0..10 {
        let reply = client
            .call_with_params(
                "test_method",
                json!({"seq": i, "name": "Jiang Lu", "mail": "jianglu@ensurebit.com"}),
            )
            .await
            .unwrap();
        println!("reply: {reply:?}");

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }
}
