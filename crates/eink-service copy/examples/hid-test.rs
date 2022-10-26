use std::ffi::OsString;
use std::io::{Error as IoError, Result as IoResult};
use std::mem::{size_of, zeroed, MaybeUninit};
use std::os::windows::prelude::OsStringExt;
use std::slice;

use anyhow::bail;
use libc::c_void;
use log::warn;
use ntapi::ntexapi::SYSTEM_HANDLE_INFORMATION;
use ntapi::ntexapi::{SystemHandleInformation, SYSTEM_HANDLE_TABLE_ENTRY_INFO};
use ntapi::ntioapi::{FileNameInformation, FILE_INFORMATION_CLASS, IO_STATUS_BLOCK};
use ntapi::ntzwapi::ZwQueryInformationFile;
use widestring::{U16CStr, U16CString};
use winapi::um::winnt::PVOID;
use windows::core::{GUID, PCWSTR};
use windows::Devices::Enumeration::{DeviceInformation, DevicePicker};
use windows::Devices::HumanInterfaceDevice::HidDevice;
use windows::Storage::FileAccessMode;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiChangeState, SetupDiEnumDeviceInfo, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceInterfaceDetailW, SetupDiRestartDevices,
    SetupDiSetClassInstallParamsW, DICS_DISABLE, DICS_ENABLE, DICS_FLAG_CONFIGGENERAL,
    DICS_FLAG_GLOBAL, DIF_PROPERTYCHANGE, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO,
    SP_CLASSINSTALL_HEADER, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
    SP_DEVINFO_DATA, SP_PROPCHANGE_PARAMS,
};
use windows::Win32::Devices::HumanInterfaceDevice::HidD_GetHidGuid;
use windows::Win32::Foundation::{
    GetLastError, BOOL, ERROR_INSUFFICIENT_BUFFER, HANDLE, HWND, INVALID_HANDLE_VALUE,
    STATUS_INFO_LENGTH_MISMATCH,
};
use windows::Win32::Storage::FileSystem::{GetFinalPathNameByHandleW, FILE_NAME_NORMALIZED};

