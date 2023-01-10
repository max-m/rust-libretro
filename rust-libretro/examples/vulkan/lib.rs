//! Port of <https://github.com/libretro/libretro-samples/blob/bce193bc1b8c9a3da43b2ead0158a69e28b37ed8/video/vulkan/vk_rendering/libretro-test.c>
//!
//! Original license:
//! Copyright  (C) 2010-2015 The RetroArch team
//!
//! Permission is hereby granted, free of charge,
//! to any person obtaining a copy of this software and associated documentation files (the "Software"),
//! to deal in the Software without restriction, including without limitation the rights to
//! use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software,
//! and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
//! INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
//! IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
//! WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use crate::ash::prelude::VkResult;
use rust_libretro::{
    c_char_ptr, c_str,
    contexts::*,
    core::Core,
    env_version,
    proc::CoreOptions,
    retro_core,
    sys::{
        vulkan::{ash, ash::vk},
        *,
    },
    types::*,
    util::Version,
};
use std::{
    ffi::CString,
    ptr::{null, null_mut},
};
use vk_shader_macros::include_glsl;

const CRATE_VERSION: Version = env_version!("CARGO_PKG_VERSION");
const VK_API_VERSION: u32 = vk::make_api_version(0, 1, 0, 18);

const BASE_WIDTH: u32 = 320;
const BASE_HEIGHT: u32 = 240;
const MAX_SYNC: usize = 8;

#[derive(Debug, Default)]
struct Buffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
}

#[derive(Debug)]
struct VulkanData {
    index: usize,
    num_swapchain_images: usize,
    swapchain_mask: u32,
    vbo: Buffer,
    ubo: [Buffer; MAX_SYNC],

    memory_properties: vk::PhysicalDeviceMemoryProperties,
    gpu_properties: vk::PhysicalDeviceProperties,

    set_layout: vk::DescriptorSetLayout,
    desc_pool: vk::DescriptorPool,
    desc_set: [vk::DescriptorSet; MAX_SYNC],

    pipeline_cache: vk::PipelineCache,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,

    images: [retro_vulkan_image; MAX_SYNC],
    image_memory: [vk::DeviceMemory; MAX_SYNC],
    framebuffers: [vk::Framebuffer; MAX_SYNC],
    cmd_pool: [vk::CommandPool; MAX_SYNC],
    cmd: [vk::CommandBuffer; MAX_SYNC],
}

impl Default for VulkanData {
    fn default() -> Self {
        #[allow(invalid_value)]
        unsafe {
            std::mem::MaybeUninit::zeroed().assume_init()
        }
    }
}

#[derive(CoreOptions)]
#[categories({
    "video_settings",
    "Video",
    "Options related to video output."
})]
#[options({
    "testvulkan_resolution",
    "Video > Internal resolution",
    "Internal resolution",
    "Setting 'Video > Internal resolution' forces the internal render resolution to the specified value.",
    "Setting 'Internal resolution' forces the internal render resolution to the specified value",
    "video_settings",
    {
        { "320x240" },
        { "360x480" },
        { "480x272" },
        { "512x384" },
        { "512x512" },
        { "640x240" },
        { "640x448" },
        { "640x480" },
        { "720x576" },
        { "800x600" },
        { "960x720" },
        { "1024x768" },
        { "1024x1024" },
        { "1280x720" },
        { "1280x960" },
        { "1600x1200" },
        { "1920x1080" },
        { "1920x1440" },
        { "1920x1600" },
        { "2048x2048" },
    }
})]
struct TestCore {
    resolution: (u16, u16),
    frame: u64,

    entry: Option<ash::Entry>,
    device: Option<ash::Device>,
    instance: Option<ash::Instance>,
    vulkan: Option<retro_hw_render_interface_vulkan>,
    vk: VulkanData,
}

retro_core!(TestCore {
    resolution: (BASE_WIDTH as u16, BASE_HEIGHT as u16),
    frame: 0,

    entry: None,
    device: None,
    instance: None,
    vulkan: None,
    vk: Default::default(),
});

impl TestCore {
    fn get_av_info(&mut self) -> retro_system_av_info {
        retro_system_av_info {
            geometry: retro_game_geometry {
                base_width: BASE_WIDTH,
                base_height: BASE_HEIGHT,
                max_width: BASE_WIDTH,
                max_height: BASE_HEIGHT,
                aspect_ratio: BASE_WIDTH as f32 / BASE_HEIGHT as f32,
            },
            timing: retro_system_timing {
                fps: 60.0,
                sample_rate: 0.0,
            },
        }
    }

