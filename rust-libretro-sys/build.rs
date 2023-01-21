use crate::build_helpers::generate_namespaced_modules;
use bindgen::{
    callbacks::{DeriveTrait, ImplementsTrait, MacroParsingBehavior},
    Bindings,
};
use std::{env, path::PathBuf};

// Well ... it’s technically public and we have functions that
// return `quote!()`
use quote::__private::TokenStream;

mod build_helpers;

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
            println!("cargo:rerun-if-changed={}", filename);
        }
    }

    println!("cargo:rerun-if-changed=libretro.h");
    println!("cargo:rerun-if-changed=src/bindings_libretro_preamble.rs");

    bindgen::Builder::default()
        .header("libretro.h")
        .clang_arg("-I.")
        .clang_arg("-fparse-all-comments")
        .allowlist_type("(retro|RETRO)_.*")
        .allowlist_function("(retro|RETRO)_.*")
        .allowlist_var("(retro|RETRO)_.*")
        .prepend_enum_name(false)
        .impl_debug(true)
        .enable_function_attribute_detection()
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .parse_callbacks(Box::new(LibretroParseCallbacks))
        .raw_line({
            let file = include_str!("src/bindings_libretro_preamble.rs");
            // skip the first line
            let offset = file.find('\n').unwrap_or(0);
            &file[offset + 1..]
        })
}

fn libretro_vulkan_h() -> bindgen::Builder {
    #[derive(Debug)]
    pub struct LibretroVulkanParseCallbacks;

    impl bindgen::callbacks::ParseCallbacks for LibretroVulkanParseCallbacks {
        /// When running inside a `build.rs` script, this can be used to make cargo invalidate the
        /// generated bindings whenever any of the files included from the header change:
        fn include_file(&self, filename: &str) {
            println!("cargo:rerun-if-changed={}", filename);
        }

        fn blocklisted_type_implements_trait(
            &self,
            name: &str,
            derive_trait: bindgen::callbacks::DeriveTrait,
        ) -> Option<ImplementsTrait> {
            match name {
                "PFN_vkGetDeviceProcAddr" | "PFN_vkGetInstanceProcAddr" => {
                    if matches!(derive_trait, DeriveTrait::Debug) {
                        return Some(ImplementsTrait::No);
                    } else {
                        return Some(ImplementsTrait::Yes);
                    }
                }
                // These types all implement Copy, Debug, Default, Hash, PartialEqOrPartialOrd
                "PFN_vkVoidFunction"
                | "enum retro_hw_render_context_negotiation_interface_type"
                | "enum retro_hw_render_interface_type"
                | "VkDevice"
                | "VkImageLayout"
                | "VkImageView"
                | "VkImageViewCreateInfo"
                | "VkInstance"
                | "VkPhysicalDevice"
                | "VkQueue"
                | "VkSemaphore"
                | "VkSurfaceKHR" => return Some(ImplementsTrait::Yes),
                _ => (),
            }
            None
        }
    }

    println!("cargo:rerun-if-changed=libretro_vulkan.h");
    println!("cargo:rerun-if-changed=src/bindings_libretro_vulkan_preamble.rs");

    bindgen::Builder::default()
        .header("libretro_vulkan.h")
        .clang_arg("-I.")
        .clang_arg("-fparse-all-comments")
        .allowlist_type("retro.*vulkan")
        .allowlist_var("RETRO_.+_VULKAN_VERSION")
        .blocklist_type("^retro_hw_render.*type$")
        .blocklist_type("Vk.*")
        .blocklist_type("PFN_vk.*")
        .impl_debug(false)
        .enable_function_attribute_detection()
        .parse_callbacks(Box::new(LibretroVulkanParseCallbacks))
        .raw_line({
            let file = include_str!("src/bindings_libretro_vulkan_preamble.rs");
            // skip the first line
            let offset = file.find('\n').unwrap_or(0);
            &file[offset + 1..]
        })
}

pub fn get_out_path(name: &str) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    out_path.join(name)
}

fn save_bindings(bindings: Bindings, name: &str) {
    let out_path = get_out_path(name);

    bindings.emit_warnings();

    bindings
        .write_to_file(&out_path)
        .unwrap_or_else(|_| panic!("writing bindings to {out_path:?} to succeed"));
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

    generate_namespaced_modules();
}
