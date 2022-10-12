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

#![recursion_limit = "1024"]

use anyhow::Result;
use ntapi::winapi::um::winnt::GENERIC_ALL;

use std::{
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
};

use windows::core::{Borrowed, IInspectable, IUnknown, InParam, Interface, HSTRING, PCWSTR};
use windows::w;
use windows::Devices::Display::Core::DisplayManager;
use windows::Devices::Display::Core::DisplayManagerOptions;
use windows::Devices::Display::Core::{
    DisplayFence, DisplayManagerResult, DisplayModeQueryOptions, DisplayPathScaling,
    DisplayPrimaryDescription, DisplayState, DisplayStateApplyOptions, DisplayTarget,
};
use windows::Foundation::{Collections::IIterable, IReference, PropertyValue};
use windows::Graphics::DirectX::{
    Direct3D11::Direct3DMultisampleDescription, DirectXColorSpace, DirectXPixelFormat,
};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::LUID;
use windows::Win32::Foundation::RECT;
use windows::Win32::Foundation::{E_FAIL, LRESULT};
use windows::Win32::Foundation::{
    E_UNEXPECTED, HINSTANCE, HWND, INVALID_HANDLE_VALUE, LPARAM, S_OK, WPARAM,
};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL};
use windows::Win32::Graphics::Direct3D11::D3D11_RTV_DIMENSION_TEXTURE2D;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11Device5, ID3D11DeviceContext, ID3D11DeviceContext4,
    ID3D11Fence, ID3D11Texture2D, D3D11_CREATE_DEVICE_FLAG, D3D11_FENCE_FLAG_SHARED,
    D3D11_RENDER_TARGET_VIEW_DESC, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
};
use windows::Win32::Graphics::Dxgi::CreateDXGIFactory2;
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::Graphics::{
    Dxgi::{IDXGIAdapter4, IDXGIFactory6, DXGI_CREATE_FACTORY_DEBUG},
    Gdi::HBRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateEventW;
use windows::Win32::System::Threading::SetProcessShutdownParameters;
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRect, CreateWindowExW, DestroyCursor, DispatchMessageW, LoadCursorW, PeekMessageW,
    RegisterClassExW, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, HCURSOR, HICON,
    IDC_ARROW, MSG, PM_REMOVE, WINDOW_EX_STYLE, WM_QUERYENDSESSION, WM_QUIT, WM_USER, WNDCLASSEXW,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPEDWINDOW, WS_POPUP,
};
use windows::{core::HRESULT, Devices::Display::Core::DisplayPath};

mod duplicator;
mod iterable;
mod specialized;
mod winrt_ext;

// const TRUE: i32 = 1;
// const FALSE: i32 = 0;

use cpp::cpp;

use crate::winrt_ext::DisplayPathExt;

