use std::collections::HashMap;

use crate::{build_helpers::*, get_out_path, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    punctuated::Punctuated,
    FnArg, Ident, Item,
    Item::{Const, ForeignMod, Macro, Mod, Struct, Type},
    ItemStruct, ItemType, MacroDelimiter, PathSegment,
};

/// Our source data is a C header, thus all types will declared before their first use
///
/// <Ident, (crate path, new path, new ident)>
#[derive(Debug)]
struct SymbolMap(HashMap<Ident, (TokenStream, TokenStream, Ident)>);

impl SymbolMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(
        &mut self,
        ident: &Ident,
        crate_path: &TokenStream,
        new_module: &Ident,
        new_path: &str,
        new_ident: &Ident,
    ) {
        let mut segments = vec![new_module.clone()];

        if !new_path.is_empty() {
            segments.extend(new_path.split("::").map(|ident| format_ident!("{}", ident)));
        }

        let segments: Vec<PathSegment> = segments.into_iter().map(PathSegment::from).collect();

        let new_path =
            Punctuated::<PathSegment, syn::Token![::]>::from_iter(segments).into_token_stream();

        self.0.insert(
            ident.clone(),
            (crate_path.clone(), new_path, new_ident.clone()),
        );
    }

    pub fn get(&mut self, ident: &Ident) -> Option<&(TokenStream, TokenStream, Ident)> {
        self.0.get(ident)
    }
}

fn name_map(name: &str) -> Option<(&str, &str)> {
    match name {
        "retro_set_sensor_state_t" => Some(("set_sensor_state_t", "sensor")),

        "retro_set_led_state_t" => Some(("set_led_state_t", "led")),
        "retro_set_rumble_state_t" => Some(("set_rumble_state_t", "rumble")),

        "retro_environment_t" => Some(("environment_t", "environment")),

        "retro_message" => Some(("message", "message")),
        "retro_message_ext" => Some(("message_ext", "message")),

        "retro_hw_context_reset_t" => Some(("reset_t", "hw::render_context")),
        "retro_hw_render_context_negotiation_interface" => {
            Some(("negotiation_interface", "hw::render_context"))
        }
        "retro_hw_render_context_negotiation_interface_vulkan" => {
            Some(("negotiation_interface_vulkan", "hw::render_context"))
        }

        // Disk control interface
        "retro_disk_control_callback" => Some(("disk_control_callback", "disk_control_interface")),
        "retro_disk_control_ext_callback" => {
            Some(("disk_control_ext_callback", "disk_control_interface"))
        }
        "retro_set_eject_state_t" => Some(("set_eject_state_t", "disk_control_interface")),
        "retro_get_eject_state_t" => Some(("get_eject_state_t", "disk_control_interface")),
        "retro_get_image_index_t" => Some(("get_image_index_t", "disk_control_interface")),
        "retro_set_image_index_t" => Some(("set_image_index_t", "disk_control_interface")),
        "retro_get_num_images_t" => Some(("get_num_images_t", "disk_control_interface")),
        "retro_replace_image_index_t" => Some(("replace_image_index_t", "disk_control_interface")),
        "retro_add_image_index_t" => Some(("add_image_index_t", "disk_control_interface")),
        "retro_set_initial_image_t" => Some(("set_initial_image_t", "disk_control_interface")),
        "retro_get_image_path_t" => Some(("get_image_path_t", "disk_control_interface")),
        "retro_get_image_label_t" => Some(("get_image_label_t", "disk_control_interface")),

        "retro_get_cpu_features_t" => Some(("get_cpu_features_t", "perf")),

        "retro_get_proc_address_interface" => {
            Some(("get_proc_address_interface", "proc_address_interface"))
        }
        "retro_get_proc_address_t" => Some(("get_proc_address_t", "proc_address_interface")),
        "retro_proc_address_t" => Some(("proc_address_t", "proc_address_interface")),

        "RETRO_MEMORY_MASK" => Some(("MASK", "memory::memory_type")),
        "RETRO_MEMORY_SAVE_RAM" => Some(("SAVE_RAM", "memory::memory_type")),
        "RETRO_MEMORY_RTC" => Some(("RTC", "memory::memory_type")),
        "RETRO_MEMORY_SYSTEM_RAM" => Some(("SYSTEM_RAM", "memory::memory_type")),
        "RETRO_MEMORY_VIDEO_RAM" => Some(("VIDEO_RAM", "memory::memory_type")),

        "RETRO_NUM_CORE_OPTION_VALUES_MAX" => Some(("NUM_CORE_OPTION_VALUES_MAX", "core_option")),

        _ => None,
    }
}

