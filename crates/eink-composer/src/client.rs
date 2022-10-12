// //
// // Copyright (C) Lenovo ThinkBook Gen4 Project.
// //
// // This program is protected under international and China copyright laws as
// // an unpublished work. This program is confidential and proprietary to the
// // copyright owners. Reproduction or disclosure, in whole or in part, or the
// // production of derivative works therefrom without the express permission of
// // the copyright owners is prohibited.
// //
// // All rights reserved.
// //

// use ipc_channel::ipc::IpcReceiverSet;

// pub struct SharedBuffer {}

// /// 维护一个显示层的多个共享缓冲区
// /// 每一个显示层的生产/消费步调都由会对应的 SharedBufferStack 来控制。
// /// 而它内部就用了几个成员变量来控制读写位置。
// pub struct SharedBufferStack {}

// impl SharedBufferStack {
//     pub fn new() -> Self {
//         Self {}
//     }
// }

// /// 一个 Client 最多支持 31 个显示层
// pub struct Client {
//     id: u64,
// }

// pub struct ClientManager {
//     clients: Vec<Client>,
//     rx_set: IpcReceiverSet,
// }

// impl ClientManager {
//     /// 创建客户端管理器
//     ///
//     /// ```
//     /// #include <stdio.h>
//     /// ```
//     ///
//     /// This function will return an error if .
//     fn new() -> anyhow::Result<Self> {
//         let clients = Vec::default();
//         let mut rx_set = IpcReceiverSet::new()?;
//         Ok(Self { clients, rx_set })
//     }

//     fn new_client(&mut self) -> &Client {
//         self.rx_set.add(client);
//         &client
//     }
// }
