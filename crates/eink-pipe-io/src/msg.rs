use jsonrpc_lite::JsonRpc;
use remoc::rch;

// User-defined data structures needs to implement Serialize
// and Deserialize.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IpcMsg {
    pub payload: JsonRpc,
    // Most Remoc types like channels can be included in serializable
    // data structures for transmission to remote endpoints.
    pub reply_tx: Option<rch::mpsc::Sender<JsonRpc>>,
}
