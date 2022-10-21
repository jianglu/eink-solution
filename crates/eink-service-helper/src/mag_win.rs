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
    core::{HSTRING, PCWSTR},
    w,
    Win32::{
        Foundation::{BOOL, HINSTANCE, HWND, POINT, RECT, SIZE},
        UI::{
            Magnification::{
                MagSetWindowSource, MagSetWindowTransform, MAGTRANSFORM,
            },
            WindowsAndMessaging::{
                CreateWindowExW, SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE,
                WINDOW_EX_STYLE, WINDOW_STYLE, WS_CHILD, WS_EX_COMPOSITED, WS_VISIBLE,
            },
        },
    },
};
pub struct MagWindow {
    _hwnd: HWND,
    _mag_factor: f32,
    _window_position: POINT,
    _window_size: SIZE,

    // Rectangle of screen that is centered at the mouse coordinates to be magnified.
    _source_rect: RECT,
}

impl MagWindow {
    /// 创建放大镜窗口
    pub fn new(mag_factor: f32, window_position: POINT, window_size: SIZE) -> Result<Self> {
        Ok(Self {
            _hwnd: HWND(0),
            _mag_factor: mag_factor,
            _window_size: window_size,
            _window_position: window_position,
            _source_rect: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
        })
    }

    pub fn update_source_rect(
        &mut self,
        mouse_point: &mut POINT,
        pan_offset: POINT,
        window_size: SIZE,
    ) -> Result<()> {
        self._source_rect.left = mouse_point.x + pan_offset.x
            - ((window_size.cx as f32 / 2f32) / self._mag_factor) as i32;
        self._source_rect.top = mouse_point.y + pan_offset.y
            - ((window_size.cy as f32 / 2f32) / self._mag_factor) as i32;
        self._source_rect.right =
            mouse_point.x + ((window_size.cx as f32 / 2f32) / self._mag_factor) as i32;
        self._source_rect.bottom =
            mouse_point.y + ((window_size.cy as f32 / 2f32) / self._mag_factor) as i32;
        Ok(())
    }

    pub fn set_magnification_factor_internal(&mut self, mag_factor: f32) -> bool {
        let mut matrix: MAGTRANSFORM = unsafe { zeroed() };
        matrix.v[0] = mag_factor;
        matrix.v[1 * 3 + 1] = mag_factor;
        matrix.v[2 * 3 + 2] = 1.0f32;

        // TODO Avoid race condition where calls to UpdateSourceRect + UpdateMagnifier happen with new magFactor, but before MagSetWindowTransform is called
        //      This is isn't a problem though since this method is always assumed to be called a non-active magWindow
        self._mag_factor = mag_factor;

        unsafe { MagSetWindowTransform(self._hwnd, &mut matrix).as_bool() }
    }

    pub fn create(&mut self, inst: HINSTANCE, hwnd_host: HWND, visible: bool) -> bool {
        let mut style: u32 = 0u32;

        style |= WS_CHILD.0; // Required for magnification window
        style |= WS_EX_COMPOSITED.0; // Double-buffered

        if visible {
            style |= WS_VISIBLE.0;
        }

        // Magnifier window class name defined in magnification.h
        let class_name = w!("Magnifier"); // WC_MAGNIFIER
        let window_title = w!("MagnifierWindow2");

        unsafe {
            self._hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR::from(class_name),
                PCWSTR::from(window_title),
                WINDOW_STYLE(style),
                self._window_position.x,
                self._window_position.y,
                self._window_size.cx,
                self._window_size.cy,
                hwnd_host,
                None,
                inst,
                None,
            );
        }

        if self._hwnd == HWND(0) {
            return false;
        }

        self.set_magnification_factor_internal(self._mag_factor)
    }

    pub fn get_handle(&self) -> HWND {
        return self._hwnd;
    }

    pub fn set_magnification_factor(&mut self, magFactor: f32) -> bool {
        if self._mag_factor != 0f32 && self._mag_factor == magFactor {
            return false;
        }
        self.set_magnification_factor_internal(magFactor)
    }

    pub fn set_size(&mut self, width: i32, height: i32) -> bool {
        if self._window_size.cx == width && self._window_size.cy == height {
            return false;
        }

        self._window_size.cx = width;
        self._window_size.cy = height;

        unsafe {
            SetWindowPos(
                self._hwnd,
                HWND_TOP,
                self._window_position.x,
                self._window_position.y,
                self._window_size.cx,
                self._window_size.cy,
                SWP_NOACTIVATE | SWP_NOMOVE,
            )
            .as_bool()
        }
    }

    pub fn update_magnifier(
        &mut self,
        mouse_point: &mut POINT,
        pan_offset: POINT,
        window_size: SIZE,
    ) -> bool {
        self.update_source_rect(mouse_point, pan_offset, window_size);

        // Set the source rectangle for the magnifier control.
        unsafe { MagSetWindowSource(self._hwnd, self._source_rect).as_bool() }
    }
}