unsafe fn unsafe_main() -> HRESULT {
    // let mut duplicator = Duplicator::new();

    // Event used by the threads to signal an unexpected error and we want to quit the app
    let unexpected_error_event = CreateEventW(null(), true, false, None).unwrap();
    println!("unexpected_error_event: {:?}", unexpected_error_event);

    if unexpected_error_event == INVALID_HANDLE_VALUE {
        // ProcessFailure(nullptr, L"UnexpectedErrorEvent creation failed", L"Error", E_UNEXPECTED);
        return E_UNEXPECTED;
    }

    // Event for when a thread encounters an expected error
    let expected_error_event = CreateEventW(null(), true, false, None).unwrap();
    println!("expected_error_event: {:?}", expected_error_event);

    if expected_error_event == INVALID_HANDLE_VALUE {
        // ProcessFailure(nullptr, L"ExpectedErrorEvent creation failed", L"Error", E_UNEXPECTED);
        return E_UNEXPECTED;
    }

    // Event to tell spawned threads to quit
    let terminate_threads_event = CreateEventW(null(), true, false, None).unwrap();
    println!("terminate_threads_event: {:?}", terminate_threads_event);

    if terminate_threads_event == INVALID_HANDLE_VALUE {
        // ProcessFailure(nullptr, L"TerminateThreadsEvent creation failed", L"Error", E_UNEXPECTED);
        return E_UNEXPECTED;
    }

    let instance: HINSTANCE = GetModuleHandleW(None).unwrap();
    println!("instance: {:?}", instance);

    // Load simple cursor
    let cursor: HCURSOR = LoadCursorW(instance, IDC_ARROW).unwrap();
    println!("cursor: {:?}", cursor);

    if !cursor.is_invalid() {
        // ProcessFailure(nullptr, L"Cursor load failed", L"Error", E_UNEXPECTED);
        return E_UNEXPECTED;
    }

    let class_name = w!("FusionCompositor");
    println!("class_name: {}", class_name);

    let window_name = w!("FusionCompositor");
    println!("window_name: {}", window_name);

    // Register class
    let mut wc: WNDCLASSEXW = zeroed();
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.style = CS_HREDRAW | CS_VREDRAW;
    wc.lpfnWndProc = Some(wnd_proc);
    wc.cbClsExtra = 0;
    wc.cbWndExtra = 0;
    wc.hInstance = instance;
    wc.hIcon = HICON(0);
    wc.hCursor = cursor;
    wc.hbrBackground = HBRUSH(0);
    wc.lpszMenuName = PCWSTR::null();
    wc.lpszClassName = class_name.into();
    wc.hIconSm = HICON(0);

    if RegisterClassExW(&wc) == 0 {
        // ProcessFailure(nullptr, L"Window class registration failed", L"Error", E_UNEXPECTED);
        return E_UNEXPECTED;
    }

    // Create window
    let mut window_rect: RECT = zeroed();
    window_rect.left = 0;
    window_rect.top = 0;
    window_rect.right = 500;
    window_rect.bottom = 500;

    AdjustWindowRect(&mut window_rect, WS_OVERLAPPEDWINDOW, false);

    // Window
    let hwnd = CreateWindowExW(
        (WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW) as WINDOW_EX_STYLE,
        class_name,
        window_name,
        WS_POPUP,
        0,
        0,
        window_rect.right - window_rect.left,
        window_rect.bottom - window_rect.top,
        None,
        None,
        instance,
        null(),
    );
    println!("CreateWindowExW(instance: {:?}): {:?}", instance, hwnd);

    if hwnd == HWND(0) {
        // ProcessFailure(nullptr, L"Window creation failed", L"Error", E_FAIL);
        return E_FAIL;
    }

    let ret = DestroyCursor(cursor);
    println!("DestroyCursor({:?}): {:?}", cursor, ret);

    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
    let ret = ShowWindow(hwnd, SW_SHOW);
    println!("ShowWindow({:?}, SW_SHOW): {:?}", hwnd, ret);

    let ret = UpdateWindow(hwnd);
    println!("UpdateWindow({:?}): {:?}", hwnd, ret);

    // 设置拦截注销消息的优先级
    SetProcessShutdownParameters(0x3ff, 0);

    let mut msg: MSG = zeroed();

    while WM_QUIT != msg.message {
        // DUPL_RETURN ret = DUPL_RETURN_SUCCESS;
        if PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE).as_bool() {
            println!("PeekMessageW: {}", msg.message);

            const OCCLUSION_STATUS_MSG: u32 = WM_USER;

            if msg.message == OCCLUSION_STATUS_MSG {
                // Present may not be occluded now so try again
                // occluded = false;
            } else if msg.message == WM_QUERYENDSESSION {
                println!("msg.message == WM_QUERYENDSESSION");
                // 用户注销系统.退出
                break;
            } else {
                // Process window messages
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    // Clean up

    CloseHandle(unexpected_error_event);
    CloseHandle(expected_error_event);
    CloseHandle(terminate_threads_event);

    return S_OK;
}

unsafe extern "system" fn wnd_proc(
    param0: HWND,
    param1: u32,
    param2: WPARAM,
    param3: LPARAM,
) -> LRESULT {
    println!("param1: {}", param1);
    return LRESULT(0);
}

fn test_display_target(
    display_manager: &DisplayManager,
    display_target: &DisplayTarget,
    display_state: &DisplayState,
) {
    let display_adapter = display_target.Adapter().unwrap();

    //
    // 获取当前 DisplayTarget 对应的 DisplayPath
    //
    let display_path = display_state.GetPathForTarget(display_target).unwrap();
    println!(
        "DisplayPath.Target: {:?}",
        display_path.Target().unwrap().StableMonitorId().unwrap()
    );
    println!(
        "DisplayPath.SourceResolution: {:?}",
        display_path.SourceResolution()
    );

    use windows::Win32::System::WinRT::Display::IDisplayPathInterop;
    let display_path_interop: IDisplayPathInterop = display_path.cast().unwrap();

    // let source_presentation_handle = unsafe {
    //     display_path_interop
    //         .CreateSourcePresentationHandle()
    //         .unwrap()
    // };

    let source_id = unsafe { display_path_interop.GetSourceId().unwrap() };
    println!("DisplayPathInterop.GetSourceId: {}", source_id);

    let resolution = display_path.SourceResolution().unwrap();
    let resolution = resolution.Value().unwrap();
    println!("DisplayPath.SourceResolution: {:?}", resolution);

    let pixel_format = display_path.SourcePixelFormat().unwrap();
    println!("DisplayPath.PixelFormat: {:?}", pixel_format);

    //
    // 将 WinRT::DisplayAdapter 转换为 DXGIAdapter
    //
    let factory: IDXGIFactory6 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_DEBUG).unwrap() };

    let display_adapter_id = display_adapter.Id().unwrap();

    let adapter_luid = LUID {
        LowPart: display_adapter_id.LowPart,
        HighPart: display_adapter_id.HighPart,
    };

    let dxgi_adapter: IDXGIAdapter4 = unsafe { factory.EnumAdapterByLuid(adapter_luid).unwrap() };

    //
    // 创建 D3D11 设备
    //
    let mut d3d_device_opt: Option<ID3D11Device> = None;
    let mut d3d_device_context_opt: Option<ID3D11DeviceContext> = None;
    let mut feature_levels = [D3D_FEATURE_LEVEL(0); 0];
    let mut feature_level = D3D_FEATURE_LEVEL(0);

    unsafe {
        D3D11CreateDevice(
            &dxgi_adapter,               // pAdapter
            D3D_DRIVER_TYPE_UNKNOWN,     // DriverType
            HINSTANCE(0),                // Software
            D3D11_CREATE_DEVICE_FLAG(0), // Flags
            &feature_levels,             // [in] pFeatureLevels
            D3D11_SDK_VERSION,           // SDKVersion
            &mut d3d_device_opt,         // ppDevice
            &mut feature_level,          // [out] pFeatureLevel
            &mut d3d_device_context_opt,
        )
        .unwrap()
    }; // ppImmediateContext

    let d3d_device = d3d_device_opt.unwrap();
    let d3d_device_5: ID3D11Device5 = d3d_device.cast().unwrap();

    let d3d_device_context = d3d_device_context_opt.unwrap();

    println!("D3DDevice: {:?}", d3d_device_5);
    println!("FeatureLevel: {:?}", feature_level);

    // 创建 Display 同步栅栏
    // D3D11_FENCE_FLAG_SHARED - 同适配器不同上下文
    // D3D11_FENCE_FLAG_SHARED_CROSS_ADAPTER - 不同适配器
    let mut d3d_fence_opt: Option<ID3D11Fence> = None;

    unsafe {
        d3d_device_5
            .CreateFence(0, D3D11_FENCE_FLAG_SHARED, &mut d3d_fence_opt)
            .unwrap();
    }

    let d3d_fence = d3d_fence_opt.unwrap();
    println!("d3d_fence: {:?}", d3d_fence);

    // 创建 ID3D11Fence 的跨进程访问句柄
    let fence_shared_handle = unsafe {
        d3d_fence
            .CreateSharedHandle(null(), GENERIC_ALL, None)
            .unwrap()
    };
    println!("fence_shared_handle: {:?}", fence_shared_handle);

    //
    // 基于 DisplayAdapter 创建 DisplayDevice 逻辑设备
    // DisplayDevice 逻辑设备等效于 D3DDevice，用于创建 Surface、Present 等操作
    //
    let display_device = display_manager
        .CreateDisplayDevice(&display_adapter)
        .unwrap();
    println!("display_device: {:?}", display_device);

    use windows::Win32::System::WinRT::Display::IDisplayDeviceInterop;

    let display_device_interop = display_device.cast::<IDisplayDeviceInterop>().unwrap();

    let display_fence = unsafe {
        display_device_interop
            .OpenSharedHandle(fence_shared_handle, DisplayFence::IID)
            .unwrap()
    };
    println!("display_fence: {:?}", display_fence);

    let display_fence1 = unsafe {
        (display_fence as *mut IUnknown)
            .cast::<DisplayFence>()
            .as_ref()
            .unwrap()
    };
    println!("display_fence1: {:?}", display_fence1);

    let display_fence2 = unsafe { (display_fence as *mut DisplayFence).as_ref().unwrap() };
    println!("display_fence2: {:?}", display_fence2);

    // display_device.CreatePeriodicFence(target, offsetfromvblank);

    //
    // 创建显示设备任务池，处理 Present 任务
    //
    let display_task_pool = display_device.CreateTaskPool().unwrap();
    println!("display_task_pool: {:?}", display_task_pool);

    // 创建 DisplayTarget 可用的显示输入源，DisplaySource 用来描述需要 Render 的对象
    // 相当于 SwapChain 的功能
    let display_source = display_device.CreateScanoutSource(display_target).unwrap();
    println!("display_source: {:?}", display_source);

    ///////////////////////////////////////////////////////////////////////////////////////////////
    //
    // 创建主表面
    //
    let display_width = resolution.Width;
    let display_height = resolution.Height;
    println!("display_width: {:?}", display_width);
    println!("display_height: {:?}", display_height);

    // 配置默认的多重采样参数
    let multisample_desc = Direct3DMultisampleDescription {
        Count: 1,
        Quality: 0,
    };

    // 创建主表面描述符
    let primary_desc = DisplayPrimaryDescription::CreateWithProperties(
        None,
        display_width as u32,
        display_height as u32,
        pixel_format,
        DirectXColorSpace::RgbFullG22NoneP709,
        false,
        multisample_desc,
    )
    .unwrap();
    println!("CreateWithProperties: {:?}", primary_desc);

    // A full-screen primary surface.
    // A 2D pixel buffer that was allocated to be compatible with scanning out to one or
    // more DisplaySource objects.
    // DisplaySurface 表示显示内存，DisplayScanout 表示渲染目标
    let primary_surface = display_device
        .CreatePrimary(display_target, &primary_desc)
        .unwrap();
    println!("CreatePrimary: {:?}", primary_surface);

    let sub_resource_index = 0;
    let sync_interval = 1;
    let primary_scanout = display_device
        .CreateSimpleScanout(
            &display_source,
            &primary_surface,
            sub_resource_index,
            sync_interval,
        )
        .unwrap();
    println!("CreateSimpleScanout: {:?}", primary_scanout);

    // Render Target View
    // auto surfaceRaw = mPrimarySurfaces[surfaceIndex].as<::IInspectable>();

    let surface_raw = primary_surface.cast::<IInspectable>().unwrap();
    println!("surface_raw: {:?}", surface_raw);

    //
    // 创建 DisplaySurface {Texture2D} 的跨 Device 共享句柄
    //
    // winrt::handle surfaceHandle;
    let surface_handle = unsafe {
        display_device_interop
            .CreateSharedHandle(
                InParam::owned(surface_raw),
                null(),      // pSecurityAttributes
                GENERIC_ALL, // Access
                &HSTRING::default(),
            )
            .unwrap()
    };
    println!("surface_handle: {:?}", surface_handle);

    let d3d_surface: ID3D11Texture2D =
        unsafe { d3d_device_5.OpenSharedResource1(surface_handle).unwrap() };
    println!("d3d_surface: {:?}", d3d_surface);

    let mut surface_desc: D3D11_TEXTURE2D_DESC = unsafe { zeroed() };
    unsafe { d3d_surface.GetDesc(&mut surface_desc) };

    let mut view_desc = D3D11_RENDER_TARGET_VIEW_DESC::default();
    view_desc.Format = surface_desc.Format;
    view_desc.ViewDimension = D3D11_RTV_DIMENSION_TEXTURE2D;

    let d3d_render_target = unsafe {
        d3d_device_5
            .CreateRenderTargetView(&d3d_surface, &view_desc)
            .unwrap()
    };
    println!("d3d_render_target: {:?}", d3d_render_target);

    let mut fence_value = 0;

    let d3d_context_4 = d3d_device_context.cast::<ID3D11DeviceContext4>().unwrap();
    println!("d3d_context_4: {:?}", d3d_context_4);

    loop {
        // Clear Render Target View
        unsafe {
            let clear_color = [1f32, 1f32, 1f32, 1f32];
            d3d_device_context.ClearRenderTargetView(
                InParam::borrowed(Borrowed::new(Some(&d3d_render_target))),
                &clear_color as *const f32,
            );
        }
        println!("ClearRenderTargetView");

        // 配置在完成命令处理后发栅栏信号给 Fence
        fence_value = fence_value + 1;

        unsafe { d3d_context_4.Signal(&d3d_fence, fence_value).unwrap() };
        println!("Signal: {}", fence_value);

        // Present to display
        let display_task = display_task_pool.CreateTask().unwrap();
        println!("CreateTask: {:?}", display_task);

        display_task.SetScanout(&primary_scanout).unwrap();
        println!("SetScanout");

        display_task.SetWait(display_fence1, fence_value).unwrap();
        println!("SetWait: {}", fence_value);

        display_task_pool
            .TryExecuteTask(InParam::owned(display_task))
            .unwrap();

        // 等待 DisplayTarget 向 DisplaySource 发送的 V-Sync 同步信号
        display_device.WaitForVBlank(&display_source).unwrap();
    }
}

