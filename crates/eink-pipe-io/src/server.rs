use std::{
    marker,
    os::windows::prelude::AsRawHandle,
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
    net::windows::named_pipe::{self, ClientOptions, ServerOptions},
    time,
};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{BOOL, ERROR_PIPE_BUSY, ERROR_SUCCESS, HANDLE, PSID},
        Security::{
            AllocateAndInitializeSid,
            Authorization::{
                SetEntriesInAclW, SetSecurityInfo, ACCESS_MODE, EXPLICIT_ACCESS_W, SET_ACCESS,
                SE_KERNEL_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_TYPE,
            },
            FreeSid, InitializeSecurityDescriptor, SetSecurityDescriptorDacl, ACE_FLAGS, ACL,
            DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, SID,
        },
        Storage::FileSystem::FILE_WRITE_DATA,
        System::{
            Memory::{LocalAlloc, LocalFree, LPTR},
            SystemServices::{GENERIC_READ, GENERIC_WRITE, SECURITY_DESCRIPTOR_REVISION},
        },
    },
};

use crate::msg::IpcMsg;

pub struct ServerHandlers {
    pub on_request: Signal<(i32, JsonRpc), JsonRpc>,
}

pub struct Server {
    pipe_name: String,
    // handlers: Arc<Mutex<ServerHandlers>>,
    on_connection: Signal<(Arc<Mutex<Socket>>, i32), i32>,
    security_attributes: SecurityAttributes,
}

impl Server {
    pub fn new(pipe_name: &str) -> Self {
        Self {
            pipe_name: pipe_name.to_string(),
            on_connection: Signal::new(),
            security_attributes: SecurityAttributes::allow_everyone_create().unwrap(),
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
        let mut pipe_server = unsafe {
            ServerOptions::new()
                .first_pipe_instance(true)
                .reject_remote_clients(true)
                // .access_inbound(true)
                // .access_outbound(true)
                // .in_buffer_size(65536 * 100)
                // .out_buffer_size(65536 * 100)
                // .pipe_mode(named_pipe::PipeMode::Message)
                .create_with_security_attributes_raw(
                    &self.pipe_name,
                    std::mem::transmute(self.security_attributes.as_ptr()),
                )
                .unwrap()
        };

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

/// Security attributes.
pub struct SecurityAttributes {
    attributes: Option<InnerAttributes>,
}

pub const DEFAULT_SECURITY_ATTRIBUTES: SecurityAttributes = SecurityAttributes {
    attributes: Some(InnerAttributes {
        descriptor: SecurityDescriptor {
            descriptor_ptr: PSECURITY_DESCRIPTOR(std::ptr::null_mut()),
        },
        acl: Acl {
            acl_ptr: std::ptr::null_mut(),
        },
        attrs: SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: std::ptr::null_mut(),
            bInheritHandle: BOOL(0),
        },
    }),
};

impl SecurityAttributes {
    /// New default security attributes.
    pub fn empty() -> SecurityAttributes {
        DEFAULT_SECURITY_ATTRIBUTES
    }

    /// New default security attributes that allow everyone to connect.
    pub fn allow_everyone_connect(&self) -> std::io::Result<SecurityAttributes> {
        let attributes = Some(InnerAttributes::allow_everyone(
            GENERIC_READ | FILE_WRITE_DATA.0,
        )?);
        Ok(SecurityAttributes { attributes })
    }

    /// Set a custom permission on the socket
    pub fn set_mode(self, _mode: u32) -> std::io::Result<Self> {
        // for now, does nothing.
        Ok(self)
    }

    /// New default security attributes that allow everyone to create.
    pub fn allow_everyone_create() -> std::io::Result<SecurityAttributes> {
        let attributes = Some(InnerAttributes::allow_everyone(
            GENERIC_READ | GENERIC_WRITE,
        )?);
        Ok(SecurityAttributes { attributes })
    }

    /// Return raw handle of security attributes.
    pub(crate) unsafe fn as_ptr(&mut self) -> *mut SECURITY_ATTRIBUTES {
        match self.attributes.as_mut() {
            Some(attributes) => attributes.as_ptr(),
            None => std::ptr::null_mut(),
        }
    }
}

unsafe impl Send for SecurityAttributes {}

struct Sid {
    sid_ptr: PSID,
}

impl Sid {
    fn everyone_sid() -> std::io::Result<Sid> {
        pub const SECURITY_WORLD_SID_AUTHORITY: [u8; 6] = [0, 0, 0, 0, 0, 1];
        pub const SECURITY_WORLD_RID: u32 = 0x00000000;

        let mut sid_ptr = PSID(std::ptr::null_mut());
        let result = unsafe {
            #[allow(const_item_mutation)]
            AllocateAndInitializeSid(
                SECURITY_WORLD_SID_AUTHORITY.as_mut_ptr() as *mut _,
                1,
                SECURITY_WORLD_RID,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                &mut sid_ptr as *mut PSID,
            )
        };

        if !result.as_bool() {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Sid { sid_ptr })
        }
    }

    // Unsafe - the returned pointer is only valid for the lifetime of self.
    unsafe fn as_ptr(&self) -> PSID {
        self.sid_ptr
    }
}

impl Drop for Sid {
    fn drop(&mut self) {
        if !self.sid_ptr.is_invalid() {
            unsafe {
                FreeSid(self.sid_ptr);
            }
        }
    }
}

struct AceWithSid<'a> {
    explicit_access: EXPLICIT_ACCESS_W,
    _marker: marker::PhantomData<&'a Sid>,
}

