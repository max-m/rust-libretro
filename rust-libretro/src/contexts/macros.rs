#[doc(hidden)]
macro_rules! into_generic {
    ($type:ty, $lifetime:tt) => {
        into_generic!($type, GenericContext, $lifetime);
    };
    ($type:ty, $other:ident, $lifetime:tt) => {
        impl<$lifetime> From<&$type> for $other<$lifetime> {
            fn from(other: &$type) -> $other<$lifetime> {
                $other::new(other.environment_callback, Arc::clone(&other.interfaces))
            }
        }

        impl<$lifetime> From<&mut $type> for $other<$lifetime> {
            fn from(other: &mut $type) -> $other<$lifetime> {
                $other::new(other.environment_callback, Arc::clone(&other.interfaces))
            }
        }
    };
}

#[doc(hidden)]
macro_rules! make_context {
    ($name:ident $(, #[doc = $doc:tt ])?) => {
        $(#[doc = $doc])?
        pub struct $name<'a> {
            pub(crate) environment_callback: &'a retro_environment_t,
            pub(crate) interfaces: Interfaces,
        }

        impl<'a> $name<'a> {
            pub(crate) fn new(environment_callback: &'a retro_environment_t, interfaces: Interfaces) -> Self {
                Self {
                    environment_callback,
                    interfaces
                }
            }
        }

        into_generic!($name<'a>, 'a);
    };
}

#[doc(hidden)]
macro_rules! get_interface {
    ($interfaces:ident, $( $interface_ident:ident ).+, $interface_name:literal, $enable_fn:ident) => {
        $interfaces.$($interface_ident).+.ok_or(
            EnvironmentCallError::InterfaceNotFound(
                $interface_name,
                concat!(stringify!($enable_fn), "()"),
            ),
        )?
    };
}

#[doc(hidden)]
macro_rules! unwrap_interface_function {
    ($interface:ident, $fn_name:ident) => {
        $interface
            .$fn_name
            .ok_or(EnvironmentCallError::NullPointer(stringify!($fn_name)))?
    };
}

#[doc(hidden)]
macro_rules! get_interface_function {
    ($interfaces:ident, $( $interface_ident:ident ).+, $interface_name:literal, $enable_fn:ident, $fn_name:ident) => {{
        let interface = get_interface!($interfaces, $($interface_ident).+, $interface_name, $enable_fn);

        unwrap_interface_function!(interface, $fn_name)
    }};
}

#[doc(hidden)]
macro_rules! get_location_interface_function {
    ($interfaces:ident, $fn_name:ident) => {{
        get_interface_function!(
            $interfaces,
            location_interface,
            "Location",
            enable_location_interface,
            $fn_name
        )
    }};
}

#[doc(hidden)]
macro_rules! get_perf_interface {
    ($interfaces:ident) => {{
        get_interface!(
            $interfaces,
            perf_interface.interface,
            "Performance",
            enable_perf_interface
        )
    }};
}

#[doc(hidden)]
macro_rules! get_perf_interface_function {
    ($interfaces:ident, $fn_name:ident) => {{
        let interface = get_perf_interface!($interfaces);

        unwrap_interface_function!(interface, $fn_name)
    }};
}

#[doc(hidden)]
macro_rules! get_rumble_interface_function {
    ($interfaces:ident, $fn_name:ident) => {{
        get_interface_function!(
            $interfaces,
            rumble_interface,
            "Rumble",
            enable_rumble_interface,
            $fn_name
        )
    }};
}

#[doc(hidden)]
#[crate::proc::unstable(feature = "env-commands")]
macro_rules! get_camera_interface_function {
    ($interfaces:ident, $fn_name:ident) => {{
        get_interface_function!(
            $interfaces,
            camera_interface,
            "Camera",
            enable_camera_interface,
            $fn_name
        )
    }};
}

#[doc(hidden)]
#[crate::proc::unstable(feature = "env-commands")]
macro_rules! get_led_interface_function {
    ($interfaces:ident, $fn_name:ident) => {{
        get_interface_function!(
            $interfaces,
            led_interface,
            "LED",
            enable_led_interface,
            $fn_name
        )
    }};
}

#[doc(hidden)]
#[crate::proc::unstable(feature = "env-commands")]
macro_rules! get_sensor_interface_function {
    ($interfaces:ident, $fn_name:ident) => {{
        get_interface_function!(
            $interfaces,
            sensor_interface,
            "Sensor",
            enable_sensor_interface,
            $fn_name
        )
    }};
}

#[doc(hidden)]
#[crate::proc::unstable(feature = "env-commands")]
macro_rules! get_midi_interface_function {
    ($interfaces:ident, $fn_name:ident) => {{
        get_interface_function!(
            $interfaces,
            midi_interface,
            "MIDI",
            enable_midi_interface,
            $fn_name
        )
    }};
}

#[doc(hidden)]
#[crate::proc::unstable(feature = "env-commands")]
macro_rules! get_vfs_function {
    ($interfaces:ident, $fn_name:ident, $version:expr) => {
        if $interfaces.vfs_interface_info.supported_version < $version {
            return Err(VfsError::VersionMismatch(
                $interfaces.vfs_interface_info.supported_version,
                $version,
            )
            .into());
        } else {
            get_vfs_function!($interfaces, $fn_name)
        }
    };
    ($interfaces:ident, $fn_name:ident) => {{
        get_interface_function!(
            $interfaces,
            vfs_interface_info.interface,
            "VFS",
            enable_vfs_interface,
            $fn_name
        )
    }};
}
