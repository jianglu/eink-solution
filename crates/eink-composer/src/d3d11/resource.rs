//
// Copyright (C) Lenovo ThinkBook Gen4 Project.
//
// This program is protected under international and China copyright laws as
// an unpublished work. This program is confidential and proprietary to the
// copyright owners. Reproduction or disclosure, in whole or in part, or the
// production of derivative works therefrom without the express permission of
// the copyright owners is prohibited.
//
// All rights reserved.
//

use windows::Win32::Graphics::Direct3D11::ID3D11Resource;

use super::{IDeviceChild, ResourceDimension};

pub trait IResource: IDeviceChild {
    fn get_eviction_priority(&self) -> u32;
    fn get_type(&self) -> ResourceDimension;
    fn set_eviction_priority(&self, priority: u32);
}

#[derive(Clone, Debug)]
pub struct Resource(ID3D11Resource);
// macro_rules! impl_resource {
//     ($s: ident, $interface: ident) => {
//         impl_devicechild!($s, $interface);
//         impl IResource for $s {
//             fn get_eviction_priority(&self) -> u32 {
//                 unsafe { self.0.GetEvictionPriority() }
//             }
//             fn get_type(&self) -> ResourceDimension {
//                 let mut value = 0;
//                 unsafe {
//                     self.0.GetType(&mut value);
//                     std::mem::transmute(value)
//                 }
//             }
//             fn set_eviction_priority(&self, priority: u32) {
//                 unsafe { self.0.SetEvictionPriority(priority) }
//             }
//         }
//     };
// }
// impl_resource!(Resource, ID3D11Resource);