    unsafe extern "C" fn get_application_info() -> *const vk::ApplicationInfo {
        const INFO: vk::ApplicationInfo = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_next: null_mut(),
            p_application_name: c_char_ptr!(env!("CARGO_CRATE_NAME")),
            application_version: CRATE_VERSION.to_u32(),
            p_engine_name: c_char_ptr!(env!("CARGO_PKG_NAME")),
            engine_version: CRATE_VERSION.to_u32(),
            api_version: VK_API_VERSION,
        };

        return &INFO;
    }

    fn find_memory_type_from_requirements(
        &self,
        device_requirements: u32,
        host_requirements: vk::MemoryPropertyFlags,
    ) -> u32 {
        let props = &self.vk.memory_properties;
        for i in 0..vk::MAX_MEMORY_TYPES {
            if (device_requirements & (1 << i)) != 0 {
                if props.memory_types[i]
                    .property_flags
                    .contains(host_requirements)
                {
                    return i as u32;
                }
            }
        }

        0
    }

    fn create_shader_module(device: &ash::Device, data: &[u32]) -> VkResult<vk::ShaderModule> {
        let module_info = vk::ShaderModuleCreateInfo::builder().code(data).build();

        unsafe { device.create_shader_module(&module_info, None) }
    }

    fn init(&mut self) {
        if self.vulkan.is_none() {
            return;
        }

        let vulkan = self.vulkan.as_ref().unwrap();
        let instance = self.instance.as_ref().unwrap();

        self.vk.gpu_properties = unsafe { instance.get_physical_device_properties(vulkan.gpu) };
        self.vk.memory_properties =
            unsafe { instance.get_physical_device_memory_properties(vulkan.gpu) };

        let mut num_images = 0;
        let mask = unsafe { vulkan.get_sync_index_mask.unwrap()(vulkan.handle) };

        for i in 0..32 {
            if (mask & (1 << i)) != 0 {
                num_images = i + 1;
            }
        }

        self.vk.num_swapchain_images = num_images;
        self.vk.swapchain_mask = mask;

        self.init_uniform_buffer();
        self.init_vertex_buffer();
        self.init_command();
        self.init_descriptor();

        self.vk.pipeline_cache = unsafe {
            self.device
                .as_ref()
                .unwrap()
                .create_pipeline_cache(&vk::PipelineCacheCreateInfo::default(), None)
                .unwrap()
        };

        self.init_render_pass(vk::Format::R8G8B8A8_UNORM);
        self.init_pipeline();
        self.init_swapchain();
    }

    fn deinit(&mut self) {
        if self.vulkan.is_none() {
            return;
        }

        let device = self.device.as_ref().unwrap();

        unsafe {
            device.device_wait_idle().unwrap();

            for i in 0..self.vk.num_swapchain_images {
                device.destroy_framebuffer(self.vk.framebuffers[i], None);
                device.destroy_image_view(self.vk.images[i].image_view, None);
                device.free_memory(self.vk.image_memory[i], None);
                device.destroy_image(self.vk.images[i].create_info.image, None);

                device.free_memory(self.vk.ubo[i].memory, None);
                device.destroy_buffer(self.vk.ubo[i].buffer, None);
            }

            if let Err(err) =
                device.free_descriptor_sets(self.vk.desc_pool, self.vk.desc_set.as_slice())
            {
                log::error!("{}", err);
            }
            device.destroy_descriptor_pool(self.vk.desc_pool, None);

            device.destroy_render_pass(self.vk.render_pass, None);
            device.destroy_pipeline(self.vk.pipeline, None);
            device.destroy_descriptor_set_layout(self.vk.set_layout, None);
            device.destroy_pipeline_layout(self.vk.pipeline_layout, None);

            device.free_memory(self.vk.vbo.memory, None);
            device.destroy_buffer(self.vk.vbo.buffer, None);
            device.destroy_pipeline_cache(self.vk.pipeline_cache, None);

            for i in 0..self.vk.num_swapchain_images {
                let commands = [self.vk.cmd[i]];
                device.free_command_buffers(self.vk.cmd_pool[i], &commands);
                device.destroy_command_pool(self.vk.cmd_pool[i], None);
            }
        }

        self.vk = Default::default();
    }

    fn create_buffer(
        &mut self,
        data: *const libc::c_void,
        size: usize,
        flags: vk::BufferUsageFlags,
    ) -> Buffer {
        let device = self.device.as_ref().unwrap();

        let info = vk::BufferCreateInfo::builder()
            .size(size as u64)
            .usage(flags)
            .build();

        let buffer = unsafe { device.create_buffer(&info, None).unwrap() };

        let mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer) };

        let memory_type_index = self.find_memory_type_from_requirements(
            mem_reqs.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let alloc = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_reqs.size)
            .memory_type_index(memory_type_index)
            .build();

        let memory = unsafe { device.allocate_memory(&alloc, None).unwrap() };
        unsafe { device.bind_buffer_memory(buffer, memory, 0).unwrap() };

        if !data.is_null() {
            unsafe {
                let ptr = device
                    .map_memory(memory, 0, size as u64, vk::MemoryMapFlags::empty())
                    .unwrap();

                let src = std::slice::from_raw_parts(data as *const u8, size);
                let dst = std::slice::from_raw_parts_mut(ptr as *mut u8, size);

                dst.clone_from_slice(src);

                device.unmap_memory(memory);
            }
        }

        Buffer { buffer, memory }
    }

    fn init_uniform_buffer(&mut self) {
        for i in 0..self.vk.num_swapchain_images {
            self.vk.ubo[i] = self.create_buffer(
                std::ptr::null(),
                16 * std::mem::size_of::<f32>(),
                vk::BufferUsageFlags::UNIFORM_BUFFER,
            );
        }
    }

    fn init_vertex_buffer(&mut self) {
        #[rustfmt::skip]
        const DATA: [f32; 24] = [
            // vec4 position, vec4 color
            -0.5, -0.5, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0,
            -0.5,  0.5, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0,
             0.5, -0.5, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0,
        ];

        self.vk.vbo = self.create_buffer(
            DATA.as_ptr() as *const _,
            DATA.len() * std::mem::size_of::<f32>(),
            vk::BufferUsageFlags::VERTEX_BUFFER,
        );
    }

    fn init_command(&mut self) {
        let device = self.device.as_ref().unwrap();

        let pool_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(self.vulkan.as_ref().unwrap().queue_index)
            .build();

        let mut info = vk::CommandBufferAllocateInfo::default();

        for i in 0..self.vk.num_swapchain_images {
            self.vk.cmd_pool[i] = unsafe { device.create_command_pool(&pool_info, None).unwrap() };

            info.command_pool = self.vk.cmd_pool[i];
            info.level = vk::CommandBufferLevel::PRIMARY;
            info.command_buffer_count = 1;

            self.vk.cmd[i] = unsafe { device.allocate_command_buffers(&info).unwrap()[0] };
        }
    }

    fn init_descriptor(&mut self) {
        let device = self.device.as_ref().unwrap();

        let bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build()];

        let pool_sizes = [vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(self.vk.num_swapchain_images as u32)
            .build()];

        let set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .build();

        self.vk.set_layout = unsafe {
            device
                .create_descriptor_set_layout(&set_layout_info, None)
                .unwrap()
        };

        let layouts = [self.vk.set_layout];

        let layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&layouts)
            .build();

        self.vk.pipeline_layout =
            unsafe { device.create_pipeline_layout(&layout_info, None).unwrap() };

        let pool_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(self.vk.num_swapchain_images as u32)
            .pool_sizes(&pool_sizes)
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .build();

        self.vk.desc_pool = unsafe { device.create_descriptor_pool(&pool_info, None).unwrap() };
        let layouts = [self.vk.set_layout];

        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.vk.desc_pool)
            .set_layouts(&layouts)
            .build();

        for i in 0..self.vk.num_swapchain_images {
            self.vk.desc_set[i] =
                unsafe { device.allocate_descriptor_sets(&alloc_info).unwrap()[0] };

            let buffer_infos = [vk::DescriptorBufferInfo::builder()
                .buffer(self.vk.ubo[i].buffer)
                .offset(0)
                .range(16 * std::mem::size_of::<f32>() as u64)
                .build()];

            let writes = [vk::WriteDescriptorSet::builder()
                .dst_set(self.vk.desc_set[i])
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos)
                .build()];

            unsafe {
                device.update_descriptor_sets(&writes, &[]);
            }
        }
    }

    fn init_render_pass(&mut self, format: vk::Format) {
        let device = self.device.as_ref().unwrap();

        let attachments = [vk::AttachmentDescription::builder()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];

        let attachment_references = [vk::AttachmentReference::builder()
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];

        let subpasses = [vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&attachment_references)
            .build()];

        let rp_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .build();

        self.vk.render_pass = unsafe { device.create_render_pass(&rp_info, None).unwrap() }
    }

    fn init_pipeline(&mut self) {
        const VERT: &[u32] = include_glsl!("examples/vulkan/shaders/triangle.vert");
        const FRAG: &[u32] = include_glsl!("examples/vulkan/shaders/triangle.frag");

        let device = self.device.as_ref().unwrap();

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .build();

        let attributes = [
            vk::VertexInputAttributeDescription::builder()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(0)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(4 * std::mem::size_of::<f32>() as u32)
                .build(),
        ];

        let bindings = [vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<f32>() as u32 * 8)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()];

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attributes)
            .build();

        let raster = vk::PipelineRasterizationStateCreateInfo::builder()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .depth_bias_enable(false)
            .line_width(1.0)
            .build();

        let blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .build()];

        let blend = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(&blend_attachments)
            .build();

        let viewport = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1)
            .build();

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .build();

        let multisample = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .build();

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

        let dynamic = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&dynamic_states)
            .build();

        let vert_mod =
            Self::create_shader_module(device, VERT).expect("Vertex shader creation to succeed");

        let frag_mod =
            Self::create_shader_module(device, FRAG).expect("Fragment shader creation to succeed");

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_mod)
                .name(c_str!("main"))
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_mod)
                .name(c_str!("main"))
                .build(),
        ];

        let pipes = [vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .rasterization_state(&raster)
            .color_blend_state(&blend)
            .multisample_state(&multisample)
            .viewport_state(&viewport)
            .depth_stencil_state(&depth_stencil)
            .dynamic_state(&dynamic)
            .render_pass(self.vk.render_pass)
            .layout(self.vk.pipeline_layout)
            .build()];

        self.vk.pipeline = unsafe {
            device
                .create_graphics_pipelines(self.vk.pipeline_cache, &pipes, None)
                .unwrap()[0]
        };

        unsafe {
            device.destroy_shader_module(shader_stages[0].module, None);
            device.destroy_shader_module(shader_stages[1].module, None);
        }
    }

    fn init_swapchain(&mut self) {
        let device = self.device.as_ref().unwrap();

        for i in 0..self.vk.num_swapchain_images {
            let image = vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::TYPE_2D)
                .flags(vk::ImageCreateFlags::default() | vk::ImageCreateFlags::MUTABLE_FORMAT)
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(
                    vk::Extent3D::builder()
                        .width(self.resolution.0 as u32)
                        .height(self.resolution.1 as u32)
                        .depth(1)
                        .build(),
                )
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT
                        | vk::ImageUsageFlags::SAMPLED
                        | vk::ImageUsageFlags::TRANSFER_SRC,
                )
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .mip_levels(1)
                .array_layers(1)
                .build();

            self.vk.images[i].create_info.image =
                unsafe { device.create_image(&image, None).unwrap() };

            let mem_reqs = unsafe {
                device.get_image_memory_requirements(self.vk.images[i].create_info.image)
            };

            let memory_type_index = self.find_memory_type_from_requirements(
                mem_reqs.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            );

            let alloc = vk::MemoryAllocateInfo::builder()
                .allocation_size(mem_reqs.size)
                .memory_type_index(memory_type_index)
                .build();

            self.vk.image_memory[i] = unsafe { device.allocate_memory(&alloc, None).unwrap() };

            unsafe {
                device
                    .bind_image_memory(
                        self.vk.images[i].create_info.image,
                        self.vk.image_memory[i],
                        0,
                    )
                    .unwrap();
            }

            let info = &mut self.vk.images[i].create_info;
            info.s_type = vk::StructureType::IMAGE_VIEW_CREATE_INFO;
            info.view_type = vk::ImageViewType::TYPE_2D;
            info.format = vk::Format::R8G8B8A8_UNORM;
            info.subresource_range.base_mip_level = 0;
            info.subresource_range.base_array_layer = 0;
            info.subresource_range.level_count = 1;
            info.subresource_range.layer_count = 1;
            info.subresource_range.aspect_mask = vk::ImageAspectFlags::COLOR;
            info.components.r = vk::ComponentSwizzle::R;
            info.components.g = vk::ComponentSwizzle::G;
            info.components.b = vk::ComponentSwizzle::B;
            info.components.a = vk::ComponentSwizzle::A;

            self.vk.images[i].image_view = unsafe {
                device
                    .create_image_view(&self.vk.images[i].create_info, None)
                    .unwrap()
            };

            self.vk.images[i].image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;

            let attachments = [self.vk.images[i].image_view];

            let fb_info = vk::FramebufferCreateInfo::builder()
                .render_pass(self.vk.render_pass)
                .attachments(&attachments)
                .width(self.resolution.0 as u32)
                .height(self.resolution.1 as u32)
                .layers(1)
                .build();

            self.vk.framebuffers[i] = unsafe { device.create_framebuffer(&fb_info, None).unwrap() };
        }
    }

    fn update_ubo(&mut self) {
        let device = self.device.as_ref().unwrap();

        let c = (self.frame as f32 * 0.01).cos();
        let s = (self.frame as f32 * 0.01).sin();
        self.frame = self.frame.wrapping_add(1);

        let mut data = [0.0; 16];
        data[0] = c;
        data[1] = s;
        data[4] = -s;
        data[5] = c;
        data[10] = 1.0;
        data[15] = 1.0;

        unsafe {
            let size = 16 * std::mem::size_of::<f32>();
            let ptr = device
                .map_memory(
                    self.vk.ubo[self.vk.index].memory,
                    0,
                    size as u64,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();

            let src = std::slice::from_raw_parts(&data as *const _ as *const u8, size);
            let dst = std::slice::from_raw_parts_mut(ptr as *mut u8, size);

            dst.clone_from_slice(src);

            device.unmap_memory(self.vk.ubo[self.vk.index].memory);
        }
    }

    fn render(&mut self) {
        self.update_ubo();

        let device = self.device.as_ref().unwrap();
        let cmd = self.vk.cmd[self.vk.index];

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();

        unsafe {
            device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::default())
                .unwrap();
            device.begin_command_buffer(cmd, &begin_info).unwrap();
        }

        let prepare_renderings = [vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::NONE)
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::COLOR_ATTACHMENT_READ,
            )
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.vk.images[self.vk.index].create_info.image)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1)
                    .build(),
            )
            .build()];

        unsafe {
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &prepare_renderings,
            );
        }

        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.8, 0.6, 0.2, 1.0],
            },
        }];

        let rp_begin = vk::RenderPassBeginInfo::builder()
            .render_pass(self.vk.render_pass)
            .framebuffer(self.vk.framebuffers[self.vk.index])
            .render_area(
                vk::Rect2D::builder()
                    .extent(
                        vk::Extent2D::builder()
                            .width(self.resolution.0 as u32)
                            .height(self.resolution.1 as u32)
                            .build(),
                    )
                    .build(),
            )
            .clear_values(&clear_values)
            .build();

        let desc_sets = [self.vk.desc_set[self.vk.index]];

        unsafe {
            device.cmd_begin_render_pass(cmd, &rp_begin, vk::SubpassContents::INLINE);
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.vk.pipeline);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.vk.pipeline_layout,
                0,
                &desc_sets,
                &[],
            );
        }

        let view_ports = [vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(self.resolution.0 as f32)
            .height(self.resolution.1 as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];

        unsafe {
            device.cmd_set_viewport(cmd, 0, &view_ports);
        }

        let scissors = [vk::Rect2D::builder()
            .extent(
                vk::Extent2D::builder()
                    .width(self.resolution.0 as u32)
                    .height(self.resolution.1 as u32)
                    .build(),
            )
            .build()];

        let vertex_buffers = [self.vk.vbo.buffer];
        let vbo_offsets = [0];

        unsafe {
            device.cmd_set_scissor(cmd, 0, &scissors);

            device.cmd_bind_vertex_buffers(cmd, 0, &vertex_buffers, &vbo_offsets);

            device.cmd_draw(cmd, 3, 1, 0, 0);

            device.cmd_end_render_pass(cmd);
        }

        let prepare_presentations = [vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.vk.images[self.vk.index].create_info.image)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1)
                    .build(),
            )
            .build()];

        unsafe {
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::ALL_GRAPHICS,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &prepare_presentations,
            );

            device.end_command_buffer(cmd).unwrap();
        }
    }
}

