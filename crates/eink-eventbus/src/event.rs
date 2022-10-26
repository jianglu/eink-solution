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

use crate::TopicKey;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

/// An `Event` for passing
pub struct Event<T> {
    pub(crate) topic: TopicKey,
    pub(crate) message: T,
}

impl<T> Event<T> {
    /// create an new event
    pub fn new<K: Into<TopicKey>>(topic_key: K, message: T) -> Self {
        Self {
            topic: topic_key.into(),
            message,
        }
    }

    /// into inner message
    pub fn into_inner(self) -> T {
        self.message
    }
}

impl<T> Deref for Event<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.message
    }
}

impl<T> DerefMut for Event<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.message
    }
}

impl<T: Debug> Debug for Event<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(format!("Event<{}>", std::any::type_name::<T>()).as_str())
            .field("topic", &self.topic)
            .field("message", &&self.message)
            .finish()
    }
}

impl<T: Clone> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self {
            topic: self.topic.clone(),
            message: self.message.clone(),
        }
    }
}