unsafe fn surface_flinger_main() -> HRESULT {
    let display_manager = DisplayManager::Create(DisplayManagerOptions::None).unwrap();

    let display_adapters = display_manager.GetCurrentAdapters().unwrap();

    println!(
        "Display Adapter Count: {}",
        display_adapters.Size().unwrap()
    );

    for adapter in display_adapters {
        println!(
            "{:?}: {}",
            adapter.Id().unwrap(),
            adapter.DeviceInterfacePath().unwrap()
        );
    }

    let display_targets = display_manager.GetCurrentTargets().unwrap();
    println!("Display Target Count: {}", display_targets.Size().unwrap());

    for target in display_targets {
        let target: DisplayTarget = target;
        let monitor_result = target.TryGetMonitor();

        if monitor_result.is_err() {
            continue;
        }

        let stable_monitor_id = target.StableMonitorId().unwrap();
        let device_interface_path = target.DeviceInterfacePath().unwrap();

        println!("StableMonitorId: {}", stable_monitor_id);
        println!("DeviceInterfacePath: {}", device_interface_path);

        let monitor = monitor_result.unwrap();

        let display_name = monitor.DisplayName().unwrap();
        println!("DisplayName: {}", display_name);

        if stable_monitor_id == "GBR01560_21_07E3_EF" {
            let targets: IIterable<DisplayTarget> = iterable::Iterable(vec![target.clone()]).into();

            let state_result = display_manager
                .TryAcquireTargetsAndCreateEmptyState(InParam::owned(targets.clone()))
                .unwrap();

            // let state_result = display_manager
            //     .TryAcquireTargetsAndReadCurrentState(InParam::owned(targets))
            //     .unwrap();

            let state_error_code = state_result.ErrorCode().unwrap();
            println!(
                "ErrorCode: {}",
                display_manager_result_to_string(state_error_code)
            );

            let state_extended_error_code = state_result.ExtendedErrorCode().unwrap();
            println!("ExtendedErrorCode: {}", state_extended_error_code.0);

            let display_state = state_result.State().unwrap();

            println!(
                "Current State Targets: {:?}",
                display_state.Targets().unwrap().Size().unwrap()
            );

            let display_path = display_state
                .ConnectTarget(InParam::owned(target.clone()))
                .unwrap();

            display_path.set_is_interlaced(false).unwrap();
            display_path.SetIsStereo(false).unwrap();
            display_path
                .SetScaling(DisplayPathScaling::Identity)
                .unwrap();

            // R8G8B8A8UIntNormalized
            display_path
                .SetSourcePixelFormat(DirectXPixelFormat::B8G8R8A8UIntNormalized)
                .unwrap();

            let modes = display_path
                .FindModes(DisplayModeQueryOptions::OnlyPreferredResolution)
                .unwrap();

            for (index, mode) in modes.into_iter().enumerate() {
                let source_resolution = mode.SourceResolution().unwrap();
                let presentation_rate = mode.PresentationRate().unwrap();

                let v_sync = presentation_rate.VerticalSyncRate;
                let v_sync_double = v_sync.Numerator / v_sync.Denominator;
                println!(
                    "Mode[{}]: {}x{} {}",
                    index, source_resolution.Width, source_resolution.Height, v_sync_double
                );

                if v_sync_double == 60 {
                    display_path
                        .ApplyPropertiesFromMode(InParam::owned(mode))
                        .unwrap();
                    break;
                }
            }

            // 应用以上代码中对 State 配置的 DisplayPath
            let apply_result = display_state
                .TryApply(DisplayStateApplyOptions::None)
                .unwrap();

            let state_extended_error_code = apply_result.ExtendedErrorCode().unwrap();
            println!(
                "ApplyResult ExtendedErrorCode: {:?}",
                state_extended_error_code
            );
            // check_hresult(applyResult.ExtendedErrorCode());

            // 重新读取应用 DisplayPath 后的 State 的状态
            let state_result = display_manager
                .TryAcquireTargetsAndReadCurrentState(InParam::owned(targets.clone()))
                .unwrap();
            let state_extended_error_code = state_result.ExtendedErrorCode().unwrap();
            println!(
                "AcquireResult ExtendedErrorCode: {:?}",
                state_extended_error_code
            );

            let display_state = state_result.State().unwrap();
            println!("display_state: {:?}", display_state);

            test_display_target(&display_manager, &target, &display_state);
        }
    }

    return S_OK;
}

fn display_manager_result_to_string(result: DisplayManagerResult) -> &'static str {
    match result {
        DisplayManagerResult::Success => "Success",
        DisplayManagerResult::TargetAccessDenied => "TargetAccessDenied",
        DisplayManagerResult::TargetStale => "TargetStale",
        DisplayManagerResult::RemoteSessionNotSupported => "RemoteSessionNotSupported",
        DisplayManagerResult::UnknownFailure | _ => "UnknownFailure",
    }
}

fn main2() {
    unsafe {
        // println!("unsafe_main: {}", unsafe_main());
        println!("surface_flinger_main: {:?}", surface_flinger_main());
    }
}