impl Core for TestCore {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("TestCore Vulkan").unwrap(),
            library_version: CString::new(CRATE_VERSION.to_string()).unwrap(),
            valid_extensions: CString::new("").unwrap(),

            need_fullpath: false,
            block_extract: false,
        }
    }

    fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info {
        self.get_av_info()
    }

    fn on_set_environment(&mut self, initial: bool, ctx: &mut SetEnvironmentContext) {
        if !initial {
            return;
        }

        ctx.set_support_no_game(true);
    }

    fn on_options_changed(&mut self, ctx: &mut OptionsChangedContext) {
        match ctx.get_variable("testvulkan_resolution") {
            Some(value) => {
                let dimensions = value
                    .split('x')
                    .map(|x| x.parse::<u16>().unwrap())
                    .collect::<Vec<_>>();

                let resolution = (dimensions[0], dimensions[1]);
                let reinitialize = resolution != self.resolution;
                self.resolution = resolution;

                if reinitialize {
                    self.deinit();
                    self.init();
                }
            }
            _ => (),
        }
    }

    fn on_load_game(
        &mut self,
        _game: Option<retro_game_info>,
        ctx: &mut LoadGameContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let enabled = ctx.enable_hw_render(
                retro_hw_context_type::RETRO_HW_CONTEXT_VULKAN,
                false,
                VK_API_VERSION,
                0,
                false,
            );

            if !enabled {
                return Err("Failed to enable Vulkan context".into());
            }

            let success = ctx.enable_hw_render_negotiation_interface_vulkan(
                Some(Self::get_application_info),
                None,
                None,
            );

            if !success {
                log::warn!("Failed to set hardware context negotiation interface");
            }
        }

        Ok(())
    }

    fn on_hw_context_reset(&mut self, ctx: &mut GenericContext) {
        log::info!("on_hw_context_reset");

        self.vulkan.take();
        let iface = unsafe { ctx.get_hw_render_interface_vulkan() };

        if let Some(iface) = iface {
            if iface.interface_type
                != retro_hw_render_interface_type::RETRO_HW_RENDER_INTERFACE_VULKAN
            {
                log::error!(
                    "Unexpected hardware interface type: {:?}",
                    iface.interface_type as u32
                );
                return;
            }

            if iface.interface_version != RETRO_HW_RENDER_INTERFACE_VULKAN_VERSION {
                log::error!(
                    "Unexpected hardware interface version {}, got version {}",
                    RETRO_HW_RENDER_INTERFACE_VULKAN_VERSION,
                    iface.interface_version
                );
                return;
            }

            if iface.get_instance_proc_addr.is_none() {
                log::error!("Invalid function pointer to \"get_instance_proc_addr\"");
                return;
            }

            let static_fn = vk::StaticFn {
                get_instance_proc_addr: iface.get_instance_proc_addr.unwrap(),
            };

            let instance = unsafe { ash::Instance::load(&static_fn, iface.instance) };
            let device = unsafe { ash::Device::load(&instance.fp_v1_0(), iface.device) };
            let entry = unsafe { ash::Entry::from_static_fn(static_fn) };

            self.instance.replace(instance);
            self.device.replace(device);
            self.entry.replace(entry);
            self.vulkan.replace(iface);

            self.init();
        }
    }

    fn on_hw_context_destroyed(&mut self, _ctx: &mut GenericContext) {
        log::info!("on_hw_context_destroyed");

        self.deinit();
        self.vulkan.take();
        self.vk = Default::default();
    }

    fn on_run(&mut self, ctx: &mut RunContext, _delta_us: Option<i64>) {
        if let Some(vulkan) = self.vulkan.take() {
            let handle = vulkan.handle;

            unsafe {
                if vulkan.get_sync_index_mask.unwrap()(handle) != self.vk.swapchain_mask {
                    self.deinit();
                    self.init();
                }

                vulkan.wait_sync_index.unwrap()(handle);

                self.vk.index = vulkan.get_sync_index.unwrap()(handle) as usize;
            }

            self.render();

            unsafe {
                vulkan.set_image.unwrap()(
                    handle,
                    &self.vk.images[self.vk.index],
                    0,
                    null(),
                    vk::QUEUE_FAMILY_IGNORED,
                );

                vulkan.set_command_buffers.unwrap()(
                    handle,
                    1,
                    &self.vk.cmd[self.vk.index] as *const _,
                );
            }

            ctx.draw_hardware_frame(self.resolution.0 as u32, self.resolution.1 as u32, 0);

            self.vulkan.replace(vulkan);
        }
    }
}
