#[macro_export]
macro_rules! input_descriptor {
    ( $port:expr, $device:expr, $index:expr, $id:expr, $description:literal $(,)? ) => {
        retro_input_descriptor {
            port: $port,
            device: $device,
            index: $index,
            id: $id,
            description: concat!($description, '\0').as_ptr() as *const libc::c_char,
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
