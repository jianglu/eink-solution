use jsonrpc_lite::{JsonRpc, Params};
use signals2::Connection;
use tokio::runtime::Runtime;

use crate::client::Client;

struct BlockingServer {}

impl BlockingServer {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct BlockingClient {
    inner: crate::client::Client,
    rt: Runtime,
}

impl BlockingClient {
    pub fn on_request<Callback>(&mut self, cb: Callback) -> Connection
    where
        Callback: Fn(i32, JsonRpc) -> JsonRpc + Send + Sync + 'static,
    {
        self.rt.block_on(self.inner.on_request(cb))
    }

    pub fn call_with_params<P: Into<Params>>(
        &mut self,
        method: &str,
        params: P,
    ) -> anyhow::Result<JsonRpc> {
        self.rt
            .block_on(self.inner.call_with_params(method, params))
    }
}

// /// Establish a connection with the Redis server located at `addr`.
// ///
// /// `addr` may be any type that can be asynchronously converted to a
// /// `SocketAddr`. This includes `SocketAddr` and strings. The `ToSocketAddrs`
// /// trait is the Tokio version and not the `std` version.
// ///
// /// # Examples
// ///
// /// ```no_run
// /// use pipe_io::blocking;
// ///
// /// fn main() {
// ///     let client = match blocking::connect("localhost:6379") {
// ///         Ok(client) => client,
// ///         Err(_) => panic!("failed to establish connection"),
// ///     };
// /// # drop(client);
// /// }
// /// ```
pub fn connect(pipe_name: &str) -> anyhow::Result<BlockingClient> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let inner = rt.block_on(crate::client::connect(pipe_name))?;

    Ok(BlockingClient { inner, rt })
}
