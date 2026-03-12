use std::path::PathBuf;
use std::sync::Mutex;

/// File paths received from macOS "Open with..." Apple Events.
pub static OPEN_FILES: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Adds `application:openFiles:` to winit's `WinitApplicationDelegate` class
/// so macOS "Open with..." delivers file paths to the app.
///
/// Must be called after eframe/winit has registered the delegate class
/// (i.e. during or after `CreationContext` callback).
pub fn register_open_handler() {
    use objc2::ffi;
    use std::ffi::CString;

    extern "C" fn open_files(
        _self: *mut ffi::objc_object,
        _cmd: *const ffi::objc_selector,
        _app: *mut ffi::objc_object,
        filenames: *mut ffi::objc_object,
    ) {
        use objc2::msg_send;
        use objc2::runtime::AnyObject;
        use std::ffi::CStr;

        if filenames.is_null() {
            return;
        }
        let filenames: &AnyObject = unsafe { &*(filenames as *const AnyObject) };

        let count: usize = unsafe { msg_send![filenames, count] };
        for i in 0..count {
            let nsstring: *const AnyObject = unsafe { msg_send![filenames, objectAtIndex: i] };
            if nsstring.is_null() {
                continue;
            }
            let utf8: *const std::ffi::c_char =
                unsafe { msg_send![&*nsstring, UTF8String] };
            if utf8.is_null() {
                continue;
            }
            let s = unsafe { CStr::from_ptr(utf8) }.to_string_lossy();
            if let Ok(mut files) = OPEN_FILES.lock() {
                files.push(PathBuf::from(s.as_ref()));
            }
        }
    }

    unsafe {
        let class_name = CString::new("WinitApplicationDelegate").unwrap();
        let cls = ffi::objc_getClass(class_name.as_ptr());
        if cls.is_null() {
            return;
        }

        let sel = ffi::sel_registerName(c"application:openFiles:".as_ptr());
        let types = CString::new("v@:@@").unwrap();

        ffi::class_addMethod(
            cls as *mut _,
            sel,
            Some(std::mem::transmute::<
                extern "C" fn(
                    *mut ffi::objc_object,
                    *const ffi::objc_selector,
                    *mut ffi::objc_object,
                    *mut ffi::objc_object,
                ),
                unsafe extern "C" fn(),
            >(open_files)),
            types.as_ptr(),
        );
    }
}