fn enum_map(name: &str) -> Option<(&str, &str)> {
    match name {
        "retro_camera_buffer" => Some(("RETRO_CAMERA_BUFFER_", "camera::buffer_type")),

        "retro_hw_context_type" => Some(("RETRO_HW_CONTEXT_", "hw::render_context::context_type")),
        "retro_hw_render_context_negotiation_interface_type" => Some((
            "RETRO_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE_",
            "hw::render_context::negotiation_interface_type",
        )),
        "retro_hw_render_interface_type" => {
            Some(("RETRO_HW_RENDER_INTERFACE_", "hw::render_interface_type"))
        }

        "retro_key" => Some(("RETROK_", "input::keyboard::key")),
        "retro_mod" => Some(("RETROKMOD_", "input::keyboard::modifier")),

        "retro_language" => Some(("RETRO_LANGUAGE_", "language")),

        "retro_pixel_format" => Some(("RETRO_PIXEL_", "pixel_format")),

        "retro_rumble_effect" => Some(("RETRO_RUMBLE_", "rumble::effect")),

        "retro_savestate_context" => Some(("RETRO_SAVESTATE_CONTEXT_", "savestate_context")),

        "retro_sensor_action" => Some(("RETRO_SENSOR_", "sensor::action")),

        "retro_log_level" => Some(("RETRO_LOG_", "logging::level")),
        "retro_message_target" => Some(("RETRO_MESSAGE_TARGET_", "message::target")),
        "retro_message_type" => Some(("RETRO_MESSAGE_TYPE_", "message::message_type")),
        _ => None,
    }
}

// prefix, module
#[rustfmt::skip]
const PREFIX_MAP: &[(&str, &str)] = &[
    ("retro_audio_", "audio"),
    ("retro_camera_", "camera"),
    ("retro_core_", "core_option"),
    ("retro_frame_time_", "frame_time"),
    ("retro_led_", "led"),
    ("retro_location_", "location"),
    ("retro_log_", "logging"),
    ("retro_memory_", "memory"),
    ("retro_message_", "message"),
    ("retro_midi_", "midi"),
    ("retro_perf_", "perf"),
    ("retro_rumble_", "rumble"),

    // Input devices
    ("retro_controller_", "input::controller"),
    ("retro_keyboard_", "input::keyboard"),
    ("retro_input_", "input"),
    ("RETRO_DEVICE_ID_ANALOG_", "input::device::id::analog"),
    ("RETRO_DEVICE_ID_JOYPAD_", "input::device::id::joypad"),
    ("RETRO_DEVICE_ID_LIGHTGUN_", "input::device::id::lightgun"),
    ("RETRO_DEVICE_ID_MOUSE_", "input::device::id::mouse"),
    ("RETRO_DEVICE_ID_POINTER_", "input::device::id::pointer"),
    ("RETRO_DEVICE_ID_", "input::device::id"),
    ("RETRO_DEVICE_INDEX_", "input::device::index"),
    ("RETRO_DEVICE_", "input::device"),

    ("RETRO_ENVIRONMENT_", "environment"),

    ("retro_hw_", "hw"),
    ("RETRO_HW_RENDER_CONTEXT_", "hw::render_context"),
    ("RETRO_HW_RENDER_INTERFACE_", "hw::render_interface"),
    ("RETRO_HW_", "hw"),

    ("RETRO_MEMDESC_", "memory::descriptor_flags"),
    ("RETRO_MEMORY_", "memory"),

    ("RETRO_REGION_", "region"),

    ("retro_sensor_", "sensor"),
    ("RETRO_SENSOR_", "sensor"),

    ("RETRO_SERIALIZATION_QUIRK_", "serialization_quirk"),

    ("RETRO_SIMD_", "simd"),

    ("RETRO_THROTTLE_", "throttle"),

    ("retro_vfs_", "vfs"),
    ("RETRO_VFS_FILE_ACCESS_", "vfs::file_access"),
    ("RETRO_VFS_SEEK_", "vfs::seek"),
    ("RETRO_VFS_STAT_", "vfs::stat"),
    ("RETRO_VFS_", "vfs"),
];

