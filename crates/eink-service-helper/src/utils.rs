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

use jsonrpc_lite::{Id, JsonRpc};
use std::path::PathBuf;

/// 获得当前 exe 所在目录
pub fn get_current_exe_dir() -> PathBuf {
    let exe_path = std::env::current_exe().expect("Cannot get current exe path from env");
    let exe_dir = exe_path.parent().expect("Current exe path is wrong");
    exe_dir.to_owned().canonicalize().unwrap()
}

/// 获得当前数据存储目录
pub fn get_current_data_dir() -> PathBuf {
    let mut data_dir = dirs::data_local_dir().expect("Cannot get data local dir");
    data_dir.push(&"Lenovo\\ThinkBookEinkPlus\\");
    data_dir
}

/// 返回成功（字符串值）
pub fn jsonrpc_success_string(id: Id, result: &str) -> JsonRpc {
    JsonRpc::success(id, &serde_json::Value::String(result.to_owned()))
}

/// 返回成功（u32值）
pub fn jsonrpc_success_u32(id: Id, result: u32) -> JsonRpc {
    JsonRpc::success(id, &serde_json::json!(result))
}

/// 返回错误（无效参数）
pub fn jsonrpc_error_invalid_params(id: Id) -> JsonRpc {
    JsonRpc::error(id, jsonrpc_lite::Error::invalid_params())
}

/// 返回错误（找不到方法）
pub fn jsonrpc_error_method_not_found(id: Id) -> JsonRpc {
    JsonRpc::error(id, jsonrpc_lite::Error::method_not_found())
}

/// 返回错误（内部错误）
pub fn jsonrpc_error_internal_error(id: Id) -> JsonRpc {
    JsonRpc::error(id, jsonrpc_lite::Error::internal_error())
}
