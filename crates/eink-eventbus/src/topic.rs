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

use crate::{Event, EventListeners, Eventbus, TopicKey};
use std::fmt::{Debug, Formatter};

/// A `Topic` wrapper for a `TopicKey`
pub struct Topic<T> {
    pub(crate) key: TopicKey,
    pub(crate) bus: Eventbus,
    pub(crate) event_listeners: EventListeners<T>,
}

impl<T> Topic<T> {
    /// create an event from message
    pub fn create_event(&self, message: T) -> Event<T> {
        Event::new(self.key.clone(), message)
    }

    /// get the key of a topic
    pub fn get_key(&self) -> &TopicKey {
        &self.key
    }

    /// get the associated eventbus
    pub fn get_bus(&self) -> &Eventbus {
        &self.bus
    }

    /// get event listeners subscribed to this topic
    pub fn get_listeners(&self) -> &EventListeners<T> {
        &self.event_listeners
    }
}

impl<T> Debug for Topic<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(format!("Topic<{}>", std::any::type_name::<T>()).as_str())
            .field("key", &self.key)
            .field("bus", &self.bus)
            .finish()
    }
}
