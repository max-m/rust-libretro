//! Utility functions
use crate::error::StringError;
use std::{
    ffi::{c_char, CStr, CString},
    fmt::Display,
    path::{Path, PathBuf},
};

#[cfg(target_family = "unix")]
pub mod unix {
    use std::{
        ffi::{CStr, CString, OsStr, OsString},
        os::unix::ffi::{OsStrExt, OsStringExt},
        path::{Path, PathBuf},
    };

    pub fn c_string_to_path(input: &CString) -> &Path {
        Path::new(OsStr::from_bytes(input.as_bytes()))
    }

    pub fn c_string_to_path_buf(input: CString) -> PathBuf {
        PathBuf::from(OsString::from_vec(input.into_bytes()))
    }

    pub fn c_string_to_os_str(input: &CString) -> &OsStr {
        OsStr::from_bytes(input.as_bytes())
    }

    pub fn c_string_to_os_string(input: CString) -> OsString {
        OsString::from_vec(input.into_bytes())
    }

    pub fn c_str_to_path(input: &CStr) -> &Path {
        Path::new(OsStr::from_bytes(input.to_bytes()))
    }

    pub fn c_str_to_path_buf(input: &CStr) -> PathBuf {
        Path::new(OsStr::from_bytes(input.to_bytes())).to_path_buf()
    }

    pub fn c_str_to_os_str(input: &CStr) -> &OsStr {
        OsStr::from_bytes(input.to_bytes())
    }

    pub fn c_str_to_os_string(input: &CStr) -> OsString {
        OsStr::from_bytes(input.to_bytes()).to_os_string()
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn get_cstr_from_pointer<'a>(ptr: *const c_char) -> Result<&'a CStr, StringError> {
    if ptr.is_null() {
        return Err(StringError::NullPointer("string pointer"));
    }

    Ok(unsafe { CStr::from_ptr(ptr) })
}

pub fn get_cstring_from_pointer(ptr: *const c_char) -> Result<CString, StringError> {
    get_cstr_from_pointer(ptr).map(ToOwned::to_owned)
}

/// Tries to convert a pointer to a [`CString`] into a Rust [`str`]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn get_str_from_pointer<'a>(ptr: *const c_char) -> Result<&'a str, StringError> {
    if ptr.is_null() {
        return Err(StringError::NullPointer("string pointer"));
    }

    let slice = unsafe { CStr::from_ptr(ptr) };

    std::str::from_utf8(slice.to_bytes()).map_err(Into::into)
}

/// Tries to convert a pointer to a [`CString`] into a Rust [`String`]
pub fn get_string_from_pointer(ptr: *const c_char) -> Result<String, StringError> {
    get_str_from_pointer(ptr).map(|s| s.to_owned())
}

/// Tries to convert a pointer to a [`CString`] into a Rust [`Path`]
pub fn get_path_from_pointer<'a>(ptr: *const c_char) -> Result<&'a Path, StringError> {
    if ptr.is_null() {
        return Err(StringError::NullPointer("string pointer"));
    }

    let slice = unsafe { CStr::from_ptr(ptr as *const _) };

    cfg_if::cfg_if! {
        if #[cfg(target_family = "unix")] {
            Ok(unix::c_str_to_path(slice))
        }
        else {
            let s = std::str::from_utf8(slice.to_bytes())?;
            let path: &Path = s.as_ref();
            Ok(path)
        }
    }
}

/// Tries to convert a pointer to a [`CString`] into a Rust [`PathBuf`]
pub fn get_path_buf_from_pointer(ptr: *const c_char) -> Result<PathBuf, StringError> {
    get_str_from_pointer(ptr).map(PathBuf::from)
}

#[derive(Debug, Copy, Clone)]
pub struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}

impl Version {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        if patch > 0xfff {
            panic!("Invalid patch level");
        }

        if minor > 0x3ff {
            panic!("Invalid minor level");
        }

        if major > 0x3ff {
            panic!("Invalid major level");
        }

        Self {
            major,
            minor,
            patch,
        }
    }

    pub const fn from_u32(version: u32) -> Self {
        // 0bMMMMMMMMMM_mmmmmmmmmm_pppppppppppp
        Self {
            major: ((version >> 22) & 0x3ff) as u16,
            minor: ((version >> 12) & 0x3ff) as u16,
            patch: (version & 0xfff) as u16,
        }
    }

    pub const fn to_u32(self) -> u32 {
        // 0bMMMMMMMMMM_mmmmmmmmmm_pppppppppppp
        ((self.major as u32) << 22) | ((self.minor as u32) << 12) | (self.patch as u32)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
