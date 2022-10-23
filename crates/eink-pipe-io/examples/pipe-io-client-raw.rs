use std::time::Duration;

use jsonrpc_lite::JsonRpc;
use remoc::rch;
use serde_json::json;
use tokio::{net::windows::named_pipe::ClientOptions, time};
use windows::Win32::Foundation::ERROR_PIPE_BUSY;

const PIPE_NAME: &str = r"\\.\pipe\pipe-io-idiomatic-server";

// User-defined data structures needs to implement Serialize
// and Deserialize.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct IoMsg {
    payload: JsonRpc,
    // Most Remoc types like channels can be included in serializable
    // data structures for transmission to remote endpoints.
    reply_tx: rch::mpsc::Sender<JsonRpc>,
}

// This would be run on the client.
// It sends a count request to the server and receives each number
// as it is counted over a newly established MPSC channel.
async fn run_client(mut tx: rch::base::Sender<IoMsg>) {
    for i in 0..10 {
        // By sending seq_tx over an existing remote channel, a new remote
        // channel is automatically created and connected to the server.
        // This all happens inside the existing TCP connection.
        let (reply_tx, mut reply_rx) = rch::mpsc::channel(1);

        tx.send(IoMsg {
            payload: jsonrpc_lite::JsonRpc::request_with_params(
                0,
                "set_window_topmost",
                json!({"name":"Jiang Lu", "mail": "jianglu@ensurebit.com"}),
            ),
            reply_tx,
        })
        .await
        .unwrap();

        let reply = reply_rx.recv().await.unwrap();

        println!("reply : {reply:?}");
    }

    // Receive counted numbers over new channel.
    // for i in 0..5 {
    // }
    // assert_eq!(seq_rx.recv().await.unwrap(), Some(0));
    // assert_eq!(seq_rx.recv().await.unwrap(), Some(1));
    // assert_eq!(seq_rx.recv().await.unwrap(), Some(2));
    // assert_eq!(seq_rx.recv().await.unwrap(), Some(3));
    // assert_eq!(seq_rx.recv().await.unwrap(), None);
}

#[tokio::main]
async fn main() {
    // Establish named-pipe connection.
    let pipe_client = loop {
        match ClientOptions::new().open(PIPE_NAME) {
            Ok(client) => break client,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32) => (),
            Err(e) => return,
        }

        time::sleep(Duration::from_millis(50)).await;
    };

    let (pipe_rx, pipe_tx) = tokio::io::split(pipe_client);

    // Establish Remoc connection over TCP.
    // The connection is always bidirectional, but we can just drop
    // the unneeded receiver.
    let (conn, tx, _rx): (_, _, rch::base::Receiver<JsonRpc>) =
        remoc::Connect::io(remoc::Cfg::default(), pipe_rx, pipe_tx)
            .await
            .unwrap();

    tokio::spawn(conn);

    // Run client.
    run_client(tx).await;
}
