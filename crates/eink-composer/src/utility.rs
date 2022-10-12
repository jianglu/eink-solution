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

use anyhow::Result;

use crate::winrt::*;

#[doc(hidden)]
#[macro_export]
macro_rules! impl_bitflag_operators {
    ($s: ident) => {
        impl $s {
            pub fn enabled(&self, flag: Self) -> bool {
                Self(self.0 & flag.0) == flag
            }
            pub fn disabled(&self, flag: Self) -> bool {
                !self.enabled(flag)
            }
        }
        impl std::ops::BitAnd for $s {
            type Output = Self;
            fn bitand(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }
        }
        impl std::ops::BitAndAssign for $s {
            fn bitand_assign(&mut self, other: Self) {
                self.0 &= other.0
            }
        }
        impl std::ops::BitOr for $s {
            type Output = Self;
            fn bitor(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
        }
        impl std::ops::BitOrAssign for $s {
            fn bitor_assign(&mut self, other: Self) {
                self.0 |= other.0
            }
        }
    };
}

// #[doc(hidden)]
// #[macro_export]
// macro_rules! impl_interface {
//     ($s: ident, $api_type: ident) => {
//         unsafe impl Send for $s {}
//         unsafe impl Sync for $s {}

//         impl Interface for $s {
//             type APIType = $api_type;
//             fn new(p: com_ptr::ComPtr<Self::APIType>) -> Self {
//                 $s(p)
//             }
//             fn uuidof() -> $crate::api::Guid {
//                 use winapi::Interface as _;
//                 Self::APIType::uuidof().into()
//             }
//             fn as_ptr(&self) -> *mut Self::APIType {
//                 self.0.as_ptr()
//             }
//             fn as_com_ptr(&self) -> &com_ptr::ComPtr<Self::APIType> {
//                 &self.0
//             }
//             fn as_unknown(&self) -> *mut winapi::um::unknwnbase::IUnknown {
//                 Interface::as_ptr(self) as *mut winapi::um::unknwnbase::IUnknown
//             }
//             fn from_com_ptr(p: com_ptr::ComPtr<Self::APIType>) -> Self {
//                 $s(p)
//             }
//             fn query_interface<T: $crate::Interface>(&self) -> Result<T, $crate::result::HResult> {
//                 let p = self
//                     .as_com_ptr()
//                     .query_interface::<<T as $crate::Interface>::APIType>();
//                 if let Err(e) = p {
//                     Err(e.into())
//                 } else {
//                     Ok(T::new(p.unwrap()))
//                 }
//             }
//         }
//     };
// }

/// This macros allows to hide panicing messages in output binary when feature `no-msgs` is present.
#[macro_export]
macro_rules! expect {
    ($val:expr, $msg:expr) => {
        if cfg!(feature = "no-msgs") {
            $val.unwrap()
        } else {
            $val.expect($msg)
        }
    };
}

#[macro_export]
macro_rules! panic_msg {
    ($($t:tt)*) => {
        if cfg!(feature = "no-msgs") {
            unimplemented!()
        } else {
            panic!($($t)*)
        }
    };
}

/// Creates zero terminated string.
#[macro_export]
macro_rules! pc_str {
    ($cstr:expr) => {
        windows::core::PCSTR(concat!($cstr, "\x00").as_ptr() as _)
    };
}

pub fn pid_to_handle(pid: u32) -> Result<HANDLE> {
    unsafe { Ok(OpenProcess(PROCESS_ALL_ACCESS, false, pid)?) }
}
