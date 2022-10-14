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

mod capture;
mod cli;
mod d3d;
mod display_info;
mod logger;
mod window_info;

use anyhow::bail;
use clap::Parser;
use cli::CaptureMode;
use eink_composer_lib::SurfaceComposerClient;
use log::{error, info};
use widestring::{U16CString, U16String};
use windows::core::{IInspectable, Interface, Result, HSTRING, PCSTR, PCWSTR, PWSTR};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapEncoder, BitmapPixelFormat};
use windows::Storage::{CreationCollisionOption, FileAccessMode, StorageFolder};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Direct3D::{WKPDID_CommentStringW, WKPDID_D3DDebugObjectName};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device4, ID3D11RenderTargetView, ID3D11Resource, ID3D11Texture2D, D3D11_BIND_FLAG,
    D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_RENDER_TARGET_VIEW_DESC, D3D11_RESOURCE_MISC_FLAG,
    D3D11_RTV_DIMENSION_TEXTURE2D, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::{
    IDXGIKeyedMutex, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE,
};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, HMONITOR, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::Console::GetConsoleWindow;
use windows::Win32::System::Threading::{
    CreateProcessW, CREATE_NEW_CONSOLE, NORMAL_PRIORITY_CLASS, PROCESS_INFORMATION, STARTUPINFOW,
};
use windows::Win32::System::WinRT::{
    Graphics::Capture::IGraphicsCaptureItemInterop, RoInitialize, RO_INIT_MULTITHREADED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetClassNameA, GetDesktopWindow, GetWindowLongA, GetWindowLongW,
    GetWindowRect, GetWindowTextA, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
    RealGetWindowClassA, SetWindowTextA, SetWindowTextW, ShowWindow, GA_ROOT, GET_ANCESTOR_FLAGS,
    GWL_STYLE, SW_SHOWMINIMIZED, WS_VISIBLE,
};

use capture::enumerate_capturable_windows;
use display_info::enumerate_displays;
use std::collections::HashSet;
use std::ffi::{c_void, CStr, CString};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use window_info::WindowInfo;
use windows::{s, w};

//var hwnd = GetAncestor(findHwnd, GetAncestorFlags.GA_ROOT);

fn get_window_ancestor(hwnd: HWND) -> anyhow::Result<HWND> {
    unsafe {
        return Ok(GetAncestor(hwnd, GA_ROOT));
    }
}

fn get_window_class(hwnd: HWND) -> anyhow::Result<String> {
    unsafe {
        let mut buf: [u8; 256] = std::mem::zeroed();
        GetClassNameA(hwnd, &mut buf);
        let class_name = CStr::from_bytes_with_nul_unchecked(&buf);
        return Ok(class_name
            .to_str()?
            .trim_end_matches(|c: char| c == '\0')
            .to_string());
    }
}

fn get_window_real_class(hwnd: HWND) -> anyhow::Result<String> {
    unsafe {
        let mut buf: [u8; 256] = std::mem::zeroed();
        RealGetWindowClassA(hwnd, &mut buf);
        let class_name = CStr::from_bytes_with_nul_unchecked(&buf);
        return Ok(class_name
            .to_str()?
            .trim_end_matches(|c: char| c == '\0')
            .to_string());
    }
}

fn get_window_text(hwnd: HWND) -> anyhow::Result<String> {
    unsafe {
        let mut buf: [u8; 256] = std::mem::zeroed();
        GetWindowTextA(hwnd, &mut buf);
        let win_text = CStr::from_bytes_with_nul_unchecked(&buf);
        return Ok(win_text
            .to_str()?
            .trim_end_matches(|c: char| c == '\0')
            .to_string());
    }
}