fn main() -> anyhow::Result<()> {
    // let di = DeviceInformation::FindAllAsync()?.get().unwrap();
    // for (i, dev) in di.into_iter().enumerate() {
    //     let id = dev.Id().unwrap();
    //     if id.to_string().starts_with("\\\\?\\HID#VID_048D&PID_8957") {
    //         let kind = dev.Kind().unwrap();
    //         let name = dev.Name().unwrap();
    //         println!("{}: {:?} {:?}, {:?}", i, id, kind, name)
    //     }
    // }

    // HID\VID_048D&PID_8957&MI_02&COL01\7&25D10E82&0&0000

    // // let USAGE_PAGE_DIGITIZER = 0x0D;

    // HidDevice::FromIdAsync(deviceid, FileAccessMode::ReadWrite);

    const HID_USAGE_PAGE_DIGITIZER: u16 = 0x0D;
    const HID_USAGE_DIGITIZER_TOUCH_SCREEN: u16 = 0x04;

    // let selector =
    //     HidDevice::GetDeviceSelector(HID_USAGE_PAGE_DIGITIZER, HID_USAGE_DIGITIZER_TOUCH_SCREEN)?; //, 0x048D, 0x8957)?;

    // let di = DeviceInformation::CreateFromIdAsync(&selector)
    //     .unwrap()
    //     .get()
    //     .unwrap();
    // println!("{:?}", di);

    // let manager = hid::init().unwrap();
    // for (i, dev) in manager.devices().enumerate() {
    //     println!("{}: {:?}", i, dev.path());
    // }

    // unsafe {
    //     use ntapi::ntexapi::NtQuerySystemInformation;

    //     let mut buf_size = 1024 * 1024;
    //     let mut buf = libc::malloc(buf_size);

    //     let mut ret_size = 0;

    //     loop {
    //         let status = NtQuerySystemInformation(
    //             SystemHandleInformation,
    //             buf,
    //             buf_size as u32,
    //             &mut ret_size,
    //         );
    //         if status == STATUS_INFO_LENGTH_MISMATCH.0 {
    //             println!("STATUS_INFO_LENGTH_MISMATCH");
    //             buf_size = buf_size * 2;
    //             buf = libc::realloc(buf, buf_size);
    //         } else {
    //             println!("status: {}, ret_size: {}", status, ret_size);

    //             let info = buf as *mut SYSTEM_HANDLE_INFORMATION;
    //             let num_of_handles = (*info).NumberOfHandles;
    //             println!("num_of_handles: {}", num_of_handles);
    //             for i in 0..num_of_handles {
    //                 let hp = (*info).Handles.as_mut_ptr().add(i as usize);
    //                 let type_idx = (*hp).ObjectTypeIndex;
    //                 let h = (*hp).HandleValue;
    //                 let pid = (*hp).UniqueProcessId;
    //                 if pid == 1512 && type_idx == 28 {
    //                     let mut path: [u16; 256] = zeroed();
    //                     // GetFinalPathNameByHandleW(HANDLE(h as isize), &mut path, FILE_NAME_NORMALIZED);

    //                     let mut io_status_block: IO_STATUS_BLOCK = zeroed();

    //                     struct FileInformation {
    //                         length: u32,
    //                         file_name: [u16; 1024],
    //                     }

    //                     let mut file_info: FileInformation = zeroed();

    //                     ZwQueryInformationFile(
    //                         h as *mut c_void,
    //                         &mut io_status_block,
    //                         &mut file_info as *const _ as PVOID,
    //                         1024,
    //                         FileNameInformation,
    //                     );

    //                     println!("PID: {}", file_info.length);
    //                 }
    //             }
    //             break;
    //         }
    //     }

    //     libc::free(buf);
    // }

    // let mut hid_guid: GUID = Default::default();
    // unsafe { HidD_GetHidGuid(&mut hid_guid) };
    // println!("HID GUID: {:?}", &hid_guid);

    let hidapi = hidapi::HidApi::new()?;

    // let dev_path = "\\\\?\\HID#VID_048D&PID_8957&MI_02&Col01#7&25d10e82&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";

    // let dis = DeviceInfoSet::new(hid_guid)?;
    // for (i, path) in dis.devices().into_iter().enumerate() {
    //     println!("-- {}", path);
    //     let mut dev_info = dis.info(i as u32);
    //     let details = dev_info.details();
    //     if path.eq_ignore_ascii_case(dev_path) {
    //         println!("XX {}, {}", path, details.DevInst);
    //         disable_device(&dis, &mut dev_info);
    //         enable_device(&dis, &mut dev_info);
    //         break;
    //     }
    // }

    for (i, dev_info) in hidapi.device_list().enumerate() {
        // "\\\\?\\HID#VID_048D&PID_8957&MI_02&Col01#7&25d10e82&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"

        if dev_info.usage_page() == HID_USAGE_PAGE_DIGITIZER
            && dev_info.usage() == HID_USAGE_DIGITIZER_TOUCH_SCREEN
        {
            let dev_path = dev_info.path().to_str()?;
            println!(
                "{}: {},{} {}",
                i,
                dev_info.usage_page(),
                dev_info.usage(),
                dev_path
            );

            let dev = dev_info.open_device(&hidapi)?;
            println!(" - {:?}", dev.get_product_string());

            let mut buf: [u8; 1024] = unsafe { std::mem::zeroed() };

            loop {
                let ret = dev.read(&mut buf);
                if ret.is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                } else {
                    println!("{:?}: {:?}", ret, buf);
                    break;
                }
            }
        }
    }

    Ok(())
}

// /// <summary>
// /// Enable or disable a device.
// /// </summary>
// /// <param name="classGuid">The class guid of the device. Available in the device manager.</param>
// /// <param name="instanceId">The device instance id of the device. Available in the device manager.</param>
// /// <param name="enable">True to enable, False to disable.</param>
// /// <remarks>Will throw an exception if the device is not Disableable.</remarks>
// fn set_device_enabled(class_guid: GUID, instance_id: &str, enable: bool) -> Result<()> {
//     unsafe {
//         // SafeDeviceInfoSetHandle diSetHandle = null;
//         //         try
//         //         {

