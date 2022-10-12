use std::{
    borrow::Cow,
    ffi::OsStr,
    io::{Read, Write},
};

use interprocess::{
    os::windows::named_pipe::{DuplexMsgPipeStream, PipeListenerOptions, PipeMode},
    ReliableReadMsg,
};

fn main() -> anyhow::Result<()> {
    let mut pipe = PipeListenerOptions::new()
        .name(OsStr::new("Name"))
        .accept_remote(false)
        .mode(PipeMode::Messages)
        .create::<DuplexMsgPipeStream>()?;

    let mut pipe_conn = pipe.accept()?;

    let pid = pipe_conn.client_process_id()?;
    println!("PID: {:?}", pid);

    let (tx, rx) = std::sync::mpsc::channel::<DuplexMsgPipeStream>();

    let mut msg = Vec::new();
    let res = pipe_conn.try_read_msg(&mut msg)?;

    pipe_conn.write_all(&msg)?;

    let mut conn = DuplexMsgPipeStream::connect("Name")?;
    conn.write_all(&msg)?;
    let res = conn.read_msg(&mut msg)?;
    Ok(())
}
