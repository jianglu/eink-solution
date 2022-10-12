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

pub use windows::core::{
    Borrowed, IInspectable, IUnknown, InParam, Interface, HSTRING, PCSTR, PCWSTR,
};
pub use windows::s;
pub use windows::w;
pub use windows::Devices::Display::Core::{
    DisplayAdapter, DisplayDevice, DisplayFence, DisplayManager, DisplayManagerOptions,
    DisplayManagerResult, DisplayModeInfo, DisplayModeQueryOptions, DisplayPathScaling,
    DisplayPrimaryDescription, DisplayScanout, DisplayState, DisplayStateApplyOptions,
    DisplaySurface, DisplayTarget,
};
pub use windows::Foundation::{Collections::IIterable, IReference, PropertyValue};
pub use windows::Graphics::DirectX::{
    Direct3D11::Direct3DMultisampleDescription, DirectXColorSpace, DirectXPixelFormat,
};
pub use windows::Win32::Devices::Display::{
    DisplayConfigSetDeviceInfo, DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_TYPE,
};
pub use windows::Win32::Foundation::{
    CloseHandle, E_FAIL, E_UNEXPECTED, HANDLE, HINSTANCE, HWND, INVALID_HANDLE_VALUE, LPARAM,
    LRESULT, LUID, RECT, S_OK, WPARAM,
};
pub use windows::Win32::Graphics::Direct3D::Dxc::{
    DxcCreateInstance, IDxcCompiler2, IDxcCompiler3,
};
pub use windows::Win32::Graphics::Direct3D::Fxc::{D3DCompile, D3DCOMPILE_PREFER_FLOW_CONTROL};
pub use windows::Win32::Graphics::Direct3D::{
    ID3DBlob, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D11_SRV_DIMENSION_TEXTURE2D,
    D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL, D3D_PRIMITIVE_TOPOLOGY,
    D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D_SHADER_MACRO, D3D_SRV_DIMENSION_TEXTURE2D,
};
pub use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Buffer, ID3D11ClassLinkage, ID3D11Device, ID3D11Device5,
    ID3D11DeviceContext, ID3D11DeviceContext4, ID3D11Fence, ID3D11InputLayout, ID3D11PixelShader,
    ID3D11RenderTargetView, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D,
    ID3D11VertexShader, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_BIND_FLAG, D3D11_BIND_INDEX_BUFFER,
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BIND_VERTEX_BUFFER,
    D3D11_BUFFER_DESC, D3D11_COMPARISON_NEVER, D3D11_CPU_ACCESS_FLAG, D3D11_CREATE_DEVICE_FLAG,
    D3D11_FENCE_FLAG_SHARED, D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_FLOAT32_MAX,
    D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_RENDER_TARGET_VIEW_DESC,
    D3D11_RESOURCE_MISC_FLAG, D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX, D3D11_RTV_DIMENSION_TEXTURE2D,
    D3D11_SAMPLER_DESC, D3D11_SDK_VERSION, D3D11_SHADER_RESOURCE_VIEW_DESC,
    D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV,
    D3D11_TEXTURE2D_DESC, D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT,
};
pub use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R32G32_FLOAT,
    DXGI_FORMAT_R32_UINT, DXGI_FORMAT_R8G8B8A8_UNORM,
};
pub use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, IDXGIAdapter, IDXGIAdapter4, IDXGIDevice, IDXGIFactory6, IDXGIKeyedMutex,
    DXGI_CREATE_FACTORY_DEBUG,
};
pub use windows::Win32::Graphics::Gdi::{UpdateWindow, HBRUSH};
pub use windows::Win32::System::LibraryLoader::GetModuleHandleW;
pub use windows::Win32::System::Threading::{CreateEventW, SetProcessShutdownParameters};
pub use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};
pub use windows::Win32::System::WinRT::Display::IDisplayDeviceInterop;
pub use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRect, CreateWindowExW, DestroyCursor, DispatchMessageW, LoadCursorW, PeekMessageW,
    RegisterClassExW, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, HCURSOR, HICON,
    IDC_ARROW, MSG, PM_REMOVE, WINDOW_EX_STYLE, WM_QUERYENDSESSION, WM_QUIT, WM_USER, WNDCLASSEXW,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPEDWINDOW, WS_POPUP,
};
pub use windows::Win32::{Foundation::DuplicateHandle, Graphics::Dxgi::IDXGIResource};
pub use windows::{core::HRESULT, Devices::Display::Core::DisplayPath};
