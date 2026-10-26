//! IOKit HID transport for the Razer vendor protocol.
//!
//! The keyboard speaks a 90-byte vendor feature report addressed to report ID 0.
//! macOS gates keyboard HID access behind Input Monitoring (TCC), so the first
//! open requests access explicitly instead of failing with a bare IOReturn.

use std::ffi::c_void;
use std::os::raw::c_char;
use std::ptr;
use std::thread::sleep;
use std::time::Duration;

pub const REPORT_LEN: usize = 90;

const REPORT_TYPE_FEATURE: i32 = 2;
const REQUEST_TYPE_LISTEN_EVENT: i32 = 1;
const ACCESS_GRANTED: i32 = 0;
const CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

#[link(name = "IOKit", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn IOHIDManagerCreate(allocator: *const c_void, options: u32) -> *mut c_void;
    fn IOHIDManagerSetDeviceMatching(manager: *mut c_void, matching: *const c_void);
    fn IOHIDManagerScheduleWithRunLoop(
        manager: *mut c_void,
        run_loop: *mut c_void,
        mode: *const c_void,
    );
    fn IOHIDManagerOpen(manager: *mut c_void, options: u32) -> i32;
    fn IOHIDManagerCopyDevices(manager: *mut c_void) -> *const c_void;

    fn IOHIDDeviceOpen(device: *mut c_void, options: u32) -> i32;
    fn IOHIDDeviceSetReport(
        device: *mut c_void,
        report_type: i32,
        report_id: isize,
        report: *const u8,
        length: isize,
    ) -> i32;
    fn IOHIDDeviceGetReport(
        device: *mut c_void,
        report_type: i32,
        report_id: isize,
        report: *mut u8,
        length: *mut isize,
    ) -> i32;
    fn IOHIDCheckAccess(request_type: i32) -> i32;
    fn IOHIDRequestAccess(request_type: i32) -> bool;

    fn CFStringCreateWithCString(
        allocator: *const c_void,
        string: *const c_char,
        encoding: u32,
    ) -> *const c_void;
    fn CFNumberCreate(
        allocator: *const c_void,
        number_type: isize,
        value: *const c_void,
    ) -> *const c_void;
    fn CFDictionaryCreate(
        allocator: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        count: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> *const c_void;
    fn CFSetGetCount(set: *const c_void) -> isize;
    fn CFSetGetValues(set: *const c_void, values: *mut *const c_void);
    fn CFRetain(cf: *const c_void) -> *const c_void;
    fn CFRelease(cf: *const c_void);

    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source_handled: u8) -> i32;

    static kCFRunLoopDefaultMode: *const c_void;
    static kCFTypeDictionaryKeyCallBacks: [u8; 48];
    static kCFTypeDictionaryValueCallBacks: [u8; 40];
}

/// An open handle to the keyboard's vendor HID interface.
pub struct Keyboard {
    device: *mut c_void,
    manager: *mut c_void,
}

// The handle is only ever used behind a mutex.
unsafe impl Send for Keyboard {}

impl Keyboard {
    pub fn open(pid: u16) -> Result<Self, String> {
        unsafe {
            Self::ensure_input_monitoring()?;

            let manager = IOHIDManagerCreate(ptr::null(), 0);
            if manager.is_null() {
                return Err("could not create the HID manager".into());
            }

            let vid_key = CFStringCreateWithCString(
                ptr::null(),
                c"VendorID".as_ptr(),
                CF_STRING_ENCODING_UTF8,
            );
            let pid_key = CFStringCreateWithCString(
                ptr::null(),
                c"ProductID".as_ptr(),
                CF_STRING_ENCODING_UTF8,
            );

            let vendor_id: i32 = 0x1532;
            let product_id: i32 = pid as i32;
            let vid_value = CFNumberCreate(ptr::null(), 9, &vendor_id as *const i32 as *const c_void);
            let pid_value = CFNumberCreate(ptr::null(), 9, &product_id as *const i32 as *const c_void);

            let keys = [vid_key, pid_key];
            let values = [vid_value, pid_value];
            let matching = CFDictionaryCreate(
                ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                2,
                &kCFTypeDictionaryKeyCallBacks as *const _ as *const c_void,
                &kCFTypeDictionaryValueCallBacks as *const _ as *const c_void,
            );

            IOHIDManagerSetDeviceMatching(manager, matching);
            IOHIDManagerScheduleWithRunLoop(
                manager,
                CFRunLoopGetCurrent(),
                kCFRunLoopDefaultMode,
            );

            for handle in [vid_key, pid_key, vid_value, pid_value, matching] {
                CFRelease(handle);
            }

            if IOHIDManagerOpen(manager, 0) != 0 {
                CFRelease(manager);
                return Err("could not open the HID manager (is the keyboard plugged in?)".into());
            }

            // Give the manager a beat to enumerate.
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.25, 0);

            let set = IOHIDManagerCopyDevices(manager);
            if set.is_null() || CFSetGetCount(set) == 0 {
                CFRelease(set);
                CFRelease(manager);
                return Err(format!("no device 1532:{pid:04x} found"));
            }

            let count = CFSetGetCount(set) as usize;
            let mut devices: Vec<*const c_void> = vec![ptr::null(); count];
            CFSetGetValues(set, devices.as_mut_ptr());

            let device = CFRetain(devices[0]) as *mut c_void;
            CFRelease(set);

            let rc = IOHIDDeviceOpen(device, 0);
            if rc != 0 {
                CFRelease(device);
                CFRelease(manager);
                return Err(format!("could not open the device (0x{rc:x})"));
            }

            Ok(Keyboard { device, manager })
        }
    }

    fn ensure_input_monitoring() -> Result<(), String> {
        unsafe {
            if IOHIDCheckAccess(REQUEST_TYPE_LISTEN_EVENT) == ACCESS_GRANTED {
                return Ok(());
            }
            IOHIDRequestAccess(REQUEST_TYPE_LISTEN_EVENT);
            sleep(Duration::from_millis(300));
            if IOHIDCheckAccess(REQUEST_TYPE_LISTEN_EVENT) == ACCESS_GRANTED {
                Ok(())
            } else {
                Err("macOS Input Monitoring permission is required. Enable it for this \
                     process in System Settings > Privacy & Security > Input Monitoring."
                    .into())
            }
        }
    }

    /// Send one 90-byte report and read the device's response.
    pub fn exchange(&self, report: &[u8; REPORT_LEN]) -> Result<[u8; REPORT_LEN], String> {
        unsafe {
            let write = IOHIDDeviceSetReport(
                self.device,
                REPORT_TYPE_FEATURE,
                0,
                report.as_ptr(),
                REPORT_LEN as isize,
            );
            if write != 0 {
                return Err(format!("SET_REPORT failed (0x{write:x})"));
            }

            // The board needs a moment before the response is readable.
            sleep(Duration::from_micros(1_500));

            let mut response = [0u8; REPORT_LEN];
            let mut length = REPORT_LEN as isize;
            let read = IOHIDDeviceGetReport(
                self.device,
                REPORT_TYPE_FEATURE,
                0,
                response.as_mut_ptr(),
                &mut length,
            );
            if read != 0 {
                return Err(format!("GET_REPORT failed (0x{read:x})"));
            }
            if length < 9 {
                return Err("device returned a short report".into());
            }
            Ok(response)
        }
    }
}

impl Drop for Keyboard {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.device);
            CFRelease(self.manager);
        }
    }
}
