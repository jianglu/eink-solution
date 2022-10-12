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

use std::ffi::c_void;

use windows::{
    core::{IUnknown, PCSTR},
    Win32::Graphics::{
        Direct3D::{
            ID3DBlob, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_NULL, D3D_DRIVER_TYPE_REFERENCE,
            D3D_DRIVER_TYPE_SOFTWARE, D3D_DRIVER_TYPE_UNKNOWN, D3D_DRIVER_TYPE_WARP,
            D3D_FEATURE_LEVEL, D3D_SHADER_MACRO,
        },
        Direct3D11::{
            D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_DEBUG,
            D3D11_CREATE_DEVICE_DEBUGGABLE, D3D11_CREATE_DEVICE_DISABLE_GPU_TIMEOUT,
            D3D11_CREATE_DEVICE_PREVENT_ALTERING_LAYER_SETTINGS_FROM_REGISTRY,
            D3D11_CREATE_DEVICE_PREVENT_INTERNAL_THREADING_OPTIMIZATIONS,
            D3D11_CREATE_DEVICE_SINGLETHREADED, D3D11_CREATE_DEVICE_SWITCH_TO_REF,
            D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
        },
    },
};

use crate::impl_bitflag_operators;

// /// Driver type options.
// #[derive(Clone, Copy, Debug)]
// #[repr(u32)]
// pub enum DriverType {
//     Unknown = D3D_DRIVER_TYPE_UNKNOWN.0 as u32,
//     Hardware = D3D_DRIVER_TYPE_HARDWARE.0 as u32,
//     Reference = D3D_DRIVER_TYPE_REFERENCE.0 as u32,
//     Null = D3D_DRIVER_TYPE_NULL.0 as u32,
//     Software = D3D_DRIVER_TYPE_SOFTWARE.0 as u32,
//     Warp = D3D_DRIVER_TYPE_WARP.0 as u32,
// }

// #[derive(Clone, Copy, PartialEq, Eq, Debug)]
// pub struct CreateDeviceFlags(pub(crate) u32);
// #[allow(non_upper_case_globals)]
// impl CreateDeviceFlags {
//     pub const SingleThreaded: Self = Self(D3D11_CREATE_DEVICE_SINGLETHREADED.0 as u32);
//     pub const Debug: Self = Self(D3D11_CREATE_DEVICE_DEBUG.0 as u32);
//     pub const SwitchToRef: Self = Self(D3D11_CREATE_DEVICE_SWITCH_TO_REF.0 as u32);
//     pub const PreventInternalThreadingOptimizations: Self =
//         Self(D3D11_CREATE_DEVICE_PREVENT_INTERNAL_THREADING_OPTIMIZATIONS.0 as u32);
//     pub const BGRASupport: Self = Self(D3D11_CREATE_DEVICE_BGRA_SUPPORT.0 as u32);
//     pub const Debuggable: Self = Self(D3D11_CREATE_DEVICE_DEBUGGABLE.0 as u32);
//     pub const PreventAlteringLayerSettingsFromRegistry: Self =
//         Self(D3D11_CREATE_DEVICE_PREVENT_ALTERING_LAYER_SETTINGS_FROM_REGISTRY.0 as u32);
//     pub const DisableGPUTimeout: Self = Self(D3D11_CREATE_DEVICE_DISABLE_GPU_TIMEOUT.0 as u32);
//     pub const VideoSupport: Self = Self(D3D11_CREATE_DEVICE_VIDEO_SUPPORT.0 as u32);
// }
// impl_bitflag_operators!(CreateDeviceFlags);

// /// Represents a feature level.
// #[derive(Clone, Copy, PartialEq, Eq, Debug)]
// pub struct FeatureLevel(pub u32, pub u32);
// impl From<FeatureLevel> for D3D_FEATURE_LEVEL {
//     fn from(src: FeatureLevel) -> D3D_FEATURE_LEVEL {
//         D3D_FEATURE_LEVEL(((src.0 << 12) | (src.1 << 8)) as i32)
//     }
// }
// impl From<D3D_FEATURE_LEVEL> for FeatureLevel {
//     fn from(src: D3D_FEATURE_LEVEL) -> FeatureLevel {
//         FeatureLevel(((src.0 >> 12) & 0xf) as u32, ((src.0 >> 8) & 0xf) as u32)
//     }
// }

/// Defines a shader macro.
#[derive(Clone, Debug)]
pub struct ShaderMacro<'a, 'b> {
    pub name: &'a str,
    pub definition: &'b str,
}

impl<'a, 'b> ShaderMacro<'a, 'b> {
    /// .
    pub fn new(name: &'a impl AsRef<str>, definition: &'b impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref(),
            definition: definition.as_ref(),
        }
    }

    // This function is used in d3dcompiler.rs.
    #[allow(dead_code)]
    pub(crate) fn to_c_struct(&self) -> (D3D_SHADER_MACRO, (std::ffi::CString, std::ffi::CString)) {
        let name = std::ffi::CString::new(self.name).unwrap();
        let definition = std::ffi::CString::new(self.definition).unwrap();
        (
            D3D_SHADER_MACRO {
                Name: PCSTR(name.as_ptr() as *const u8),
                Definition: PCSTR(definition.as_ptr() as *const u8),
            },
            (name, definition),
        )
    }
}

/// Defines the ID3D12Blob interface.
pub trait IBlob {
    fn get_buffer_pointer(&self) -> *const c_void;
    fn get_buffer_pointer_mut(&mut self) -> *mut c_void;
    fn get_buffer_size(&self) -> usize;
    fn as_slice(&self) -> &[u8];
    fn as_mut_slice(&mut self) -> &mut [u8];
    fn to_vec(&self) -> Vec<u8>;
    fn as_cstr(&self) -> Result<&std::ffi::CStr, std::ffi::FromBytesWithNulError>;
}

#[derive(Clone, Debug)]
// #[implement(windows::core::IUnknown)]
pub struct Blob(pub(crate) ID3DBlob);

impl IBlob for Blob {
    fn get_buffer_pointer(&self) -> *const c_void {
        unsafe { self.0.GetBufferPointer() }
    }

    fn get_buffer_pointer_mut(&mut self) -> *mut c_void {
        unsafe { self.0.GetBufferPointer() }
    }

    fn get_buffer_size(&self) -> usize {
        unsafe { self.0.GetBufferSize() }
    }

    fn as_slice(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.get_buffer_pointer() as *const u8,
                self.get_buffer_size(),
            )
        }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.get_buffer_pointer_mut() as *mut u8,
                self.get_buffer_size(),
            )
        }
    }

    fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    fn as_cstr(&self) -> Result<&std::ffi::CStr, std::ffi::FromBytesWithNulError> {
        std::ffi::CStr::from_bytes_with_nul(self.as_slice())
    }
}
