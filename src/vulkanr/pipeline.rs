use super::data::*;
use super::descriptor::*;
use super::device::*;
use super::swapchain::*;
use super::vulkan::*;
use crate::vulkanr::render::RRRender;
use std::fs::File;
use std::io::Read;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::vk::PrimitiveTopology;

#[derive(Clone, Debug, Default)]
pub struct RRPipeline {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

impl RRPipeline {
    pub unsafe fn new(
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrrender: &RRRender,
        rrdescriptor_set: &RRDescriptorSet,
        vertex_shader_path: &str,
        fragment_shader_path: &str,
        topology: PrimitiveTopology,
        polygon_mode: vk::PolygonMode,
    ) -> Self {
        let mut rrpipeline = RRPipeline::default();
        let _ = create_pipeline(
            rrdevice,
            rrswapchain,
            rrrender,
            rrdescriptor_set,
            &mut rrpipeline,
            vertex_shader_path,
            fragment_shader_path,
            topology,
            polygon_mode,
        );
        println!("rrpipeline: {:?}", rrpipeline);
        rrpipeline
    }

    /// Create ImGui rendering pipeline with push constants and alpha blending
    pub unsafe fn new_imgui(
        rrdevice: &RRDevice,
        rrrender: &RRRender,
        descriptor_set_layout: vk::DescriptorSetLayout,
        vertex_shader_path: &str,
        fragment_shader_path: &str,
        msaa_samples: vk::SampleCountFlags,
    ) -> Result<Self> {
        let mut rrpipeline = RRPipeline::default();
        create_imgui_pipeline(
            rrdevice,
            rrrender,
            descriptor_set_layout,
            &mut rrpipeline,
            vertex_shader_path,
            fragment_shader_path,
            msaa_samples,
        )?;
        println!("ImGui pipeline created: {:?}", rrpipeline);
        Ok(rrpipeline)
    }
}
unsafe fn create_pipeline(
    rrdevice: &RRDevice,
    rrswapchain: &RRSwapchain,
    rrrender: &RRRender,
    rrdescriptor_set: &RRDescriptorSet,
    rrpipeline: &mut RRPipeline,
    vertex_shader_path: &str,
    fragment_shader_path: &str,
    topology: vk::PrimitiveTopology,
    polygon_mode: vk::PolygonMode,
) -> Result<()> {
    println!("start create pipeline");
    let mut vert_file = File::open(vertex_shader_path)?;
    let mut frag_file = File::open(fragment_shader_path)?;
    println!("vert_file: {:?}", vert_file);
    println!("frag_file: {:?}", frag_file);
    let mut vert = Vec::new();
    let mut frag = Vec::new();
    vert_file.read_to_end(&mut vert)?;
    frag_file.read_to_end(&mut frag)?;
    let vert_shader_module = create_shader_module(rrdevice, &vert[..])?;
    let frag_shader_module = create_shader_module(rrdevice, &frag[..])?;
    println!("vertex shader module: {:?}", vert_shader_module);
    println!("frag shader module: {:?}", frag_shader_module);

    let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vert_shader_module)
        .name(b"main\0");
    let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(frag_shader_module)
        .name(b"main\0");

    let binding_discriptions = &[Vertex::binding_description()];
    let attribute_descriptions = Vertex::attribute_descriptions();
    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(binding_discriptions)
        .vertex_attribute_descriptions(&attribute_descriptions);

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(topology)
        .primitive_restart_enable(false);

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(rrswapchain.swapchain_extent.width as f32)
        .height(rrswapchain.swapchain_extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);

    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(rrswapchain.swapchain_extent);

    let viewports = &[viewport];
    let scissors = &[scissor];
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(viewports)
        .scissors(scissors);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(polygon_mode)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(true) // https://registry.khronos.org/vulkan/specs/1.0/html/vkspec.html#primsrast-sampleshading
        .min_sample_shading(0.9) //  Minimum fraction for sample shading; closer to one is smoother.
        .sample_shading_enable(false)
        .rasterization_samples(rrdevice.msaa_samples);

