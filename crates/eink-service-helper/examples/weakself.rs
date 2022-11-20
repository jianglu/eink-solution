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

use std::sync::{Arc, Weak};

use anyhow::Result;
use parking_lot::Mutex;
use weak_self::WeakSelf;

pub struct Foo {
    weak_self: WeakSelf<Mutex<Foo>>,
}

impl Foo {
    pub fn new() -> Arc<Mutex<Foo>> {
        let foo = Arc::new(Mutex::new(Foo {
            weak_self: WeakSelf::new(),
        }));
        foo.lock().weak_self.init(&foo);
        foo
    }

    pub fn do_something(&mut self) {
        println!("Foo::do_something");
    }

    pub fn start(&mut self) {
        let weak_ref = self.weak();
        let _thr = std::thread::spawn(move || {
            if let Some(this) = weak_ref.upgrade() {
                this.lock().do_something();
            } else {
                // Self is disposed
            }
        });
    }

    fn weak(&self) -> Weak<Mutex<Self>> {
        self.weak_self.get()
    }
}

/// 服务助手程序
fn main() -> Result<()> {
    let foo = Foo::new();
    foo.lock().start();
    std::thread::sleep(std::time::Duration::from_secs(2));
    Ok(())
}
