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

const PIPE_NAME: &str = r"\\.\pipe\pipe-io-idiomatic-server";

// User-defined data structures needs to implement Serialize
// and Deserialize.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JsonRpcMsg {
    payload: JsonRpc,
    // Most Remoc types like channels can be included in serializable
    // data structures for transmission to remote endpoints.
    reply_tx: rch::mpsc::Sender<JsonRpc>,
}

#[tokio::main]
async fn main() {
    // The first server needs to be constructed early so that clients can
    // be correctly connected. Otherwise calling .wait will cause the client to
    // error.
    //
    // Here we also make use of `first_pipe_instance`, which will ensure that
    // there are no other servers up and running already.
    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(PIPE_NAME)
        .unwrap();

    // Spawn the server loop.
    let server = tokio::spawn(async move {
        loop {
            // Wait for a client to connect.
            let _connected = match server.connect().await {
                Ok(res) => res,
                Err(err) => panic!("err: {err}"),
            };

            let pipe_server = server;

            // Construct the next server to be connected before sending the one
            // we already have of onto a task. This ensures that the server
            // isn't closed (after it's done in the task) before a new one is
            // available. Otherwise the client might error with
            // `io::ErrorKind::NotFound`.
            server = ServerOptions::new().create(PIPE_NAME).unwrap();

            let client = tokio::spawn(async move {
                /* use the connected client */
                let (pipe_rx, pipe_tx) = tokio::io::split(pipe_server);

                // Establish Remoc connection over pipe connection.
                // The connection is always bidirectional, but we can just drop
                // the unneeded sender.
                let (conn, tx, rx): (
                    _,
                    rch::base::Sender<JsonRpcMsg>,
                    rch::base::Receiver<JsonRpcMsg>,
                ) = remoc::Connect::io(remoc::Cfg::default(), pipe_rx, pipe_tx)
                    .await
                    .unwrap();

                tokio::spawn(conn);

                let conn_id = uuid::Uuid::new_v4().as_u128();

                // Run server.
                run_server(conn_id, tx, rx).await;
            });
        }
    });

    /* do something else not server related here */
    let _ret = tokio::join!(server);
}

// This would be run on the server.
// It receives a count request from the client and sends each number
// as it is counted over the MPSC channel sender provided by the client.
async fn run_server(
    conn_id: u128,
    _tx: rch::base::Sender<JsonRpcMsg>,
    mut rx: rch::base::Receiver<JsonRpcMsg>,
) {
    println!("\n\n[{conn_id}] New connection !!!!");
    loop {
        match rx.recv().await {
            Ok(received) => match received {
                Some(rpc_msg) => match &rpc_msg.payload {
                    JsonRpc::Request(_) => {
                        tokio::spawn(async move {
                            println!("[{conn_id}] Received Request Message, wait for a moment");
                            let id = rpc_msg.payload.get_id().unwrap();
                            let method = rpc_msg.payload.get_method().unwrap();

                            // do some business
                            time::sleep(Duration::from_millis(1000)).await;

                            rpc_msg
                                .reply_tx
                                .send(jsonrpc_lite::JsonRpc::success(
                                    id,
                                    &json!({ "method": method }),
                                ))
                                .await
                                .unwrap();
                            println!("[{conn_id}] Sended Reply Message");
                        });
                    }
                    JsonRpc::Notification(_) => {
                        println!("[{conn_id}] Received Notification Message");
                    }
                    JsonRpc::Success(_) => {
                        eprintln!("[{conn_id}] Received Success Message");
                    }
                    JsonRpc::Error(_) => {
                        eprintln!("[{conn_id}] Received Error Message");
                    }
                },
                _ => {
                    // ignore
                    eprintln!(
                        "[{conn_id}] Received Empty Message, tx.is_closed(): {}",
                        tx.is_closed()
                    );
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
    println!("[{conn_id}] Connection closed !!!!\n\n");
}