impl<'a> AceWithSid<'a> {
    fn new(sid: &'a Sid, trustee_type: i32) -> AceWithSid<'a> {
        let mut explicit_access = unsafe { std::mem::zeroed::<EXPLICIT_ACCESS_W>() };
        explicit_access.Trustee.TrusteeForm = TRUSTEE_IS_SID;
        explicit_access.Trustee.TrusteeType = TRUSTEE_TYPE(trustee_type as i32);
        explicit_access.Trustee.ptstrName = unsafe { PWSTR(std::mem::transmute(sid.as_ptr())) };

        AceWithSid {
            explicit_access,
            _marker: marker::PhantomData,
        }
    }

    fn set_access_mode(&mut self, access_mode: i32) -> &mut Self {
        self.explicit_access.grfAccessMode = ACCESS_MODE(access_mode);
        self
    }

    fn set_access_permissions(&mut self, access_permissions: u32) -> &mut Self {
        self.explicit_access.grfAccessPermissions = access_permissions;
        self
    }

    fn allow_inheritance(&mut self, inheritance_flags: u32) -> &mut Self {
        self.explicit_access.grfInheritance = ACE_FLAGS(inheritance_flags);
        self
    }
}

struct Acl {
    acl_ptr: *mut ACL,
}

impl Acl {
    fn empty() -> std::io::Result<Acl> {
        Self::new(&mut [])
    }

    fn new(entries: &mut [AceWithSid<'_>]) -> std::io::Result<Acl> {
        let mut acl_ptr = std::ptr::null_mut();
        let result = unsafe {
            SetEntriesInAclW(
                Some(unsafe {
                    &*(entries as *mut _ as *mut [EXPLICIT_ACCESS_W]) as &[EXPLICIT_ACCESS_W]
                }),
                None,
                &mut acl_ptr,
            )
        };

        if result != ERROR_SUCCESS.0 {
            return Err(std::io::Error::from_raw_os_error(result as i32));
        }

        Ok(Acl { acl_ptr })
    }

    unsafe fn as_ptr(&self) -> *mut ACL {
        self.acl_ptr
    }
}

impl Drop for Acl {
    fn drop(&mut self) {
        if !self.acl_ptr.is_null() {
            unsafe { LocalFree(std::mem::transmute(self.acl_ptr)) };
        }
    }
}

struct SecurityDescriptor {
    descriptor_ptr: PSECURITY_DESCRIPTOR,
}

#[cfg(target_pointer_width = "64")]
pub const SECURITY_DESCRIPTOR_MIN_LENGTH: usize = 40;
#[cfg(target_pointer_width = "32")]
pub const SECURITY_DESCRIPTOR_MIN_LENGTH: usize = 20;

impl SecurityDescriptor {
    fn new() -> std::io::Result<Self> {
        let descriptor_ptr = unsafe { LocalAlloc(LPTR, SECURITY_DESCRIPTOR_MIN_LENGTH) };
        let descriptor_ptr = PSECURITY_DESCRIPTOR(unsafe { std::mem::transmute(descriptor_ptr) });
        if descriptor_ptr.is_invalid() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to allocate security descriptor",
            ));
        }

