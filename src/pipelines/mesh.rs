use rendy::{
    command::{Families, QueueId, RenderPassEncoder},
    factory::{Config, Factory, ImageState},
    graph::{
        present::PresentNode, render::*, Graph, GraphBuilder, GraphContext, NodeBuffer, NodeImage,
    },
    hal::{self, adapter::PhysicalDevice, pso::ShaderStageFlags},
    init::winit::{
        self,
        dpi::{PhysicalSize, Size},
        event::{DeviceEvent, Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    },
    init::AnyWindowedRendy,
    memory::Dynamic,
    mesh::{AsVertex, Mesh, Position},
    resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
    shader::{
        ShaderKind, ShaderSet, ShaderSetBuilder, SourceLanguage, SourceShaderInfo, SpirvReflection,
        SpirvShader,
    },
    texture::{image::ImageTextureConfig, Texture},
};

use std::{
    fs::read_to_string,
};

use super::*;

lazy_static::lazy_static! {
    static ref VERT_SRC: String = read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/assets/mesh.vert")).expect("Couldn't open shader file.");
    static ref FRAG_SRC: String = read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/assets/mesh.frag")).expect("Couldn't open shader file.");

    static ref VERTEX: SpirvShader = {
        SourceShaderInfo::new(
            VERT_SRC.as_str(),
            "mesh.vert",
            ShaderKind::Vertex,
            SourceLanguage::GLSL,
            "main",
        ).precompile().unwrap()
    };
    static ref FRAGMENT: SpirvShader = {
        SourceShaderInfo::new(
            FRAG_SRC.as_str(),
            "mesh.frag",
            ShaderKind::Fragment,
            SourceLanguage::GLSL,
            "main",
        ).precompile().unwrap()
    };
    static ref SHADERS: rendy::shader::ShaderSetBuilder = rendy::shader::ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();

    static ref SHADER_REFLECTION: SpirvReflection = SHADERS.reflect().unwrap();
}

#[derive(Debug, Default)]
pub struct PipelineDesc;

#[derive(Debug)]
pub struct Pipeline;

impl<B> SimpleGraphicsPipelineDesc<B, Aux<B>> for PipelineDesc
where
    B: hal::Backend,
{
    type Pipeline = Pipeline;

    fn load_shader_set(&self, factory: &mut Factory<B>, _aux: &Aux<B>) -> ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }
    
    fn vertices(
        &self,
    ) -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )> {
        vec![SHADER_REFLECTION
        .attributes(&["position"])
        .unwrap()
        .gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)]
    }
    
    fn layout(&self) -> Layout {
        SHADER_REFLECTION.layout().unwrap()
    }
    
    fn build(
        self,
        _ctx: &GraphContext<B>,
        _factory: &mut Factory<B>,
        _queue: QueueId,
        _aux: &Aux<B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<Self::Pipeline, hal::pso::CreationError> {        
        Ok(Pipeline{})
    }
}

impl<B> SimpleGraphicsPipeline<B, Aux<B>> for Pipeline
where
    B: hal::Backend,
{
    type Desc = PipelineDesc;

    fn draw(
        &mut self,
        layout: &<B as hal::Backend>::PipelineLayout,
        mut encoder: RenderPassEncoder<B>,
        _index: usize,
        aux: &Aux<B>,
    ) {
        if let Some(ref mesh) = aux.mesh {
            unsafe {
                let data = std::slice::from_raw_parts(
                    aux.camera.get_transform().as_ptr() as *const u32,
                    16,
                );
                encoder.push_constants(layout, ShaderStageFlags::VERTEX, 0, data);
            }
            let vertex = [SHADER_REFLECTION.attributes(&["position"]).unwrap()];
            mesh.bind_and_draw(0, &vertex, 0..1, &mut encoder).unwrap();
        }
    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &Aux<B>) {}
}

