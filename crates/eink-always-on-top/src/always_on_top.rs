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

pub struct AlwaysOnTop {}

/// 窗口置顶系统
impl AlwaysOnTop {
    pub fn new() -> Result<Self> {
        self = Self {};
        dpi_aware::enable_dpi_awareness_for_this_process();

        if (InitMainWindow()) {
            InitializeWinhookEventIds();

            AlwaysOnTopSettings::instance().InitFileWatcher();
            AlwaysOnTopSettings::instance().LoadSettings();

            RegisterHotkey();
            RegisterLLKH();

            SubscribeToEvents();
            StartTrackingTopmostWindows();
        } else {
            Logger::error("Failed to init AlwaysOnTop module");
            // TODO: show localized message
        }

        Ok(this)
    }
}
