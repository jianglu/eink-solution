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

use std::ffi::CString;

use windows::{
    core::{Borrowed, InParam, Interface, HRESULT, PCSTR},
    Win32::{
        Foundation::HINSTANCE,
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11InputLayout,
                D3D11_CREATE_DEVICE_FLAG, D3D11_INPUT_CLASSIFICATION, D3D11_INPUT_ELEMENT_DESC,
                D3D11_INPUT_PER_INSTANCE_DATA, D3D11_INPUT_PER_VERTEX_DATA,
                D3D11_RESOURCE_DIMENSION_BUFFER, D3D11_RESOURCE_DIMENSION_TEXTURE1D,
                D3D11_RESOURCE_DIMENSION_TEXTURE2D, D3D11_RESOURCE_DIMENSION_TEXTURE3D,
                D3D11_RESOURCE_DIMENSION_UNKNOWN, D3D11_SDK_VERSION,
            },
            Dxgi::Common::DXGI_FORMAT,
        },
    },
};

use crate::{
    d3d::{self, CreateDeviceFlags},
    dxgi,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum InputClassification {
    PerVertexData = D3D11_INPUT_PER_VERTEX_DATA.0,
    PerInstanceData = D3D11_INPUT_PER_INSTANCE_DATA.0,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum ResourceDimension {
    Unknown = D3D11_RESOURCE_DIMENSION_UNKNOWN.0,
    Buffer = D3D11_RESOURCE_DIMENSION_BUFFER.0,
    Texture1D = D3D11_RESOURCE_DIMENSION_TEXTURE1D.0,
    Texture2D = D3D11_RESOURCE_DIMENSION_TEXTURE2D.0,
    Texture3D = D3D11_RESOURCE_DIMENSION_TEXTURE3D.0,
}

#[derive(Clone, Debug)]
pub struct InputElementDesc<'a> {
    pub semantic_name: &'a str,
    pub semantic_index: u32,
    pub format: dxgi::Format,
    pub input_slot: u32,
    pub aligned_byte_offset: u32,
    pub input_slot_class: InputClassification,
    pub instance_data_step_rate: u32,
}

impl<'a> InputElementDesc<'a> {
    fn to_c_struct(&self) -> (D3D11_INPUT_ELEMENT_DESC, CString) {
        let name = CString::new(self.semantic_name).unwrap();
        (
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(name.as_ptr() as *const u8),
                SemanticIndex: self.semantic_index,
                Format: DXGI_FORMAT(self.format as u32),
                InputSlot: self.input_slot,
                AlignedByteOffset: self.aligned_byte_offset,
                InputSlotClass: D3D11_INPUT_CLASSIFICATION(self.input_slot_class as i32),
                InstanceDataStepRate: self.instance_data_step_rate,
            },
            name,
        )
    }
}

pub trait IDeviceChild: Interface {
    fn get_device(&self) -> Device;
    fn get_name(&self) -> Result<String, HRESULT>;
    fn set_name(&self, name: &str) -> Result<(), HRESULT>;
}

// macro_rules! impl_devicechild {
//     ($s: ident, $interface: ident) => {
//         impl_interface!($s, $interface);
//         impl IDeviceChild for $s {
//             fn get_device(&self) -> Device {
//                 let mut obj = std::ptr::null_mut();
//                 unsafe {
//                     self.0.GetDevice(&mut obj);
//                     Device(ComPtr::from_raw(obj))
//                 }
//             }
//             fn get_name(&self) -> Result<String, HRESULT> {
//                 unsafe {
//                     let mut sz = 0;
//                     let res = self.0.GetPrivateData(
//                         &WKPDID_D3DDebugObjectNameW,
//                         &mut sz,
//                         std::ptr::null_mut(),
//                     );
//                     let mut sz = hresult(sz, res)?;
//                     let mut buf =
//                         Vec::<u16>::with_capacity(sz as usize / std::mem::size_of::<u16>());
//                     let res = self.0.GetPrivateData(
//                         &WKPDID_D3DDebugObjectNameW,
//                         &mut sz,
//                         buf.as_mut_ptr() as *mut c_void,
//                     );
//                     let buf = hresult(buf, res)?;
//                     Ok(OsString::from_wide(&buf).to_string_lossy().to_string())
//                 }
//             }
//             fn set_name(&self, name: &str) -> Result<(), HResult> {
//                 unsafe {
//                     let wname = name.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
//                     let res = self.0.SetPrivateData(
//                         &WKPDID_D3DDebugObjectNameW,
//                         (std::mem::size_of::<u16>() * wname.len()) as u32,
//                         wname.as_ptr() as *const c_void,
//                     );
//                     hresult((), res)
//                 }
//             }
//         }
//     };
// }

pub trait IInputLayout: IDeviceChild {}

#[derive(Clone, Debug)]
pub struct InputLayout(ID3D11InputLayout);
// macro_rules! impl_inputlayout {
//     ($s: ident, $interface: ident) => {
//         impl_devicechild!($s, $interface);
//         impl IInputLayout for $s {}
//     };
// }
// impl_inputlayout!(InputLayout, ID3D11InputLayout);

#[derive(Clone, Debug)]
pub struct Device(ID3D11Device);

impl Device {
    fn create_input_layout<'a>(
        &self,
        descs: &[InputElementDesc<'a>],
        bytecode: &[u8],
    ) -> Result<InputLayout, HRESULT> {
        let (c_descs, _tmp): (Vec<_>, Vec<_>) = descs.iter().map(|d| d.to_c_struct()).unzip();

        let input_layout = unsafe {
            self.0
                .CreateInputLayout(c_descs.as_slice(), bytecode.as_slice())?
        };

        Ok(InputLayout(input_layout))
    }
}

#[derive(Clone, Debug)]
pub struct DeviceContext(ID3D11DeviceContext);

pub fn create_device(
    adapter: Option<&dxgi::Adapter>,
    driver_type: d3d::DriverType,
    software: Option<HINSTANCE>,
    flags: Option<CreateDeviceFlags>,
    feature_levels: &[d3d::FeatureLevel],
) -> Result<(Device, d3d::FeatureLevel, DeviceContext), HRESULT> {
    unsafe {
        let mut device = None;
        let mut device_context = None;

        let mut level = D3D_FEATURE_LEVEL::default();
        let feature_levels: Vec<D3D_FEATURE_LEVEL> =
            feature_levels.iter().map(|&l| l.into()).collect::<Vec<_>>();

        let res = D3D11CreateDevice(
            adapter.map_or(InParam::null(), |p| InParam::owned(p.0.clone())),
            D3D_DRIVER_TYPE(driver_type as u32 as i32),
            software.unwrap_or(HINSTANCE::default()),
            flags.map_or(D3D11_CREATE_DEVICE_FLAG::default(), |f| {
                D3D11_CREATE_DEVICE_FLAG(f.0)
            }),
            &feature_levels,
            D3D11_SDK_VERSION,
            &mut device,
            &mut level,
            &mut device_context,
        );

        if res.is_err() {
            return Err(res.into());
        }

        Ok((
            Device(device.unwrap()),
            level.into(),
            DeviceContext(device_context.unwrap()),
        ))
    }
}