        if unsafe {
            !InitializeSecurityDescriptor(descriptor_ptr, SECURITY_DESCRIPTOR_REVISION).as_bool()
        } {
            return Err(std::io::Error::last_os_error());
        };

        Ok(SecurityDescriptor { descriptor_ptr })
    }

    fn set_dacl(&mut self, acl: &Acl) -> std::io::Result<()> {
        if unsafe {
            !SetSecurityDescriptorDacl(
                self.descriptor_ptr, //
                BOOL(1),
                Some(acl.as_ptr()),
                BOOL(0),
            )
            .as_bool()
        } {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    unsafe fn as_ptr(&self) -> PSECURITY_DESCRIPTOR {
        self.descriptor_ptr
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        if !self.descriptor_ptr.is_invalid() {
            unsafe { LocalFree(std::mem::transmute(self.descriptor_ptr)) };
            self.descriptor_ptr = PSECURITY_DESCRIPTOR(std::ptr::null_mut());
        }
    }
}

struct InnerAttributes {
    descriptor: SecurityDescriptor,
    acl: Acl,
    attrs: SECURITY_ATTRIBUTES,
}

impl InnerAttributes {
    fn empty() -> std::io::Result<InnerAttributes> {
        let descriptor = SecurityDescriptor::new()?;
        let mut attrs = unsafe { std::mem::zeroed::<SECURITY_ATTRIBUTES>() };
        attrs.nLength = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;
        attrs.lpSecurityDescriptor = unsafe { std::mem::transmute(descriptor.as_ptr()) };
        attrs.bInheritHandle = BOOL(1); // false as i32;

        let acl = Acl::empty().expect("this should never fail");

        Ok(InnerAttributes {
            acl,
            descriptor,
            attrs,
        })
    }

    fn allow_everyone(permissions: u32) -> std::io::Result<InnerAttributes> {
        let mut attributes = Self::empty()?;
        let sid = Sid::everyone_sid()?;

        let mut everyone_ace = AceWithSid::new(&sid, TRUSTEE_IS_WELL_KNOWN_GROUP.0);
        everyone_ace
            .set_access_mode(SET_ACCESS.0)
            .set_access_permissions(permissions)
            .allow_inheritance(false as u32);

        let mut entries = vec![everyone_ace];
        attributes.acl = Acl::new(&mut entries)?;
        attributes.descriptor.set_dacl(&attributes.acl)?;

        Ok(attributes)
    }

    unsafe fn as_ptr(&mut self) -> *mut SECURITY_ATTRIBUTES {
        &mut self.attrs as *mut _
    }
}

#[cfg(test)]
mod test {
    use super::SecurityAttributes;

    #[test]
    fn test_allow_everyone_everything() {
        SecurityAttributes::allow_everyone_create()
            .expect("failed to create security attributes that allow everyone to create a pipe");
    }

    #[test]
    fn test_allow_eveyone_read_write() {
        SecurityAttributes::empty()
            .allow_everyone_connect()
            .expect("failed to create security attributes that allow everyone to read and write to/from a pipe");
    }
}