fn strip_prefix(original_item_name: &str) -> Ident {
    let ident = original_item_name
        .strip_prefix("retro_vulkan_")
        .or_else(|| original_item_name.strip_prefix("retro_"))
        .or_else(|| original_item_name.strip_prefix("RETRO_"))
        .unwrap_or(original_item_name);

    let is_raw = matches!(ident, "mod" | "type");

    if is_raw {
        format_ident!("r#{}", ident)
    } else {
        format_ident!("{}", ident)
    }
}

#[derive(Debug)]
struct Module {
    attrs: TokenStream,
    ident: Ident,
    content: TokenStream,
    children: Vec<Module>,
}

impl ToTokens for Module {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attrs = &self.attrs;
        let ident = &self.ident;
        let content = &self.content;
        let children = &self.children;

        tokens.extend(quote! {
            #attrs
            pub mod #ident {
                #content
                #(#children)*
            }
        });
    }
}

impl Module {
    pub fn new(ident: Ident) -> Self {
        Self {
            attrs: TokenStream::new(),
            ident,
            content: quote!(),
            children: vec![],
        }
    }

    pub fn lookup(&mut self, ident: Ident) -> &mut Module {
        if let Some(index) = self.children.iter_mut().position(|c| c.ident == ident) {
            return &mut self.children[index];
        }

        // Create new module
        self.children.push(Module::new(ident));
        self.children.last_mut().unwrap()
    }

    pub fn ingest<'a>(module: &'a mut Module, path: &str, content: TokenStream) -> &'a mut Module {
        if path.is_empty() {
            module.content.extend(content);
            module
        } else if path.contains("::") {
            let [before, after]: [&str; 2] =
                path.splitn(2, "::").collect::<Vec<_>>().try_into().unwrap();
            let before = format_ident!("{}", before);

            let sub_mod = module.lookup(before);
            Self::ingest(sub_mod, after, content)
        } else {
            let sub_mod = module.lookup(format_ident!("{}", path));
            sub_mod.content.extend(content);
            sub_mod
        }
    }
}

