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

use ntapi::winapi::shared::ntdef::HRESULT;
use windows::core::Interface;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::*;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct CreateFactoryFlag(u32);

#[allow(non_upper_case_globals)]
impl CreateFactoryFlag {
    pub const Debug: Self = Self(DXGI_CREATE_FACTORY_DEBUG);
}
crate::impl_bitflag_operators!(CreateFactoryFlag);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Adapter(pub(crate) IDXGIAdapter4);

pub trait IFactory: Interface {
    // fn enum_adapters(&self) -> Result<Vec<Adapter>, HRESULT>;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Factory(IDXGIFactory);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Factory1(IDXGIFactory1);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Factory2(IDXGIFactory2);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Factory3(IDXGIFactory3);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Factory4(IDXGIFactory4);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Factory5(IDXGIFactory5);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Factory6(IDXGIFactory6);

impl_factory!(Factory, IDXGIFactory, Factory);
impl_factory!(Factory1, IDXGIFactory1, Factory1);
impl_factory!(Factory2, IDXGIFactory2, Factory2);
impl_factory!(Factory3, IDXGIFactory3, Factory3);
impl_factory!(Factory4, IDXGIFactory4, Factory4);
impl_factory!(Factory5, IDXGIFactory5, Factory5);
impl_factory!(Factory6, IDXGIFactory6, Factory6);

macro_rules! impl_factory {
    ($s: ident, $interface: ident, Factory) => {
        // impl_interface!($s, $interface);
        impl IFactory for $s {
            fn create_software_adapter(&self, module: HINSTANCE) -> Result<Adapter, HRESULT> {
                todo!()
            }

            fn create_swap_chain<T: Interface>(
                &self,
                device: &T,
                desc: &SwapChainDesc<
                    ModeDesc<u32, u32, Rational, Format>,
                    Usage,
                    u32,
                    *const c_void,
                    bool,
                    SwapEffect,
                >,
            ) -> Result<SwapChain, HResult> {
                todo!()
            }

            fn enum_adapters(&self) -> Result<Vec<Adapter>, HResult> {
                enum_function(DXGI_ERROR_NOT_FOUND.into(), |i| {
                    Ok(Adapter(ComPtr::new(|| {
                        let mut obj = std::ptr::null_mut();
                        let res = unsafe { self.0.EnumAdapters(i, &mut obj) };
                        hresult(obj, res.into())
                    })?))
                })
            }

            fn get_window_association(&self) -> Result<HWND, HResult> {
                todo!()
            }

            fn make_window_association(
                &self,
                hwnd: &impl WindowHandle,
                flags: Option<MakeWindowAssociationFlag>,
            ) -> Result<(), HResult> {
                todo!()
            }
        }
    };
    ($s: ident, $interface: ident, Factory1) => {
        impl_factory!($s, $interface, Factory);
        impl $s {
            pub fn as_factory(&self) -> Factory {
                Factory(self.0.as::<IDXGIFactory>().unwrap())
            }
        }
        impl IFactory1 for $s {
            fn enum_adapters1(&self) -> Result<Vec<Adapter1>, HResult> {
                enum_function(DXGI_ERROR_NOT_FOUND.into(), |i| {
                    Ok(Adapter1(ComPtr::new(|| {
                        let mut obj = std::ptr::null_mut();
                        let res = unsafe { self.0.EnumAdapters1(i, &mut obj) };
                        hresult(obj, res.into())
                    })?))
                })
            }
            fn is_current(&self) -> bool {
                unsafe { self.0.IsCurrent() == TRUE }
            }
        }
    };
    ($s: ident, $interface: ident, Factory2) => {
        impl_factory!($s, $interface, Factory1);

        impl $s {
            pub fn as_factory1(&self) -> Factory1 {
                Factory1(self.0.as::<IDXGIFactory1>().unwrap())
            }
        }

        impl IFactory2 for $s {

            fn create_swap_chain_for_composition<T: Interface>(
                &self,
                device: &T,
                desc: &SwapChainDesc1<u32, u32, Format, Usage, u32, SwapEffect>,
                restrict_to_output: Option<&Output>,
            ) -> Result<SwapChain1, HResult> {
                todo!()
            }

            fn create_swap_chain_for_core_window<T: Interface, U: Interface>(
                &self,
                device: &T,
                window: &U,
                desc: &SwapChainDesc1<u32, u32, Format, Usage, u32, SwapEffect>,
                restrict_to_output: Option<&Output>,
            ) -> Result<SwapChain1, HResult> {
                todo!()
            }

            fn create_swap_chain_for_hwnd<T: Interface>(
                &self,
                device: &T,
                hwnd: &impl WindowHandle,
                desc: &SwapChainDesc1<u32, u32, Format, Usage, u32, SwapEffect>,
                fullscreen_desc: Option<&SwapChainFullscreenDesc>,
                restrict_to_output: Option<&Output>,
            ) -> Result<SwapChain1, HResult> {
                todo!()
            }

            fn get_shared_resource_adapter_luid(&self, resource: HANDLE) -> Result<Luid, HResult> {
                todo!()
            }

            fn is_windowed_stereo_enabled(&self) -> bool {
                todo!()
            }

            fn register_occlusion_status_event(&self, hevent: HANDLE) -> Result<u32, HResult> {
                todo!()
            }

            fn register_occlusion_status_window(
                &self,
                hwnd: &impl WindowHandle,
                msg: UINT,
            ) -> Result<u32, HResult> {
                todo!()
            }

            fn register_stereo_status_event(&self, hevent: HANDLE) -> Result<u32, HResult> {
                todo!()
            }

            fn register_stereo_status_window(
                &self,
                hwnd: &impl WindowHandle,
                msg: UINT,
            ) -> Result<u32, HResult> {
                todo!()
            }

            fn unregister_occlusion_status(&self, cookie: u32) {
                todo!()
            }

            fn unregister_stereo_status(&self, cookie: u32) {
                todo!()
            }
        }
    };
    ($s: ident, $interface: ident, Factory3) => {
        impl_factory!($s, $interface, Factory2);
        impl $s {
            pub fn as_factory2(&self) -> Factory2 {
                Factory2(self.0.query_interface::<IDXGIFactory2>().unwrap())
            }
        }
        impl IFactory3 for $s {
            fn get_creation_flags(&self) -> CreateFactoryFlag {
                unsafe { CreateFactoryFlag(self.0.GetCreationFlags()) }
            }
        }
    };
    ($s: ident, $interface: ident, Factory4) => {
        impl_factory!($s, $interface, Factory3);
        impl $s {
            pub fn as_factory3(&self) -> Factory3 {
                Factory3(self.0.query_interface::<IDXGIFactory3>().unwrap())
            }
        }
        impl IFactory4 for $s {

            fn enum_adapter_by_luid<T: IAdapter>(&self, adapter_luid: Luid) -> Result<T, HResult> {
                Ok(T::from_com_ptr(ComPtr::new(|| {
                    let mut obj = std::ptr::null_mut();
                    let res = unsafe {
                        self.0
                            .EnumAdapterByLuid(adapter_luid.into(), &T::uuidof().into(), &mut obj)
                    };
                    hresult(obj as *mut T::APIType, res.into())
                })?))
            }

            fn enum_warp_adapter<T: IAdapter>(&self) -> Result<T, HResult> {
                todo!()
            }
        }
    };

    ($s: ident, $interface: ident, Factory5) => {
        impl_factory!($s, $interface, Factory4);

        impl $s {
            pub fn as_fatory4(&self) -> Factory4 {
                Factory4(self.0.as::<IDXGIFactory4>().unwrap())
            }
        }

        impl IFactory5 for $s {
            fn check_feature_support(
                &self,
                feature: Feature,
            ) -> Result<FeatureSupoortData, HResult> {
                todo!()
            }
        }
    };

    ($s: ident, $interface: ident, Factory6) => {

        impl_factory!($s, $interface, Factory5);

        impl $s {
            pub fn as_factory5(&self) -> Factory5 {
                Factory5(self.0.as::<IDXGIFactory5>().unwrap())
            }
        }

        impl IFactory6 for $s {

            fn enum_adapter_by_gpu_preference<T: IAdapter>(
                &self,
                gpu_preference: GPUPreference,
            ) -> Result<Vec<T>, HResult> {
                todo!()
            }
        }
    };
}

pub fn create_dxgi_factory2<T: IFactory>(flags: Option<CreateFactoryFlag>) -> Result<T, HRESULT> {
    let factory: T = unsafe { CreateDXGIFactory2(flags.map_or(0, |f| f.0))? };
    Ok(factory)
}
