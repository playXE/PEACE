use libc::{dlsym, RTLD_DEFAULT};

use std::ffi::CString;

#[cfg(not(windows))]
pub fn find_symbol(name: &str) -> *const u8 {
    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();
    let sym = unsafe { dlsym(RTLD_DEFAULT, c_str_ptr) };

    if sym.is_null() {
        panic!("can't resolve symbol {}", name);
    }

    sym as *const u8
}

#[cfg(windows)]
pub fn find_symbol(name: &str) -> *const u8 {
    const MSVCRT_DLL: &[u8] = b"msvcrt.dll\0";

    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();

    unsafe {
        let handles = [
            // try to find the searched symbol in the currently running executable
            ptr::null_mut(),
            // try to find the searched symbol in local c runtime
            winapi::um::libloaderapi::GetModuleHandleA(MSVCRT_DLL.as_ptr() as *const i8),
        ];

        for handle in &handles {
            let addr = winapi::um::libloaderapi::GetProcAddress(*handle, c_str_ptr);
            if addr.is_null() {
                continue;
            }
            return addr as *const u8;
        }

        let msg = if handles[1].is_null() {
            "(msvcrt not loaded)"
        } else {
            ""
        };
        panic!("cannot resolve address of symbol {} {}", name, msg);
    }
}