/// 查找所有有效窗口
/// 1. 必须是顶层窗口，Ancestor 等于自身
/// 2. 必须时可见窗口
fn find_all_windows() -> HashSet<isize> {
    unsafe extern "system" fn enum_hwnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let mut hwnds = Box::from_raw(lparam.0 as *mut HashSet<isize>);

        let hwnd_ancestor = GetAncestor(hwnd, GA_ROOT);
        if hwnd_ancestor == hwnd {}

        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let visible = (style & WS_VISIBLE.0) == WS_VISIBLE.0;

        if visible {
            hwnds.insert(hwnd.0);
        }

        std::mem::forget(hwnds);
        BOOL(1)
    }

    let boxed_hwnds = Box::new(HashSet::<isize>::new());
    let boxed_hwnds_ptr = Box::into_raw(boxed_hwnds) as isize;

    unsafe {
        EnumWindows(Some(enum_hwnd), LPARAM(boxed_hwnds_ptr));
        let hwnds = Box::from_raw(boxed_hwnds_ptr as *mut HashSet<isize>);
        return *hwnds;
    }
}

fn create_capture_item_for_cmdline(cmdline: &str) -> Result<(HWND, GraphicsCaptureItem)> {
    let mut cmdline16 = U16CString::from_str(&cmdline).unwrap();

    let cmdline_path = PathBuf::from(&cmdline);
    let curr_dir = cmdline_path.parent().unwrap().to_str().unwrap();
    let curr_dir16 = U16String::from_str(curr_dir);

    info!("cmdline = {}", &cmdline);
    info!("curr_dir = {}", &curr_dir);

    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    // si.lpDesktop = PWSTR::from_raw(desktop_name.as_mut_ptr());
    si.lpDesktop = PWSTR::from_raw(w!("winsta0\\default").as_ptr() as *mut u16);
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let hwnds_before = find_all_windows();

    let ret = unsafe {
        CreateProcessW(
            None,
            PWSTR::from_raw(cmdline16.as_mut_ptr()),
            None,
            None,
            false,
            NORMAL_PRIORITY_CLASS | CREATE_NEW_CONSOLE,
            None,
            None,
            &si as *const STARTUPINFOW as *mut STARTUPINFOW,
            &mut pi,
        )
    };

    info!("pi.dwProcessId = {}", pi.dwProcessId);
    info!("pi.dwThreadId = {}", pi.dwThreadId);

    let sys_time = std::time::SystemTime::now();
    let second_10 = std::time::Duration::from_secs(10);

    let interop =
        windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().unwrap();

    'outter: loop {
        let hwnds_after = find_all_windows();

        if hwnds_after.len() > 0 {
            for hwnd in hwnds_after {
                if hwnds_before.contains(&hwnd) {
                    continue;
                }

                // let title = get_window_text(HWND(hwnd)).unwrap();
                // let class = get_window_class(HWND(hwnd)).unwrap();
                // let real_class = get_window_real_class(HWND(hwnd)).unwrap();

                // let mut process_id: u32 = 0;
                // GetWindowThreadProcessId(HWND(hwnd), Some(&mut process_id));
                // info!(
                //     "Window {}, ProcessId: {}, Title: {}, Class: {} / {}",
                //     hwnd, process_id, &title, &class, &real_class
                // );

                error!("interop.CreateForWindow(HWND({:?}))", hwnd);

                let result: ::windows::core::Result<GraphicsCaptureItem> =
                    unsafe { interop.CreateForWindow(HWND(hwnd)) };

                if result.is_err() {
                    error!("interop.CreateForWindow error: {:?}", result.unwrap_err());
                    continue;
                }

                let item_ret = result;
                if item_ret.is_err() {
                    error!("item_ret.is_err(): {:?}", item_ret.unwrap_err());
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }

                let item = item_ret.unwrap();

                let size_ret = item.Size();

                if size_ret.is_err() {
                    error!("size_ret.is_err()");
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }

                let size = size_ret.unwrap();
                info!(
                    "size.Width == {} && size.Height == {}",
                    size.Width, size.Height
                );

                if size.Width == 0 && size.Height == 0 {
                    error!("size.Width == 0 && size.Height == 0");
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }

                // 窗口尺寸正常，但是这可能不稳定，100ms 后再次查询
                std::thread::sleep(std::time::Duration::from_millis(100));

                let size_ret = item.Size();

                if size_ret.is_err() {
                    error!("size_ret.is_err()");
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }

                let size = size_ret.unwrap();
                info!(
                    "size.Width == {} && size.Height == {}",
                    size.Width, size.Height
                );

                if size.Width == 0 && size.Height == 0 {
                    error!("size.Width == 0 && size.Height == 0");
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }

                // DEBUG: 关闭窗口
                // PostThreadMessageA(pi.dwThreadId, WM_QUIT, None, None);
                return Ok((HWND(hwnd), item));
            }
        }

        // 大于 10‘s 还未启动，启动失败，退出
        if sys_time.elapsed().unwrap() > second_10 {
            break 'outter;
        }

        // sleep 1 millis
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    Err(windows::core::Error::from_win32())
}

