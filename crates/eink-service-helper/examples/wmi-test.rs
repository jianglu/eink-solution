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
use std::mem::zeroed;
use windows::{
    core::{IUnknown, InParam, BSTR, PCWSTR},
    s, w,
    Win32::{
        Foundation::{COLORREF, HWND},
        Security::PSECURITY_DESCRIPTOR,
        System::{
            Com::{
                self, CLSIDFromString, CoCreateInstance, CoCreateInstanceEx, CoInitializeEx,
                CoInitializeSecurity, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, EOAC_NONE,
                RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL_IMPERSONATE,
            },
            Wmi::{
                IWbemCallResult, IWbemClassObject, IWbemLocator, WbemLocator,
                WBEM_FLAG_RETURN_WBEM_COMPLETE,
            },
        },
        UI::{
            Shell::{ITaskbarList, ITaskbarList2, ITaskbarList3, ITaskbarList4},
            WindowsAndMessaging::{
                FindWindowA, GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW,
                GWL_EXSTYLE, GWL_STYLE, LAYERED_WINDOW_ATTRIBUTES_FLAGS, LWA_ALPHA,
                WINDOW_EX_STYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
            },
        },
    },
};

/// 服务助手程序
fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;

        CoInitializeSecurity(
            PSECURITY_DESCRIPTOR::default(),
            -1,
            None,
            None,
            RPC_C_AUTHN_LEVEL_DEFAULT,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
            None,
        )?;

        let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;

        let server = locator.ConnectServer(
            &BSTR::from("root\\wmi"),
            &BSTR::new(),
            &BSTR::new(),
            &BSTR::new(),
            0,
            &BSTR::new(),
            None,
        )?;

        let mut class: Option<IWbemClassObject> = None;

        // Get the class object for the method definition.
        let _hr = server.GetObject(
            &BSTR::from("LENOVO_TB_G4_CTRL"),
            0,
            None,
            Some(&mut class),
            None,
        )?;

        let inst = class.unwrap().SpawnInstance(0)?;

        let mut result: Option<IWbemClassObject> = None;

        let _hr = server.ExecMethod(
            &BSTR::from("LENOVO_TB_G4_CTRL"),
            &BSTR::from("GetEinkLightLevel"),
            0,
            None,
            Some(&inst),
            Some(&mut result),
            None,
        )?;

        println!("LENOVO_TB_G4_CTRL.GetEinkLightLevel: {result:?}");
    }

    Ok(())
}