//         // Get the handle to a device information set for all devices matching classGuid that are present on the
//         // system.
//         let di = SetupDiGetClassDevsW(&mut class_guid, PCWSTR::null(), HWND::default(), 2)?;
//         let di_dats = get_device_info_data(di)?;

//         // Find the index of our instance. i.e. the touchpad mouse - I have 3 mice attached...
//         let i = get_index_of_instance(di, &di_dats, instance_id)?;

//         // Disable...
//         enable_device(di, di_dats[i], enable);

//         Ok(())
//     }
// }

// fn get_device_info_data(dh: HDEVINFO) -> anyhow::Result<Vec<SP_DEVINFO_DATA>> {
//     unsafe {
//         let dats = Vec::new::<SP_DEVINFO_DATA>();
//         let mut dat: SP_DEVINFO_DATA = zeroed();
//         dat.cbSize = size_of::<SP_DEVINFO_DATA>();
//         let mut i = 0;
//         while SetupDiEnumDeviceInfo(dh, i, &mut dat).as_bool() {
//             dats.push(dat.clone());
//             i = i + 1;
//         }
//         return Ok(dats);
//     }
// }

// // Find the index of the particular DeviceInfoData for the instanceId.
// fn get_index_of_instance(
//     dh: HDEVINFO,
//     dats: &Vec<SP_DEVINFO_DATA>,
//     instance_id: &str,
// ) -> Result<usize, anyhow::Error> {
//     unsafe {
//         let instance_id = U16CString::from_str(instance_id)?;
//         for (i, dat) in dats.into_iter().enumerate() {
//             let mut required_size: u32 = 0;
//             let res = SetupDiGetDeviceInstanceIdW(dh, dat, None, &mut required_size);
//             if !res.as_bool() && GetLastError() == ERROR_INSUFFICIENT_BUFFER {
//                 let buf = Vec::with_capacity::<u16>(required_size);
//                 let id = widestring::U16CString::from_vec(buf)?;
//                 let res =
//                     SetupDiGetDeviceInstanceIdW(dh, dat, Some(id.as_mut_ptr()), &mut required_size);
//                 if !res.as_bool() {
//                     bail!("Cannot SetupDiGetDeviceInstanceIdW: {:?}", GetLastError());
//                 }
//                 if id == instance_id {
//                     return Ok(i);
//                 }
//             }
//         }
//     }
// }

// fn enable_device(dh: HDEVINFO, di_data: DeviceInfoDataW, enable: bool)
// {
//     // PropertyChangeParameters @params = new PropertyChangeParameters();
//     //         // The size is just the size of the header, but we've flattened the structure.
//     //         // The header comprises the first two fields, both integer.
//     //         @params.Size = 8;
//     //         @params.DiFunction = DiFunction.PropertyChange;
//     //         @params.Scope = Scopes.Global;
//     //         if (enable)
//     //         {
//     //             @params.StateChange = StateChangeAction.Enable;
//     //         }
//     //         else
//     //         {
//     //             @params.StateChange = StateChangeAction.Disable;
//     //         }

//     unsafe {
//         let dev_dat: SP_DEVINFO_DATA = zeroed();
//         let params: SP_CLASSINSTALL_HEADER = zeroed();
//         params.cbSize = 8;
//         params.InstallFunction =
//         let res = SetupDiSetClassInstallParamsW(dh, &mut dev_dat, ref @params, Marshal.SizeOf(@params));
//     }

//     // if (result == false) throw new Win32Exception();
//     //         result = NativeMethods.SetupDiCallClassInstaller(DiFunction.PropertyChange, handle, ref diData);
//     //         if (result == false)
//     //         {
//     //             int err = Marshal.GetLastWin32Error();
//     //             if (err == (int)SetupApiError.NotDisableable)
//     //                 throw new ArgumentException("Device can't be disabled (programmatically or in Device Manager).");
//     //             else if (err >= (int)SetupApiError.NoAssociatedClass && err <= (int)SetupApiError.OnlyValidateViaAuthenticode)
//     //                 throw new Win32Exception("SetupAPI error: " + ((SetupApiError)err).ToString());
//     //             else
//     //                 throw new Win32Exception();
//     //         }
// }

pub struct DeviceInfoSet {
    set: HDEVINFO,
    guid: GUID,
}

