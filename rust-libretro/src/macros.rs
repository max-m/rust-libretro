#[macro_export]
macro_rules! c_char_ptr {
    ( $($arg:tt)* ) => {
        concat!($($arg)*, '\0').as_ptr() as *const libc::c_char
    };
}

#[macro_export]
macro_rules! c_str {
    ( $($arg:tt)* ) => {
        unsafe { std::ffi::CStr::from_ptr(c_char_ptr!($($arg)*)) }
    };
}

#[macro_export]
macro_rules! input_descriptor {
    ( $port:expr, $device:expr, $index:expr, $id:expr, $description:literal $(,)? ) => {
        retro_input_descriptor {
            port: $port,
            device: $device,
            index: $index,
            id: $id,
            description: $crate::c_char_ptr!($description),
        }
    };
}

#[macro_export]
macro_rules! input_descriptors {
    ( $({ $port:expr, $device:expr, $index:expr, $id:expr, $description:literal $(,)? }),* $(,)? ) => { [
        $(
            $crate::input_descriptor!($port, $device, $index, $id, $description),
        )*
        // End of list
        $crate::input_descriptor!(0, 0, 0, 0, "")
    ] }
}

#[macro_export]
macro_rules! env_version {
    ( $variable:literal ) => {{
        let parts: [&str; 3] = $crate::const_str::split!(env!($variable), ".");
        let major = $crate::const_str::parse!(parts[0], u16);
        let minor = $crate::const_str::parse!(parts[1], u16);
        let patch = $crate::const_str::parse!(parts[2], u16);
        $crate::util::Version::new(major, minor, patch)
    }};
}
