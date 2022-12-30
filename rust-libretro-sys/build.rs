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
        println!("Including {filename}");
    }
}

fn main() {
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
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
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(ParseCallbacks));

    builder = if cfg!(feature = "vulkan") {
        builder.clang_arg("-DRUST_FEATURE_VULKAN")
    }
    else {
        builder
    };

    let bindings = builder
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