impl DeviceInfoSet {
    pub fn new(guid: GUID) -> IoResult<Self> {
        // Gets device set for specified HID class
        let set = unsafe {
            SetupDiGetClassDevsW(
                Some(&guid),
                PCWSTR::null(),
                HWND::default(),
                DIGCF_DEVICEINTERFACE | DIGCF_PRESENT,
            )?
        };

        if set.is_invalid() {
            panic!("SetupDiGetClassDevsW Failed");
        }

        Ok(Self { set, guid })
    }

    pub fn get_set(&self) -> HDEVINFO {
        self.set
    }

    fn get_guid(&self) -> GUID {
        self.guid
    }

    pub fn devices(&self) -> DeviceInfoSetIter {
        DeviceInfoSetIter::new(self)
    }

    pub fn info(&self, index: u32) -> DeviceInterfaceInfo {
        DeviceInterfaceInfo::new(&self.set, index)
    }
}

struct DeviceInterfaceDetailData {
    data: *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W,
    path_len: usize,
}

macro_rules! offset_of {
    ($ty:ty, $field:ident) => {
        unsafe { &(*(0 as *const $ty)).$field as *const _ as usize }
    };
}

fn from_wide_ptr(ptr: *const u16, len: usize) -> String {
    assert!(!ptr.is_null() && len % 2 == 0);
    let slice = unsafe { slice::from_raw_parts(ptr, len / 2) };
    OsString::from_wide(slice).to_string_lossy().into_owned()
}

impl DeviceInterfaceDetailData {
    fn new(size: usize) -> Option<Self> {
        let mut cb_size = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();

        if cfg!(target_pointer_width = "32") {
            cb_size = 4 + 2; // default TCHAR size
        }

        if size < cb_size {
            println!("DeviceInterfaceDetailData is too small. Size: {}", size);
            return None;
        }

        let data = unsafe { libc::malloc(size) as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W };

        if data.is_null() {
            return None;
        }

        unsafe { (*data).cbSize = cb_size as u32 };

        let offset = offset_of!(SP_DEVICE_INTERFACE_DETAIL_DATA_W, DevicePath);

        Some(Self {
            data,
            path_len: size - offset,
        })
    }

    fn get(&self) -> *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W {
        self.data
    }

    fn path(&self) -> String {
        unsafe { from_wide_ptr((*self.data).DevicePath.as_ptr(), self.path_len - 2) }
    }
}

pub struct DeviceInfoSetIter<'a> {
    set: &'a DeviceInfoSet,
    index: u32,
}

impl<'a> DeviceInfoSetIter<'a> {
    fn new(set: &'a DeviceInfoSet) -> Self {
        Self { set, index: 0 }
    }
}

impl<'a> Iterator for DeviceInfoSetIter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut device_interface_data =
            unsafe { MaybeUninit::<SP_DEVICE_INTERFACE_DATA>::uninit().assume_init() };

        device_interface_data.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;

        // Gets device interface from device set
        let rv = unsafe {
            SetupDiEnumDeviceInterfaces(
                self.set.get_set(),
                None,
                &self.set.get_guid(),
                self.index,
                &mut device_interface_data,
            )
        };

        // Past the last device index
        if !rv.as_bool() {
            warn!("The last device index has been passed");
            return None;
        }

        // get size for details from a device interface
        let mut required_size = 0;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                self.set.get_set(),
                &mut device_interface_data,
                None,
                required_size,
                Some(&mut required_size),
                None,
            )
        };

        if required_size == 0 {
            return None;
        }

        let detail = DeviceInterfaceDetailData::new(required_size as usize);

        // malloc() fails
        if detail.is_none() {
            return None;
        }

        let mut detail = detail.unwrap();
        let rv = unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                self.set.get_set(),
                &mut device_interface_data,
                Some(detail.get()),
                required_size,
                None,
                None,
            )
        };

        if !rv.as_bool() {
            return None;
        }

        self.index += 1;
        Some(detail.path())
    }
}

pub struct DeviceInterfaceInfo<'a> {
    set: &'a HDEVINFO,
    index: u32,
}