fn handle_items(
    module: &mut Module,
    items: &[Item],
    crate_path: TokenStream,
    known_symbols: &mut SymbolMap,
) {
    'item_loop: for item in items {
        // Ignore bindgen generated tests
        if is_test(item) {
            continue;
        }

        // Handle extern "C" { … }
        if let ForeignMod(ext) = item {
            assert_eq!(
                ext.abi.name.as_ref().map(|n| n.value()),
                Some(String::from("C"))
            );

            for item in &ext.items {
                match item {
                    // Rename functions and their argument types
                    syn::ForeignItem::Fn(item) => {
                        let ident = &item.sig.ident;
                        let ident_str = ident.to_string();
                        let attrs = copy_attributes(&item.attrs);

                        let mut sig = item.sig.clone();

                        // rename the function
                        sig.ident = strip_prefix(&ident_str);

                        // rename the arguments types
                        for input in sig.inputs.iter_mut() {
                            if let FnArg::Typed(ref mut arg) = input {
                                let ty = &mut *arg.ty;

                                match ty {
                                    // rename args like `arg1: retro_input_poll_t` => `arg1: input_poll_t`
                                    syn::Type::Path(ref type_path) => {
                                        if let Some(ident) = type_path.path.get_ident() {
                                            if let Some((_crate_path, new_path, new_ident)) =
                                                known_symbols.get(ident)
                                            {
                                                *ty = syn::Type::Verbatim(quote! {
                                                    #new_path :: #new_ident
                                                });
                                            }
                                        }
                                    }
                                    // rename args like `arg1: *const retro_game_info` => `arg1: *const game_info`
                                    syn::Type::Ptr(ref mut ptr) => {
                                        let ty = &mut *ptr.elem;
                                        if let syn::Type::Path(ref type_path) = ty {
                                            if let Some(ident) = type_path.path.get_ident() {
                                                if let Some((_crate_path, new_path, new_ident)) =
                                                    known_symbols.get(ident)
                                                {
                                                    *ty = syn::Type::Verbatim(quote! {
                                                        #new_path :: #new_ident
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    n => unreachable!("{n:#?}"),
                                }
                            }
                        }

                        let comment = format!("Alias for [`{}::{ident_str}`]", crate_path);
                        let content = quote! {
                            extern "C" {
                                #(#attrs)*
                                #[doc = ""]
                                #[doc = #comment]
                                #[link_name = #ident_str]
                                pub #sig;
                            }
                        };

                        known_symbols.insert(ident, &crate_path, &module.ident, "", &sig.ident);
                        Module::ingest(module, "", content);
                    }
                    n => unreachable!("{n:?}"),
                }
            }

            continue;
        }

        if let Macro(item) = item {
            if let Some(ident) = &item.ident {
                let attrs = &item.attrs;
                let ident_str = ident.to_string();

                let new_ident = strip_prefix(&ident_str);

                let mac = &item.mac;
                let mut tokens = TokenStream::new();

                match &mac.delimiter {
                    MacroDelimiter::Paren(paren) => {
                        paren.surround(&mut tokens, |tokens| mac.tokens.to_tokens(tokens));
                    }
                    MacroDelimiter::Brace(brace) => {
                        brace.surround(&mut tokens, |tokens| mac.tokens.to_tokens(tokens));
                    }
                    MacroDelimiter::Bracket(bracket) => {
                        bracket.surround(&mut tokens, |tokens| mac.tokens.to_tokens(tokens));
                    }
                }

                let content = quote! {
                    #(#attrs)*
                    macro_rules! #new_ident #tokens
                };

                known_symbols.insert(ident, &crate_path, &module.ident, "", &new_ident);
                Module::ingest(module, "", content);

                continue;
            }

            unreachable!("Unhandled macro: {item:#?}");
        }

        if !is_public(item) {
            continue;
        }

        match item {
            Const(item) => {
                let ident = &item.ident;
                let ident_str = ident.to_string();

                let ty = &item.ty;
                let attrs = copy_attributes(&item.attrs);

                let mut path = "";
                let mut new_ident = None;

                if let Some((mapped_ident, mapped_path)) = name_map(&ident_str) {
                    path = mapped_path;
                    new_ident.replace(format_ident!("{}", mapped_ident));
                }

                if new_ident.is_none() {
                    for group in PREFIX_MAP {
                        if let Some(new_name) = ident_str.strip_prefix(group.0) {
                            new_ident.replace(format_ident!("{}", new_name));
                            path = group.1;
                            break;
                        }
                    }
                }

                let new_ident = new_ident.unwrap_or_else(|| strip_prefix(&ident_str));
                let content = quote! {
                    #(#attrs)*
                    pub const #new_ident: #ty = #crate_path::#ident;
                };

                known_symbols.insert(ident, &crate_path, &module.ident, path, &new_ident);
                Module::ingest(module, path, content);
            }
            Struct(ItemStruct {
                ident,
                generics,
                attrs,
                ..
            })
            | Type(ItemType {
                ident,
                generics,
                attrs,
                ..
            }) => {
                let ident_str = ident.to_string();

                let mut path = "";
                let mut new_ident = None;

                if let Some((mapped_ident, mapped_path)) = name_map(&ident_str) {
                    path = mapped_path;
                    new_ident.replace(format_ident!("{}", mapped_ident));
                }

                if new_ident.is_none() {
                    for group in PREFIX_MAP {
                        if let Some(new_name) = ident_str.strip_prefix(group.0) {
                            new_ident.replace(format_ident!("{}", new_name));
                            path = group.1;
                            break;
                        }
                    }
                }

                let new_ident = new_ident.unwrap_or_else(|| strip_prefix(&ident_str));
                let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();
                let attrs = copy_attributes(attrs);

                let alias = quote! {
                    #(#attrs)*
                    pub type #new_ident #ty_generics = #crate_path::#ident #ty_generics;
                };

                known_symbols.insert(ident, &crate_path, &module.ident, path, &new_ident);
                Module::ingest(module, path, alias);
            }
            Mod(item) => {
                // Handles constified enum modules
                let ident = &item.ident;
                let ident_str = ident.to_string();

                if let Some((_new_ident, _path)) = name_map(&ident_str) {
                    unimplemented!("Renaming and moving enums");
                }

                if let Some((prefix, path)) = enum_map(&ident_str) {
                    // We found a specific mapping for this C enum.
                    // We move the constants into the given path, remove the given prefix
                    // from the identifiers of the constants and fix their data type.

                    let mut items = item.content.as_ref().unwrap().1.clone();

                    // We expect the first item of a constified enum module to be its type
                    if let Item::Type(ty) = items.remove(0) {
                        assert_eq!(ty.ident, format_ident!("Type"));
                    } else {
                        panic!("exptected pub type Type = ...");
                    };

                    for item in &mut items {
                        match item {
                            Item::Const(constant) => {
                                let const_ident = &constant.ident;
                                let const_ident_str = const_ident.to_string();

                                if let Some(new_name) = const_ident_str.strip_prefix(prefix) {
                                    if ident_str == "retro_key"
                                        && ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
                                            .contains(&new_name)
                                    {
                                        constant.ident = format_ident!("KEY_{}", new_name);
                                    } else {
                                        constant.ident = format_ident!("{}", new_name);
                                    }
                                } else {
                                    unreachable!(
                                        "“{const_ident_str}” failed to match prefix “{prefix}”"
                                    );
                                }

                                *constant.ty = syn::Type::Verbatim(quote! {
                                    #crate_path::#ident::Type
                                });

                                known_symbols.insert(
                                    ident,
                                    &crate_path,
                                    &module.ident,
                                    path,
                                    &constant.ident,
                                );
                            }
                            n => unreachable!("{:?}", n),
                        }
                    }

                    let content = quote! {
                       #(#items)*
                    };

                    Module::ingest(module, path, content);

                    continue 'item_loop;
                }

                println!("No mapping for enum: {ident_str}");

                // We had no explicit mapping, so keep the module, but remove prefixes
                let mut item = item.clone();
                item.ident = strip_prefix(&item.ident.to_string());

                for item in &mut item.content.as_mut().unwrap().1 {
                    match item {
                        Item::Type(..) => {
                            // Ignored
                        }
                        Item::Const(constant) => {
                            let const_ident = &constant.ident;
                            let const_ident_str = const_ident.to_string();

                            constant.ident = strip_prefix(&const_ident_str);
                        }
                        n => unreachable!("{:?}", n),
                    }
                }

                Module::ingest(module, "", quote!(#item));
            }
            n => unimplemented!("{:?}", n),
        }
    }
}

fn namespace_file(
    module: &mut Module,
    filename: &str,
    crate_path: TokenStream,
    known_symbols: &mut SymbolMap,
) {
    let file_path = get_out_path(filename);
    let file = syn::parse_file(&std::fs::read_to_string(file_path).unwrap()).unwrap();

    handle_items(module, &file.items, crate_path, known_symbols);
}

pub fn generate_namespaced_modules() {
    let mut known_symbols = SymbolMap::new();

    let mut module = Module::new(format_ident!("retro"));
    module.attrs.extend(quote! {
        #[allow(deprecated)]
    });
    module.content.extend(quote!(
        use crate::retro;
    ));

    namespace_file(
        &mut module,
        "bindings_libretro.rs",
        quote!(crate),
        &mut known_symbols,
    );

    let vulkan = module.lookup(format_ident!("{}", "vulkan"));
    vulkan.attrs.extend(quote! {
        #[cfg(feature = "vulkan")]
    });

    namespace_file(
        vulkan,
        "bindings_libretro_vulkan.rs",
        quote!(crate::vulkan),
        &mut known_symbols,
    );

    //panic!("at the disco");

    std::fs::write(
        get_out_path("bindings_namespaced.rs"),
        prettify(&module.to_token_stream().to_string()),
    )
    .expect("writing namespaced bindings to succeed");
}