fn create_capture_item_for_window(window_handle: HWND) -> Result<GraphicsCaptureItem> {
    unsafe {
        let mut buf: [u8; 256] = std::mem::zeroed();
        GetWindowTextA(window_handle, &mut buf);
        let win_text = CStr::from_bytes_with_nul_unchecked(&buf);
        info!("Window Text: {}", win_text.to_str().unwrap());

        let console_hwnd = GetConsoleWindow();

        let new_title = format!("Capturer: {}", win_text.to_str().unwrap());
        let new_title16 = U16String::from_str(&new_title);
        SetWindowTextW(console_hwnd, PCWSTR::from_raw(new_title16.as_ptr()));
    }

    info!("Window Text: {:?}", get_window_text(window_handle));
    info!("Window Class Name: {:?}", get_window_class(window_handle));
    info!(
        "Window Real Class Name: {:?}",
        get_window_real_class(window_handle)
    );
    info!(
        "Window Ancestor Class Name: {:?}",
        get_window_class(get_window_ancestor(window_handle).unwrap())
    );
    info!(
        "Window Ancestor Real Class Name: {:?}",
        get_window_real_class(get_window_ancestor(window_handle).unwrap())
    );

    //var hwnd = GetAncestor(findHwnd, GetAncestorFlags.GA_ROOT);

    info!("create_capture_item_for_window: {:?}", window_handle);
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    let result = unsafe { interop.CreateForWindow(window_handle) };
    info!("create_capture_item_for_window 2: {:?}", result);
    result
}

fn create_capture_item_for_monitor(monitor_handle: HMONITOR) -> Result<GraphicsCaptureItem> {
    info!("create_capture_item_for_monitor: {:?}", monitor_handle);
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    let result = unsafe { interop.CreateForMonitor(monitor_handle) };
    info!("create_capture_item_for_monitor 2: {:?}", result);
    result
}

fn main() -> Result<()> {
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED)?;
    }

    logger::init();
    log::set_max_level(log::LevelFilter::Trace);

    unsafe {
        let console_hwnd = GetConsoleWindow();
        info!("console_hwnd: {:?}", console_hwnd);
        ShowWindow(console_hwnd, SW_SHOWMINIMIZED);
    }

    let args = cli::Args::parse();
    let mode = CaptureMode::from_args(&args);
    let mut hwnd: Option<HWND> = None;

    let item = match mode {
        CaptureMode::CommandLine(cmdline) => {
            let (h, item) = create_capture_item_for_cmdline(&cmdline)?;
            hwnd = Some(h);
            item
        }
        CaptureMode::WindowId(hwnd) => create_capture_item_for_window(HWND(hwnd))?,
        CaptureMode::WindowTitle(query) => {
            let window = get_window_from_query(&query)?;
            hwnd = Some(window.handle);
            create_capture_item_for_window(window.handle)?
        }
        CaptureMode::Monitor(id) => {
            let displays = enumerate_displays()?;
            if id == 0 {
                info!("Invalid input, ids start with 1.");
                std::process::exit(1);
            }
            let index = (id - 1) as usize;
            if index >= displays.len() {
                info!("Invalid input, id is higher than the number of displays!");
                std::process::exit(1);
            }
            let display = &displays[index];
            create_capture_item_for_monitor(display.handle)?
        }
        CaptureMode::Primary => {
            let monitor_handle =
                unsafe { MonitorFromWindow(GetDesktopWindow(), MONITOR_DEFAULTTOPRIMARY) };
            create_capture_item_for_monitor(monitor_handle)?
        }
    };

    take_screenshot(
        hwnd,
        &item,
        args.x.unwrap_or_default(),
        args.y.unwrap_or_default(),
    )?;

    Ok(())
}

