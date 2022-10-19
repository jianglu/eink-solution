use std::{
    collections::HashSet,
    ffi::{c_void, CStr},
    mem::zeroed,
    path::PathBuf,
};

use anyhow::Result;
use cmd_lib::{run_cmd, spawn};

use log::info;
use widestring::{U16CStr, U16CString};
use windows::{
    core::{PCWSTR, PWSTR},
    w,
    Graphics::Capture::{GraphicsCaptureItem, IGraphicsCaptureItem},
    Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        Storage::FileSystem::STANDARD_RIGHTS_REQUIRED,
        System::{
            Com,
            StationsAndDesktops::{
                CloseDesktop, CreateDesktopW, EnumDesktopWindows, EnumDesktopsW, GetThreadDesktop,
                OpenDesktopW, SwitchDesktop, DF_ALLOWOTHERACCOUNTHOOK,
            },
            SystemServices::{
                DESKTOP_CREATEMENU, DESKTOP_CREATEWINDOW, DESKTOP_ENUMERATE, DESKTOP_HOOKCONTROL,
                DESKTOP_JOURNALPLAYBACK, DESKTOP_JOURNALRECORD, DESKTOP_READOBJECTS,
                DESKTOP_SWITCHDESKTOP, DESKTOP_WRITEOBJECTS,
            },
            Threading::{
                CreateProcessW, GetCurrentThreadId, CREATE_NEW_CONSOLE, NORMAL_PRIORITY_CLASS,
                PROCESS_INFORMATION, STARTUPINFOW,
            },
            WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
        },
        UI::WindowsAndMessaging::{
            CloseWindow, EnumWindows, GetAncestor, GetClassNameA, GetDesktopWindow, GetWindowLongA,
            GetWindowLongW, GetWindowTextA, GetWindowTextW, GetWindowThreadProcessId,
            PostQuitMessage, PostThreadMessageA, RealGetWindowClassA, GA_ROOT, GET_ANCESTOR_FLAGS,
            GWL_STYLE, HCF_DEFAULTDESKTOP, WM_QUIT, WNDENUMPROC, WS_VISIBLE,
        },
    },
};

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
        let mut utf16 = vec![0x0u16; 1024];
        GetWindowTextW(hwnd, &mut utf16);
        let title = U16CString::from_vec_unchecked(utf16);
        Ok(title.to_string_lossy())
    }
}

