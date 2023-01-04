use bindgen::callbacks::MacroParsingBehavior;
use std::{env, path::PathBuf};

#[derive(Debug)]
pub struct ParseCallbacks;

impl bindgen::callbacks::ParseCallbacks for ParseCallbacks {
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
        println!("cargo:rerun-if-changed={}", filename);
    }

    fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
        match info.name {
            "retro_savestate_context" => vec!["TryFromPrimitive".to_owned()],
            "retro_hw_render_interface_vulkan" => vec!["Copy".to_owned(), "Clone".to_owned()],
            // Other structs get these #[derive]s, but retro_hw_render_interface_vulkan doesn't for some reason
            _ => Vec::with_capacity(0),
        }
    }
}

fn main() {
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let retro_bindings = bindgen::Builder::default()
        .header("libretro.h")
        .clang_arg("-I.")
        .allowlist_type("(retro|RETRO)_.*")
        .allowlist_function("(retro|RETRO)_.*")
        .allowlist_var("(retro|RETRO)_.*")
        .blocklist_type("Vk.*")
        .blocklist_type("PFN_vk.*")
        .prepend_enum_name(false)
        .impl_debug(true)
        .clang_arg("-fparse-all-comments")
        .enable_function_attribute_detection()
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: true,
        })
        .newtype_enum("retro_key")
        .bitfield_enum("retro_mod")
        .size_t_is_usize(true)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(ParseCallbacks))
        .generate()
        .expect("Unable to generate bindings for libretro.h");

    // We generate two separate binding files to work around a bug in CLion's Rust plugin.
    // CLion can't (as of this writing) detect bindgen'd types that are conditionally generated
    // based on the set features.
    // As a workaround we generate all bindings unconditionally
    // but use #[cfg] in lib.rs to exclude irrelevant ones from the build.
    let retro_vulkan_bindings = bindgen::Builder::default()
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
        .size_t_is_usize(true)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(ParseCallbacks))
        .generate()
        .expect("Unable to generate bindings for libretro_vulkan.h");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    retro_bindings
        .write_to_file(out_path.join("bindings_libretro.rs"))
        .expect("Couldn't write bindings for libretro.h!");

    retro_vulkan_bindings
        .write_to_file(out_path.join("bindings_libretro_vulkan.rs"))
        .expect("Couldn't write bindings for libretro_vulkan.h!");
}