fn is_windows_visible(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongA(hwnd, GWL_STYLE) } as u32;
    return (style & WS_VISIBLE.0) == WS_VISIBLE.0;
}

fn take_screenshot(hwnd: Option<HWND>, item: &GraphicsCaptureItem, x: i32, y: i32) -> Result<()> {
    info!("take_screenshot");

    // 链接到 SurfaceFlinger
    info!("SurfaceComposerClient::new()");
    let mut composer = SurfaceComposerClient::new().unwrap();

    let item_size = item.Size()?;
    info!("item_size: {:?}", item_size);

    info!("d3d::create_d3d_device()");
    let d3d_device = match d3d::create_d3d_device() {
        Ok(v) => v,
        Err(err) => {
            error!("d3d::create_d3d_device error {:?}", &err);
            return Err(err);
        }
    };
    let d3d_context = unsafe {
        let mut d3d_context = None;
        d3d_device.GetImmediateContext(&mut d3d_context);
        d3d_context.unwrap()
    };

    let device = match d3d::create_direct3d_device(&d3d_device) {
        Ok(v) => v,
        Err(err) => {
            error!("d3d::create_direct3d_device error {:?}", &err);
            return Err(err);
        }
    };

    let frame_pool = match Direct3D11CaptureFramePool::CreateFreeThreaded(
        &device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        1,
        item_size,
    ) {
        Ok(v) => v,
        Err(err) => {
            error!(
                "Direct3D11CaptureFramePool::CreateFreeThreaded error {:?}",
                &err
            );
            return Err(err);
        }
    };

    info!("frame_pool.CreateCaptureSession");
    let mut session = frame_pool.CreateCaptureSession(item)?;

    session.SetIsCursorCaptureEnabled(true)?;
    session.SetIsBorderRequired(false)?;

    let (sender, receiver) = channel();
    frame_pool.FrameArrived(
        &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
            move |frame_pool, _| {
                let frame_pool = frame_pool.as_ref().unwrap();
                let frame = frame_pool.TryGetNextFrame()?;
                sender.send(frame).unwrap();
                Ok(())
            }
        }),
    )?;
    session.StartCapture()?;
    info!("StartCapture");

    let mut opt_dst: Option<(ID3D11Resource, ID3D11RenderTargetView)> = None;

    use eink_composer_lib::Surface;
    let mut opt_surface: Option<Surface> = None;

    let mut x = 0;
    let mut y = 0;

    let mut starting = true;

    loop {
        unsafe {
            // 30FPS
            let frame_res = receiver.recv_timeout(Duration::from_millis(1000 / 30));

            if let Err(err) = frame_res {
                // error!("RecvError: {:?}", err);
                // 超时，判断窗口是否存在
                if let Some(hwnd) = hwnd {
                    info!("RecvTimeout，IsWindow(hwnd): {:?}", IsWindow(hwnd));
                    if IsWindow(hwnd) == BOOL(0) {
                        error!("Windows is exist: exit");
                        break;
                    }
                }
                continue;
            }

            let frame = frame_res.unwrap();

            // info!("Receive frame");

            let content_size = frame.ContentSize()?;

            let frame_surface = frame.Surface()?;

            let source_texture: ID3D11Texture2D =
                d3d::get_d3d_interface_from_object(&frame_surface)?;

            let mut desc = D3D11_TEXTURE2D_DESC::default();
            source_texture.GetDesc(&mut desc);

            let item_size = item.Size()?;

            let mut position_updated = false;
            let mut size_updated = false;

            if hwnd.is_some() {
                let mut rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd.unwrap(), &mut rect as *const RECT as *mut RECT);

                if x != rect.left || y != rect.top {
                    x = rect.left;
                    y = rect.top;
                    position_updated = true;
                }
            }

            if item_size.Width != desc.Width as i32 || item_size.Height != desc.Height as i32 {
                size_updated = true;
            }

            if position_updated || size_updated {
                info!(
                    "X,Y: {}, {}, ContentSize: {:?}, TextureSize: {}x{}, ItemSize: {:?}",
                    x, y, content_size, desc.Width, desc.Height, item_size
                );

                if size_updated {
                    let recreate_res = frame_pool.Recreate(
                        &device,
                        DirectXPixelFormat::B8G8R8A8UIntNormalized,
                        1,
                        item_size,
                    );

                    if recreate_res.is_err() {
                        info!("Recreate failed, exit capturer");
                        break;
                    }
                }

                if opt_surface.is_some() {
                    composer
                        .move_surface(
                            opt_surface.as_mut().unwrap(),
                            x,
                            y,
                            item_size.Width,
                            item_size.Height,
                        )
                        .unwrap();
                }
            }

            if opt_dst.is_none() {
                // 创建表面 Surface
                let surface = composer
                    .create_surface(0, 0, content_size.Width as i32, content_size.Height as i32)
                    .unwrap();

                info!("OpenSharedResourceByName({})", &surface.shared_texture_name);

                let d3d_device = d3d_device.cast::<ID3D11Device4>()?;
                let dst_resource: ID3D11Resource = d3d_device.OpenSharedResourceByName(
                    &HSTRING::from(&surface.shared_texture_name),
                    DXGI_SHARED_RESOURCE_READ + DXGI_SHARED_RESOURCE_WRITE,
                )?;

                let mut view_desc = D3D11_RENDER_TARGET_VIEW_DESC::default();
                view_desc.ViewDimension = D3D11_RTV_DIMENSION_TEXTURE2D;
                view_desc.Anonymous.Texture2D.MipSlice = 0;
                view_desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;

                let dst_rtv = d3d_device.CreateRenderTargetView(&dst_resource, Some(&view_desc))?;

                opt_dst = Some((dst_resource, dst_rtv));
                opt_surface = Some(surface);
            }

            let dst = opt_dst.as_ref().unwrap();

            let dst_res = &dst.0;
            let dst_rtv = &dst.1;

            let dst_keyed_mutex = dst_res.cast::<IDXGIKeyedMutex>()?;
            // info!("IDXGIKeyedMutex ({:?})", dst_keyed_mutex);

            const INFINITE: u32 = 0xFFFFFFFF; // Infinite timeout

            dst_keyed_mutex.AcquireSync(0, INFINITE)?;

            let clear_color = [0.0f32, 0.0, 0.0, 0.0];
            d3d_context.ClearRenderTargetView(dst_rtv, &clear_color as *const f32);

            // ctx.CopyResource(&dst_resource, src_texture2d);
            d3d_context.CopySubresourceRegion(
                dst_res,
                0,
                0,
                0,
                0,
                &source_texture,
                0, //
                None,
            );

            // let s = "JIANG LU".as_bytes();
            // source_texture.SetPrivateData(
            //     &WKPDID_CommentStringW,
            //     s.len() as u32,
            //     s.as_ptr() as *const c_void,
            // );

            dst_keyed_mutex.ReleaseSync(0);
        }
    }

    // let texture = unsafe {
    //     let frame = receiver.recv().unwrap();

    //     let source_texture: ID3D11Texture2D =
    //         d3d::get_d3d_interface_from_object(&frame.Surface()?)?;
    //     let mut desc = D3D11_TEXTURE2D_DESC::default();
    //     source_texture.GetDesc(&mut desc);
    //     desc.BindFlags = D3D11_BIND_FLAG(0);
    //     desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0);
    //     desc.Usage = D3D11_USAGE_STAGING;
    //     desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    //     let copy_texture = { d3d_device.CreateTexture2D(&desc, std::ptr::null())? };

    //     d3d_context.CopyResource(&copy_texture, &source_texture);

    //     session.Close()?;
    //     frame_pool.Close()?;

    //     copy_texture
    // };

    // let bits = unsafe {
    //     let mut desc = D3D11_TEXTURE2D_DESC::default();
    //     texture.GetDesc(&mut desc as *mut _);

    //     let resource: ID3D11Resource = texture.cast()?;
    //     let mapped = d3d_context.Map(&resource, 0, D3D11_MAP_READ, 0)?;

    //     // Get a slice of bytes
    //     let slice: &[u8] = {
    //         std::slice::from_raw_parts(
    //             mapped.pData as *const _,
    //             (desc.Height * mapped.RowPitch) as usize,
    //         )
    //     };

    //     let bytes_per_pixel = 4;
    //     let mut bits = vec![0u8; (desc.Width * desc.Height * bytes_per_pixel) as usize];
    //     for row in 0..desc.Height {
    //         let data_begin = (row * (desc.Width * bytes_per_pixel)) as usize;
    //         let data_end = ((row + 1) * (desc.Width * bytes_per_pixel)) as usize;
    //         let slice_begin = (row * mapped.RowPitch) as usize;
    //         let slice_end = slice_begin + (desc.Width * bytes_per_pixel) as usize;
    //         bits[data_begin..data_end].copy_from_slice(&slice[slice_begin..slice_end]);
    //     }

    //     d3d_context.Unmap(&resource, 0);

    //     bits
    // };

    // let path = std::env::current_dir()
    //     .unwrap()
    //     .to_string_lossy()
    //     .to_string();
    // let folder = StorageFolder::GetFolderFromPathAsync(&HSTRING::from(path))?.get()?;
    // let file = folder
    //     .CreateFileAsync(
    //         w!("screenshot.png"),
    //         CreationCollisionOption::ReplaceExisting,
    //     )?
    //     .get()?;

    // {
    //     let stream = file.OpenAsync(FileAccessMode::ReadWrite)?.get()?;
    //     let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId()?, &stream)?.get()?;
    //     encoder.SetPixelData(
    //         BitmapPixelFormat::Bgra8,
    //         BitmapAlphaMode::Premultiplied,
    //         item_size.Width as u32,
    //         item_size.Height as u32,
    //         1.0,
    //         1.0,
    //         &bits,
    //     )?;

    //     encoder.FlushAsync()?.get()?;
    // }

    Ok(())
}

