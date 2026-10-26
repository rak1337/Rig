//! Which application is in front, via NSWorkspace.
//!
//! Used to switch profiles automatically. No subprocess, no Automation permission —
//! this is a plain property read on the shared workspace.

use std::ffi::{c_char, c_void, CStr};

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

unsafe fn send0(object: *mut c_void, selector: *mut c_void) -> *mut c_void {
    let send: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    send(object, selector)
}

unsafe fn send_cstr(object: *mut c_void, selector: *mut c_void) -> *const c_char {
    let send: extern "C" fn(*mut c_void, *mut c_void) -> *const c_char =
        std::mem::transmute(objc_msgSend as *const ());
    send(object, selector)
}

/// Bundle identifier of the frontmost application, if there is one.
pub fn frontmost_bundle() -> Option<String> {
    unsafe {
        let workspace_class = objc_getClass(c"NSWorkspace".as_ptr());
        if workspace_class.is_null() {
            return None;
        }
        let workspace = send0(workspace_class, sel_registerName(c"sharedWorkspace".as_ptr()));
        if workspace.is_null() {
            return None;
        }
        let application = send0(workspace, sel_registerName(c"frontmostApplication".as_ptr()));
        if application.is_null() {
            return None;
        }
        let bundle = send0(application, sel_registerName(c"bundleIdentifier".as_ptr()));
        if bundle.is_null() {
            return None;
        }
        let utf8 = send_cstr(bundle, sel_registerName(c"UTF8String".as_ptr()));
        if utf8.is_null() {
            return None;
        }
        CStr::from_ptr(utf8).to_str().ok().map(str::to_owned)
    }
}
