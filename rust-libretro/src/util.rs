//! Utility functions
use super::*;

/// Tries to convert a pointer to a [`CString`] into a Rust [`str`]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn get_str_from_pointer<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }

    let slice = unsafe { CStr::from_ptr(ptr) };

    std::str::from_utf8(slice.to_bytes()).ok()
}

/// Tries to convert a pointer to a [`CString`] into a Rust [`String`]
pub fn get_string_from_pointer(ptr: *const c_char) -> Option<String> {
    get_str_from_pointer(ptr).map(|s| s.to_owned())
}

/// Tries to convert a pointer to a [`CString`] into a Rust [`Path`]
pub fn get_path_from_pointer<'a>(ptr: *const c_char) -> Option<&'a Path> {
    if ptr.is_null() {
        return None;
    }

    let slice = unsafe { CStr::from_ptr(ptr as *const _) };

    cfg_if::cfg_if! {
        if #[cfg(target_family = "unix")] {
            use std::os::unix::ffi::OsStrExt;
            let oss = OsStr::from_bytes(slice.to_bytes());
            let path: &Path = oss.as_ref();
            Some(path)
        }
        else {
            let s = std::str::from_utf8(slice.to_bytes()).expect("valid UTF-8");
            let path: &Path = s.as_ref();
            Some(path)
        }
    }
}

/// Tries to convert a pointer to a [`CString`] into a Rust [`PathBuf`]
pub fn get_path_buf_from_pointer(ptr: *mut c_char) -> Option<PathBuf> {
    get_str_from_pointer(ptr).map(PathBuf::from)
}
