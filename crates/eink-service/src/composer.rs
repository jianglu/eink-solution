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
    io::{BufRead, BufReader},
    process::Command,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use log::{debug, info};

use winapi::shared::ntdef::NULL;
use winapi::shared::{minwindef::DWORD, ntdef::HANDLE};

use crate::{composer, eink, logger::output_debug_string, win_utils};

const EINK_COMPOSER_NAME: &str = "eink-composer.exe";

/// 合成器管理服务
/// 当前无论哪种模式，合成器都需要开启
/// TODO: 需要对合成器进行更深入的管理
struct ComposerServiceImpl {
    pid: Arc<Mutex<DWORD>>,
}

impl ComposerServiceImpl {
    pub fn new() -> Result<Self> {
        let eink_stable_id = eink::find_eink_display_stable_id()?;
        info!("Eink Stable Monitor Id: {}", eink_stable_id);

        let pid = Arc::new(Mutex::new(0));
        let pid_clone = pid.clone();

        // 创建 eink-composer 进程，并通过匿名管道和 eink-composer 进程建立双向通讯
        std::thread::spawn(move || {
            // cmd_lib::spawn_with_output!(eink-composer.exe --monitor-id $eink_stable_id  --test-background true --test-layer true)
            //     .unwrap()
            //     .wait_with_pipe(&mut |pipe| {
            //         BufReader::new(pipe)
            //             .lines()
            //             .for_each(|line| output_debug_string(&line.unwrap()));
            //     })
            //     .unwrap();

            let curr_dir = std::env::current_dir().unwrap();

            let proc_name = "eink-composer.exe";
            let proc_dir = curr_dir.to_str().unwrap();
            let proc_cmd = &format!(
                "\"{}\\eink-composer.exe\" --monitor-id {} --test-layer true",
                proc_dir, eink_stable_id
            );

            info!("proc_name: {}", proc_name);
            info!("proc_dir: {}", proc_dir);
            info!("proc_cmd: {}", proc_cmd);

            let pid = win_utils::run_system_privilege(proc_name, proc_dir, proc_cmd).unwrap();

            *pid_clone.lock().unwrap() = pid;

            // let mut composer = Command::new(EINK_COMPOSER_NAME)
            //     .arg("--monitor-id")
            //     .arg(&eink_stable_id)
            //     .spawn();

            // let mut composer = match composer {
            //     Ok(composer) => composer,
            //     Err(err) => {
            //         debug!("Composer Error: {:?}", err);
            //         return;
            //     }
            // };

            // let stdout = composer.stdout.take().expect("failed to get stdout");
            // let stderr = composer.stderr.take().expect("failed to get stdout");

            info!("Composer is running !!!");

            // std::thread::spawn(move || {
            //     let reader = std::io::BufReader::new(stdout);
            //     reader
            //         .lines()
            //         .for_each(|l| output_debug_string(&l.unwrap()));
            // });

            // std::thread::spawn(move || {
            //     let reader = std::io::BufReader::new(stderr);
            //     reader
            //         .lines()
            //         .for_each(|l| output_debug_string(&l.unwrap()));
            // });
        });

        Ok(Self { pid })
    }
}

impl Drop for ComposerServiceImpl {
    fn drop(&mut self) {
        let pid = *self.pid.lock().unwrap();
        win_utils::kill_process_by_pid(pid, 0);
    }
}

pub struct ComposerService {
    inner: Arc<Mutex<ComposerServiceImpl>>,
}

impl ComposerService {
    /// 创建合成器服务
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(ComposerServiceImpl::new()?)),
        })
    }
}
