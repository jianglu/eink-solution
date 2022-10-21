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

use anyhow::*;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub fn copy_to_output(path: &str, target_dir: &str, build_type: &str) -> Result<()> {
    let mut options = CopyOptions::new();
    let mut from_path = Vec::new();
    let out_path = format!("{}\\{}\\", target_dir, build_type);

    println!("cargo:warning=out_path is {:?}", out_path);

    // Overwrite existing files with same name
    options.overwrite = true;
    options.copy_inside = true;

    from_path.push(path);
    copy_items(&from_path, &out_path, &options)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=dlls/EinkTcon.dll");

    let profile = &env::var("PROFILE").unwrap();
    let out_dir = &env::var("OUT_DIR").unwrap();

    let target_dir = match find_target_dir(Path::new(out_dir)) {
        TargetDir::Path(target_dir) => target_dir.to_str().unwrap().to_owned(),
        TargetDir::Unknown => bail!("Cannot find target dir"),
    };

    copy_to_output("dlls/EinkTcon.dll", &target_dir, profile).expect("Could not copy");

    Ok(())
}

pub(crate) enum TargetDir {
    Path(PathBuf),
    Unknown,
}

pub(crate) fn find_target_dir(out_dir: &Path) -> TargetDir {
    if let Some(target_dir) = env::var_os("CARGO_TARGET_DIR") {
        let target_dir = PathBuf::from(target_dir);
        if target_dir.is_absolute() {
            return TargetDir::Path(target_dir);
        } else {
            return TargetDir::Unknown;
        };
    }

    // fs::canonicalize on Windows produces UNC paths which cl.exe is unable to
    // handle in includes.
    // https://github.com/rust-lang/rust/issues/42869
    // https://github.com/alexcrichton/cc-rs/issues/169
    let mut also_try_canonical = cfg!(not(windows));

    let mut dir = out_dir.to_owned();
    loop {
        if dir.join(".rustc_info.json").exists()
            || dir.join("CACHEDIR.TAG").exists()
            || dir.file_name() == Some(OsStr::new("target"))
                && dir
                    .parent()
                    .map_or(false, |parent| parent.join("Cargo.toml").exists())
        {
            return TargetDir::Path(dir);
        }
        if dir.pop() {
            continue;
        }
        if also_try_canonical {
            if let std::result::Result::Ok(canonical_dir) = out_dir.canonicalize() {
                dir = canonical_dir;
                also_try_canonical = false;
                continue;
            }
        }
        return TargetDir::Unknown;
    }
}
