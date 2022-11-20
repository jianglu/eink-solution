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

use std::{
    fs::File,
    io::{Read, Write},
};

fn main() {
    let shadow = shadow_rs::Shadow::build().unwrap();

    // shadow_rs::branch();
    // shadow_rs::tag();
    // shadow_rs::build_channel().to_string();

    println!("cargo:warning={}", shadow_rs::is_debug());

    for (k, v) in shadow.map.iter() {
        println!("cargo:warning=[{k}] : {:?}", &v);
    }

    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    println!("cargo:cargo:rerun-if-changed=app.manifest.rc");
    println!("cargo:warning=out_dir: {out_dir:?}");

    let dest_path = format!("{}/app.manifest.rc", out_dir.to_str().unwrap());

    let mut source = File::open("./app.manifest.rc").unwrap();
    let mut dest = File::create(&dest_path).unwrap();

    let mut data = String::new();
    source.read_to_string(&mut data).unwrap();
    drop(source);

    // display build current project version, live: "0.1.0"
    let pkg_version_major = shadow.map.get("PKG_VERSION_MAJOR").unwrap();
    let pkg_version_minor = shadow.map.get("PKG_VERSION_MINOR").unwrap();
    let pkg_version_patch = shadow.map.get("PKG_VERSION_PATCH").unwrap();

    // display current short commit_id, like: "2d98bc71"
    let short_commit = shadow.map.get("SHORT_COMMIT").unwrap();

    // "2022-11-19T10:38:20Z" ->  "2022,11,19,103820"
    let commit_date = shadow.map.get("COMMIT_DATE_3339").unwrap();
    let file_version = commit_date
        .v
        .replace("-", ",")
        .replace("T", ",")
        .replace(":", "")
        .replace("Z", "");

    // FILEVERSION    0,1,0,16384
    // PRODUCTVERSION 1,0,0,0

    let product_version = format!(
        "{},{},{},{}",
        pkg_version_major.v, pkg_version_minor.v, pkg_version_patch.v, short_commit.v
    );

    let data = data.replace("{FILEVERSION}", &file_version);
    let data = data.replace("{PRODUCTVERSION}", &product_version);
    dest.write(data.as_bytes()).unwrap();

    embed_resource::compile(&dest_path);
}