    let attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::all())
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD);

    let attachments = &[attachment];
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(attachments)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

    // NOTE: This will cause the configuration of these values to be ignored and you will be required to specify the data at drawing time.
    let dynamic_state = &[vk::DynamicState::VIEWPORT, vk::DynamicState::LINE_WIDTH];

    let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(dynamic_state);

    let descriptor_set_layouts = &[rrdescriptor_set.descriptor_set_layout];
    let layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(descriptor_set_layouts);
    rrpipeline.pipeline_layout = rrdevice.device.create_pipeline_layout(&layout_info, None)?;

    let stages = &[vert_stage, frag_stage];

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS)
        .depth_bounds_test_enable(false)
        .min_depth_bounds(0.0)
        .max_depth_bounds(1.0)
        .stencil_test_enable(false);

    let info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .layout(rrpipeline.pipeline_layout)
        .render_pass(rrrender.render_pass)
        .subpass(0);

    rrpipeline.pipeline = rrdevice
        .device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)?
        .0[0];

    rrdevice
        .device
        .destroy_shader_module(vert_shader_module, None);
    rrdevice
        .device
        .destroy_shader_module(frag_shader_module, None);
    Ok(())
}

unsafe fn create_imgui_pipeline(
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    descriptor_set_layout: vk::DescriptorSetLayout,
    rrpipeline: &mut RRPipeline,
    vertex_shader_path: &str,
    fragment_shader_path: &str,
    msaa_samples: vk::SampleCountFlags,
) -> Result<()> {
    println!("Creating ImGui pipeline");

    // Load shaders
    let mut vert_file = File::open(vertex_shader_path)?;
    let mut frag_file = File::open(fragment_shader_path)?;
    let mut vert = Vec::new();
    let mut frag = Vec::new();
    vert_file.read_to_end(&mut vert)?;
    frag_file.read_to_end(&mut frag)?;
    let vert_shader_module = create_shader_module(rrdevice, &vert[..])?;
    let frag_shader_module = create_shader_module(rrdevice, &frag[..])?;

    // Shader stages
    let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vert_shader_module)
        .name(b"main\0");
    let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(frag_shader_module)
        .name(b"main\0");
    let shader_stages = [vert_stage, frag_stage];

    // Vertex input state for ImGui DrawVert
    let vertex_binding_descriptions = [vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(std::mem::size_of::<imgui::DrawVert>() as u32)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build()];

    let vertex_attribute_descriptions = [
        // Position at offset 0
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0)
            .build(),
        // UV at offset 8
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(8)
            .build(),
        // Color at offset 16
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R8G8B8A8_UNORM)
            .offset(16)
            .build(),
    ];

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&vertex_binding_descriptions)
        .vertex_attribute_descriptions(&vertex_attribute_descriptions);

    // Input assembly
    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // Viewport state (dynamic)
    let viewports = [vk::Viewport::default()];
    let scissors = [vk::Rect2D::default()];
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&viewports)
        .scissors(&scissors);

    // Rasterization state
    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    // Multisample state - must match render pass MSAA settings
    let msaa_samples = if !msaa_samples.is_empty() {
        msaa_samples
    } else {
        vk::SampleCountFlags::_8
    };
    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(msaa_samples);

    // Depth/stencil state - ImGui doesn't use depth testing
    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(false)
        .depth_write_enable(false)
        .depth_compare_op(vk::CompareOp::ALWAYS)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false);

    // Color blend state with alpha blending
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::all())
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(vk::BlendOp::ADD)
        .build();

    let color_blend_attachments = [color_blend_attachment];
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(&color_blend_attachments);

    // Dynamic state for viewport and scissor
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(&dynamic_states);

    // Pipeline layout with push constants
    let push_constant_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .offset(0)
        .size(std::mem::size_of::<[f32; 4]>() as u32)
        .build();

    let set_layouts = [descriptor_set_layout];
    let push_constant_ranges = [push_constant_range];
    let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constant_ranges);

    rrpipeline.pipeline_layout = rrdevice.device.create_pipeline_layout(&pipeline_layout_info, None)?;

    // Create graphics pipeline
    let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .dynamic_state(&dynamic_state)
        .layout(rrpipeline.pipeline_layout)
        .render_pass(rrrender.render_pass)
        .subpass(0);

    let pipelines = rrdevice
        .device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info.build()], None)
        .map_err(|e| anyhow::anyhow!("Failed to create ImGui pipeline: {:?}", e))?;

    rrpipeline.pipeline = pipelines.0[0];

    // Clean up shader modules
    rrdevice.device.destroy_shader_module(vert_shader_module, None);
    rrdevice.device.destroy_shader_module(frag_shader_module, None);

    Ok(())
}

unsafe fn create_shader_module(rrdevice: &RRDevice, bytecode: &[u8]) -> Result<vk::ShaderModule> {
    let bytecode = Bytecode::new(bytecode).unwrap();
    let info = vk::ShaderModuleCreateInfo::builder()
        .code_size(bytecode.code_size())
        .code(bytecode.code());

    Ok(rrdevice.device.create_shader_module(&info, None)?)
}
