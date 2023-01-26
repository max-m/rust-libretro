use bindgen::{callbacks::MacroParsingBehavior, Bindings};
use std::{env, path::PathBuf};

fn libretro_h() -> bindgen::Builder {
    #[derive(Debug)]
    pub struct LibretroParseCallbacks;

    impl bindgen::callbacks::ParseCallbacks for LibretroParseCallbacks {
        fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
            // These macro constants are deprecated, we define them in our lib.rs instead
            match name {
                "RETRO_DEVICE_ID_LIGHTGUN_X"
                | "RETRO_DEVICE_ID_LIGHTGUN_Y"
                | "RETRO_DEVICE_ID_LIGHTGUN_CURSOR"
                | "RETRO_DEVICE_ID_LIGHTGUN_TURBO"
                | "RETRO_DEVICE_ID_LIGHTGUN_PAUSE" => MacroParsingBehavior::Ignore,
                _ => MacroParsingBehavior::Default,
            }
        }

        /// When running inside a `build.rs` script, this can be used to make cargo invalidate the
        /// generated bindings whenever any of the files included from the header change:
        fn include_file(&self, filename: &str) {
            println!("cargo:rerun-if-changed={filename}");
        }

        fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
            match info.name {
                "retro_language"
                // | "retro_key"
                // | "retro_mod"
                | "retro_hw_render_interface_type"
                | "retro_hw_render_context_negotiation_interface_type"
                | "retro_log_level"
                | "retro_sensor_action"
                | "retro_camera_buffer"
                | "retro_rumble_effect"
                | "retro_hw_context_type"
                | "retro_pixel_format"
                | "retro_savestate_context"
                | "retro_message_target"
                | "retro_message_type" => vec!["TryFromPrimitive".to_owned()],
                _ => Vec::with_capacity(0),
            }
        }
    }

    println!("cargo:rerun-if-changed=libretro.h");

    bindgen::Builder::default()
        .header("libretro.h")
        .clang_arg("-I.")
        .allowlist_type("(retro|RETRO)_.*")
        .allowlist_function("(retro|RETRO)_.*")
        .allowlist_var("(retro|RETRO)_.*")
        .prepend_enum_name(false)
        .impl_debug(true)
        .clang_arg("-fparse-all-comments")
        .enable_function_attribute_detection()
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: true,
        })
        .newtype_enum("retro_key")
        .bitfield_enum("retro_mod")
        .parse_callbacks(Box::new(LibretroParseCallbacks))
}

fn libretro_vulkan_h() -> bindgen::Builder {
    #[derive(Debug)]
    pub struct LibretroVulkanParseCallbacks;

    impl bindgen::callbacks::ParseCallbacks for LibretroVulkanParseCallbacks {
        /// When running inside a `build.rs` script, this can be used to make cargo invalidate the
        /// generated bindings whenever any of the files included from the header change:
        fn include_file(&self, filename: &str) {
            println!("cargo:rerun-if-changed={filename}");
        }

        fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
            match info.name {
                // Other structs get these #[derive]s, but retro_hw_render_interface_vulkan doesn't for some reason
                "retro_hw_render_interface_vulkan" => vec!["Clone".to_owned()],
                _ => Vec::with_capacity(0),
            }
        }
    }

    println!("cargo:rerun-if-changed=libretro_vulkan.h");

    bindgen::Builder::default()
        .header("libretro_vulkan.h")
        .clang_arg("-I.")
        .allowlist_type("retro.*vulkan")
        .allowlist_var("RETRO_.+_VULKAN_VERSION")
        .blocklist_type("^retro_hw_render.*type$")
        .blocklist_type("Vk.*")
        .blocklist_type("PFN_vk.*")
        .prepend_enum_name(false)
        .impl_debug(true)
        .clang_arg("-fparse-all-comments")
        .enable_function_attribute_detection()
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: true,
        })
        .parse_callbacks(Box::new(LibretroVulkanParseCallbacks))
}

fn save_bindings(bindings: Bindings, name: &str) {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join(name))
        .expect("Couldn't write bindings!");
}

fn main() {
    let bindings = libretro_h()
        .generate()
        .expect("Unable to generate libretro.h bindings");
    save_bindings(bindings, "bindings_libretro.rs");

    let bindings = libretro_vulkan_h()
        .generate()
        .expect("Unable to generate libretro_vulkan.h bindings");
    save_bindings(bindings, "bindings_libretro_vulkan.rs");
}
