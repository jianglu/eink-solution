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

use anyhow::{bail, Result};

use std::{
    ffi::{c_void, CString},
    ptr::{null, null_mut},
};

use windows::{
    core::{InParam, PCSTR},
    Win32::Graphics::Direct3D::{Fxc::*, ID3DInclude, D3D_SHADER_MACRO},
};

use crate::{
    d3d::{Blob, ShaderMacro},
    impl_bitflag_operators,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CompileFlags(u32);
impl_bitflag_operators!(CompileFlags);
#[allow(non_upper_case_globals)]
impl CompileFlags {
    pub const Debug: Self = Self(D3DCOMPILE_DEBUG);
    pub const SkipValidation: Self = Self(D3DCOMPILE_SKIP_VALIDATION);
    pub const SkipOptimization: Self = Self(D3DCOMPILE_SKIP_OPTIMIZATION);
    pub const PackMatrixRowMajor: Self = Self(D3DCOMPILE_PACK_MATRIX_ROW_MAJOR);
    pub const PackMatrixColumnMajor: Self = Self(D3DCOMPILE_PACK_MATRIX_COLUMN_MAJOR);
    pub const PartialPrecision: Self = Self(D3DCOMPILE_PARTIAL_PRECISION);
    pub const ForceVSSoftwareNoOpt: Self = Self(D3DCOMPILE_FORCE_VS_SOFTWARE_NO_OPT);
    pub const ForcePSSoftwareNoOpt: Self = Self(D3DCOMPILE_FORCE_PS_SOFTWARE_NO_OPT);
    pub const NoPreshader: Self = Self(D3DCOMPILE_NO_PRESHADER);
    pub const AvoidFlowControl: Self = Self(D3DCOMPILE_AVOID_FLOW_CONTROL);
    pub const EnableStrictness: Self = Self(D3DCOMPILE_ENABLE_STRICTNESS);
    pub const EnableBackwardsCompatiblity: Self = Self(D3DCOMPILE_ENABLE_BACKWARDS_COMPATIBILITY);
    pub const IEEEStrictness: Self = Self(D3DCOMPILE_IEEE_STRICTNESS);
    pub const OptimizationLevel0: Self = Self(D3DCOMPILE_OPTIMIZATION_LEVEL0);
    pub const OptimizationLevel1: Self = Self(D3DCOMPILE_OPTIMIZATION_LEVEL1);
    // pub const OptimizationLevel2: Self = Self(D3DCOMPILE_OPTIMIZATION_LEVEL2);
    pub const OptimizationLevel3: Self = Self(D3DCOMPILE_OPTIMIZATION_LEVEL3);
    pub const WarningsAreErrors: Self = Self(D3DCOMPILE_WARNINGS_ARE_ERRORS);
    pub const ResourcesMayAlias: Self = Self(D3DCOMPILE_RESOURCES_MAY_ALIAS);
    pub const UnboundedDescriptorTables: Self = Self(D3DCOMPILE_ENABLE_UNBOUNDED_DESCRIPTOR_TABLES);
    pub const AllResourcesBound: Self = Self(D3DCOMPILE_ALL_RESOURCES_BOUND);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CompileEffectFlags(u32);
impl_bitflag_operators!(CompileEffectFlags);
#[allow(non_upper_case_globals)]
impl CompileEffectFlags {
    pub const ChildEfect: Self = Self(D3DCOMPILE_EFFECT_CHILD_EFFECT);
    pub const AllowSlowOps: Self = Self(D3DCOMPILE_EFFECT_ALLOW_SLOW_OPS);
}

pub fn compile(
    src_data: &[u8],
    src_name: Option<&str>,
    macros: Option<&[ShaderMacro]>,
    include: Option<ID3DInclude>,
    entry_point: &str,
    target: &str,
    flags1: Option<CompileFlags>,
    flags2: Option<CompileEffectFlags>,
) -> Result<Blob> {
    let c_src_name = src_name.map(|name| CString::new(name).unwrap());

    let mut c_macros: Option<(Vec<_>, Vec<_>)> =
        macros.map(|ms| ms.iter().map(|m| m.to_c_struct()).unzip());

    if let Some((ms, _tmp)) = c_macros.as_mut() {
        ms.push(D3D_SHADER_MACRO {
            Name: PCSTR(null_mut()),
            Definition: PCSTR(null_mut()),
        });
    }

    // let mut include_obj = include.map(|i| IncludeObject::new(i));

    let c_entry_point = CString::new(entry_point)?;
    let c_target = CString::new(target)?;

    let mut blob = None;
    let mut err_blob = None;

    let psourcename = PCSTR(
        c_src_name
            .as_ref()
            .map_or(null(), |name| name.as_ptr() as *const u8),
    );

    let pdefines = if let Some((ms, _tmp)) = c_macros.as_ref() {
        ms.as_ptr()
    } else {
        null()
    };

    const D3D_COMPILE_STANDARD_FILE_INCLUDE: *const ID3DInclude = 1 as *const ID3DInclude;

    // include_obj
    // .as_mut()
    // .map_or(D3D_COMPILE_STANDARD_FILE_INCLUDE, |i| {
    //     i as *mut IncludeObject as *mut ID3DInclude
    // })

    unsafe {
        let res = D3DCompile(
            src_data.as_ptr() as *const c_void,
            src_data.len(),
            psourcename,
            pdefines,
            None, // InParam::owned(include),
            PCSTR(c_entry_point.as_ptr() as *const u8),
            PCSTR(c_target.as_ptr() as *const u8),
            flags1.map_or(0, |f| f.0),
            flags2.map_or(0, |f| f.0),
            &mut blob,
            &mut err_blob,
        );
        if res.is_err() {
            bail!("{:?}: {:?}", res.err(), err_blob);
        } else {
            Ok(Blob(blob.unwrap()))
        }
    }
}