impl<'a> DeviceInterfaceInfo<'a> {
    fn new(set: &'a HDEVINFO, index: u32) -> Self {
        Self { set, index }
    }
}

impl<'a> DeviceInterfaceInfo<'a> {
    fn details(&mut self) -> SP_DEVINFO_DATA {
        let mut device_interface_data: SP_DEVINFO_DATA =
            unsafe { MaybeUninit::uninit().assume_init() };

        device_interface_data.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;

        let res =
            unsafe { SetupDiEnumDeviceInfo(*self.set, self.index, &mut device_interface_data) };

        if res.as_bool() {
            return device_interface_data;
        } else {
            panic!("SetupDiEnumDeviceInfo failed with Error {:?}", unsafe {
                GetLastError()
            })
        }
    }
}

pub fn disable_device(
    device_info_set: &DeviceInfoSet,
    device_inter_info: &mut DeviceInterfaceInfo,
) {
    let remove = SP_CLASSINSTALL_HEADER {
        cbSize: std::mem::size_of::<SP_CLASSINSTALL_HEADER>() as u32,
        InstallFunction: DIF_PROPERTYCHANGE,
    };

    let mut remove_params = SP_PROPCHANGE_PARAMS {
        ClassInstallHeader: remove,
        StateChange: DICS_DISABLE,
        Scope: DICS_FLAG_GLOBAL,
        HwProfile: DICS_FLAG_CONFIGGENERAL,
    };

    let install_param_res = unsafe {
        SetupDiSetClassInstallParamsW(
            device_info_set.set,
            Some(&mut (*device_inter_info).details() as *const SP_DEVINFO_DATA),
            Some(&mut remove_params.ClassInstallHeader as *const _),
            std::mem::size_of::<SP_PROPCHANGE_PARAMS>() as u32,
        )
    };

    if !install_param_res.as_bool() {
        panic!(
            "SetupDiEnumDeviceInfo failed with Error {:?} {:?}",
            unsafe { GetLastError() },
            install_param_res
        )
    }

    let change_state_res = unsafe {
        SetupDiChangeState(
            device_info_set.set,
            &mut (*device_inter_info).details() as *mut SP_DEVINFO_DATA,
        )
    };

    if !change_state_res.as_bool() {
        panic!(
            "SetupDiChangeState failed with Error {:?} {:?}",
            unsafe { GetLastError() },
            change_state_res
        )
    }

    println!(
        "Unknown device detected! disabling the below device: {:?}",
        (*device_inter_info).details().ClassGuid
    );
}

pub fn enable_device(device_info_set: &DeviceInfoSet, device_inter_info: &mut DeviceInterfaceInfo) {
    let remove = SP_CLASSINSTALL_HEADER {
        cbSize: std::mem::size_of::<SP_CLASSINSTALL_HEADER>() as u32,
        InstallFunction: DIF_PROPERTYCHANGE,
    };

    let mut remove_params = SP_PROPCHANGE_PARAMS {
        ClassInstallHeader: remove,
        StateChange: DICS_ENABLE,
        Scope: DICS_FLAG_GLOBAL,
        HwProfile: DICS_FLAG_CONFIGGENERAL,
    };

    let install_param_res = unsafe {
        SetupDiSetClassInstallParamsW(
            device_info_set.set,
            Some(&mut (*device_inter_info).details() as *const SP_DEVINFO_DATA),
            Some(&mut remove_params.ClassInstallHeader as *const _),
            std::mem::size_of::<SP_PROPCHANGE_PARAMS>() as u32,
        )
    };

    if !install_param_res.as_bool() {
        panic!(
            "SetupDiEnumDeviceInfo failed with Error {:?} {:?}",
            unsafe { GetLastError() },
            install_param_res
        )
    }

    let change_state_res = unsafe {
        SetupDiChangeState(
            device_info_set.set,
            &mut (*device_inter_info).details() as *mut SP_DEVINFO_DATA,
        )
    };

    if !change_state_res.as_bool() {
        panic!(
            "SetupDiChangeState failed with Error {:?} {:?}",
            unsafe { GetLastError() },
            change_state_res
        )
    }

    println!(
        "Unknown device detected! disabling the below device: {:?}",
        (*device_inter_info).details().ClassGuid
    );
}