fn find_all_windows() -> HashSet<isize> {
    unsafe extern "system" fn enum_hwnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let mut hwnds = Box::from_raw(lparam.0 as *mut HashSet<isize>);

        let hwnd_ancestor = GetAncestor(hwnd, GA_ROOT);
        if hwnd_ancestor == hwnd {
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let visible = (style & WS_VISIBLE.0) == WS_VISIBLE.0;

            if visible {
                hwnds.insert(hwnd.0);
            }
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

#[windows_dll::dll(VirtualDesktopAccessor)]
extern "system" {

    #[allow(non_snake_case)]
    pub fn RestartVirtualDesktopAccessor() -> ();

    #[allow(non_snake_case)]
    pub fn GetDesktopCount() -> i32;
}

use windows::{core::*, Win32::Foundation::*, Win32::System::Com::*};

// #[interface("B2F925B9-5A0F-4D2E-9F4D-2B1507593C10")]
// unsafe trait IVirtualDesktopManagerInternal: windows::core::IUnknown {
//     // #[allow(non_snake_case)]
//     fn GetCount(&self, pCount: *mut u32) -> HRESULT;
// }

// pub type IUIAnimationManager = *mut ::core::ffi::c_void;

tinycom::iid!(
    IID_VDMI = 0xB2F925B9,
    0x5A0F,
    0x4D2E,
    0x9F,
    0x4D,
    0x2B,
    0x15,
    0x07,
    0x59,
    0x3C,
    0x10
);

tinycom::com_interface! {
    interface IVirtualDesktopManagerInternal: tinycom::IUnknown {
        iid: IID_VDMI,
        vtable: IVirtualDesktopManagerInternalVtbl,

        fn GetDesktopIsPerMonitor() -> i32;

        fn GetCount(hWndOrMon: *mut c_void) -> i32;

        fn GetDesktops(hWndOrMon: *mut c_void, ppDesktops: *mut *mut c_void) -> tinycom::HResult;
    }
}

// com::interfaces! {
//     #[uuid("00000000-0000-0000-C000-000000000046")]
//     pub unsafe interface IUnknown {
//         fn QueryInterface(
//             &self,
//             riid: *const com::IID,
//             ppv: *mut *mut c_void
//         ) -> HRESULT;
//         fn AddRef(&self) -> u32;
//         fn Release(&self) -> u32;
//     }
//     #[uuid("B2F925B9-5A0F-4D2E-9F4D-2B1507593C10")]
//     pub unsafe interface IVirtualDesktopManagerInternal: IUnknown {
//     }
// }

// C5E0CDCA-7B6E-41B2-9FC4-D93975CC467B
// C5E0CDCA-7B6E-41B2-9FC4-D93975CC467B
// 0xB2F925B9-0x5A0F-0x4D2E-0x9F 0x4D 0x2B 0x15 0x07 0x59 0x3C 0x10

// pub const CLSID_VIRTUAL_DESKTOP_MANAGER_INTERNAL_CLASS: com::IID = com::IID {
//     data1: 0xb2f925b9,
//     data2: 0x5a0f,
//     data3: 0x4d2e,
//     data4: [0x9f, 0x4d, 0x2b, 0x15, 0x07, 0x59, 0x3c, 0x10],
// };

// pub const CLSID_VIRTUAL_DESKTOP_MANAGER_INTERNAL_CLASS: com::IID = com::IID {
//     data1: 0xb2f925b9,
//     data2: 0x5a0f,
//     data3: 0x4d2e,
//     data4: [0x9f, 0x4d, 0x2b, 0x15, 0x07, 0x59, 0x3c, 0x10],
// };

// C2F03A33-21F5-47FA-B4BB-156362A2F239

fn main() -> Result<()> {
    eink_logger::init_with_env()?;

    // let virtual_desktop = include_str!("virtual_desktop.ps1");

    // unsafe {
    //     CoInitializeEx(std::ptr::null_mut() as _, COINIT::default())?;
    //     let service: ITaskService = CoCreateInstance(&CLSID_CTaskScheduler, None, CLSCTX_INPROC_SERVER)?;
    // }

    unsafe { Com::CoInitializeEx(None, Com::COINIT_APARTMENTTHREADED) }.unwrap();

    let clsid = unsafe {
        Com::CLSIDFromString(PCWSTR::from(&HSTRING::from(
            "{C2F03A33-21F5-47FA-B4BB-156362A2F239}",
        )))?
    };

    let provider: Com::IServiceProvider =
        unsafe { Com::CoCreateInstance(&clsid, None, Com::CLSCTX_ALL)? };
    println!("provider: {:?}", provider);

    let clsid = unsafe {
        Com::CLSIDFromString(PCWSTR::from(&HSTRING::from(
            "{C5E0CDCA-7B6E-41B2-9FC4-D93975CC467B}",
        )))?
    };

    let riid = unsafe {
        Com::CLSIDFromString(PCWSTR::from(&HSTRING::from(
            "{B2F925B9-5A0F-4D2E-9F4D-2B1507593C10}",
        )))?
    };

    unsafe {
        let mut unknown: tinycom::ComPtr<IVirtualDesktopManagerInternal> = tinycom::ComPtr::new();
        println!("unknown: {:?}", unknown);
        // IVirtualDesktopManagerInternal::from_raw(abi)
        provider.QueryService(&clsid, &riid, unknown.as_mut_ptr())?;
        println!("unknown: {:?}", unknown);

        let other: tinycom::ComPtr<IVirtualDesktopManagerInternal> =
            tinycom::ComPtr::from(&unknown);
        println!("other: {:?}", other);

        // 800706F4
        // A null reference pointer was passed to the stub
        let desk_per_mon = other.GetDesktopIsPerMonitor();
        println!("desk_per_mon: {:?}", desk_per_mon);

        //     // unknown.query_interface(&riid, object);
        //     // let vdmi = IVirtualDesktopManagerInternal::from_raw(pvobject);
        //     // println!("vdmi: {:?}", vdmi);
        //     let count = other.GetCount(0);
        //     println!("count: {:?}", count);

        //     let mut desktops: tinycom::ComPtr<tinycom::IUnknown> = tinycom::ComPtr::new();
        //     let res = other.GetDesktops(0, desktops.as_mut_ptr());
        //     println!("{:?} desktops: {:?}", res, desktops);
    }

    // // Initialize the COM apartment
    // com::runtime::init_apartment(com::runtime::ApartmentType::Multithreaded)
    //     .unwrap_or_else(|hr| panic!("Failed to initialize COM Library{:x}", hr));
    // println!("Initialized apartment");

    // // Initialises the COM library
    // com::runtime::init_runtime().expect("Failed to initialize COM Library");

    // // // Get a `BritishShortHairCat` class factory
    // // let factory = com::runtime::get_class_object::<com::interfaces::iclass_factory::IClassFactory>(
    // //     &CLSID_VIRTUAL_DESKTOP_MANAGER_INTERNAL_CLASS,
    // // )
    // // .unwrap_or_else(|hr| panic!("Failed to get cat class object 0x{:x}", hr));
    // // println!("Got cat class object");

    // // // Get an instance of a `BritishShortHairCat` as the `IUnknown` interface
    // // let unknown = factory
    // //     .create_instance::<IVirtualDesktopManagerInternal>()
    // //     .expect("Failed to get IUnknown");
    // // println!("Got IUnknown");

    // // let device_enumerator: IMMDeviceEnumerator =
    // // CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_)?;

    // let mut vdm = com::runtime::create_instance::<IVirtualDesktopManagerInternal>(
    //     &CLSID_VIRTUAL_DESKTOP_MANAGER_INTERNAL_CLASS,
    // )
    // .expect("Failed to get a cat");

    // println!("GetDesktopCount: {}", unsafe { GetDesktopCount() });
    // std::thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}