fn get_window_from_query(query: &str) -> Result<WindowInfo> {
    let windows = find_window(query);

    info!("Find window, query: '{}', size: {}", query, windows.len());

    let window = if windows.len() == 0 {
        info!("No window matching '{}' found!", query);
        std::process::exit(1);
    } else if windows.len() == 1 {
        &windows[0]
    } else {
        info!(
            "{} windows found matching '{}', please select one:",
            windows.len(),
            query
        );
        info!("    Num       PID     HWND     Window Title");
        for (i, window) in windows.iter().enumerate() {
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(window.handle, Some(&mut pid)) };
            info!(
                "    {:>3}    {:>6}    {:>6}    {}",
                i, pid, window.handle.0, window.title
            );
        }
        let index: usize;
        loop {
            print!("Please make a selection (q to quit): ");
            std::io::stdout().flush().unwrap();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            if input.to_lowercase().contains("q") {
                std::process::exit(0);
            }
            let input = input.trim();
            let selection: Option<usize> = match input.parse::<usize>() {
                Ok(selection) => {
                    if selection < windows.len() {
                        Some(selection)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(selection) = selection {
                index = selection;
                break;
            } else {
                info!("Invalid input, '{}'!", input);
                continue;
            };
        }
        &windows[index]
    };

    Ok(window.clone())
}

fn find_window(window_name: &str) -> Vec<WindowInfo> {
    let window_name_query = &window_name.to_string().to_lowercase();

    let window_list = enumerate_capturable_windows();
    let mut windows: Vec<WindowInfo> = Vec::new();
    for window_info in window_list.into_iter() {
        let title = window_info.title.to_lowercase();
        info!("{}", &title);
        if window_name_query.starts_with("*") {
            if title.contains(window_name_query) {
                windows.push(window_info.clone());
            }
        } else {
            if title.eq(window_name_query) {
                windows.push(window_info.clone());
            }
        }
    }
    windows
}
