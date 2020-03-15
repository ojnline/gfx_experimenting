use rendy::{
    command::{QueueId, RenderPassEncoder},
    factory::Factory,
    graph::{render::*, GraphContext, ImageAccess, NodeBuffer, NodeImage},
    hal::{self, device::Device, pso::DescriptorPool, pso::ShaderStageFlags},
    resource::{
        Buffer, BufferInfo, DescriptorSetLayout, Escape, Filter, Handle, ImageView, ImageViewInfo,
        Sampler, SamplerDesc, ViewKind, WrapMode,
    },
    shader::{PathBufShaderInfo, ShaderKind, ShaderSet, SourceLanguage, SpirvShader, SourceShaderInfo, SpirvReflection},
};

use std::{
    fs::read_to_string,
};

use super::*;

lazy_static::lazy_static! {
    static ref VERT_SRC: String = read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/assets/fullscreen_triangle.vert")).expect("Couldn't open shader file.");
    static ref FRAG_SRC: String = read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/assets/posteffect.frag")).expect("Couldn't open shader file.");

    static ref VERTEX: SpirvShader = {
        SourceShaderInfo::new(
            VERT_SRC.as_str(),
            "fullscreen_triangle.vert",
            ShaderKind::Vertex,
            SourceLanguage::GLSL,
            "main",
        ).precompile().unwrap()
    };
    static ref FRAGMENT: SpirvShader = {
        SourceShaderInfo::new(
            FRAG_SRC.as_str(),
            "posteffect.frag",
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
pub struct Pipeline<B: hal::Backend> {
    sets: Vec<B::DescriptorSet>,
    descriptor_pool: B::DescriptorPool,
    image_sampler: Escape<Sampler<B>>,
    image_view: Escape<ImageView<B>>
}
impl<B> SimpleGraphicsPipelineDesc<B, Aux<B>> for PipelineDesc
where
    B: hal::Backend,
{
    type Pipeline = Pipeline<B>;

    fn images(&self) -> Vec<ImageAccess> {
        vec![ImageAccess {
            access: hal::image::Access::SHADER_READ,
            usage: hal::image::Usage::SAMPLED,
            layout: hal::image::Layout::ShaderReadOnlyOptimal,
            stages: hal::pso::PipelineStage::FRAGMENT_SHADER,
        }]
    }

    fn depth_stencil(&self) -> Option<hal::pso::DepthStencilDesc> {
        None
    }

    fn load_shader_set(&self, factory: &mut Factory<B>, _aux: &Aux<B>) -> ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }

    fn layout(&self) -> Layout {
        Layout {
            sets: vec![SetLayout {
                bindings: vec![
                    hal::pso::DescriptorSetLayoutBinding {
                        binding: 0,
                        ty: hal::pso::DescriptorType::Sampler,
                        count: 1,
                        stage_flags: hal::pso::ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                    hal::pso::DescriptorSetLayoutBinding {
                        binding: 1,
                        ty: hal::pso::DescriptorType::SampledImage,
                        count: 1,
                        stage_flags: hal::pso::ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    }
                ],
            }],
            push_constants: Vec::new(),
        }
    }

    fn build(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _aux: &Aux<B>,
        _buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<Self::Pipeline, hal::pso::CreationError> {
        
        let frames = ctx.frames_in_flight as usize;

        let mut descriptor_pool = unsafe {
            factory.create_descriptor_pool(
                frames,
                vec![
                    hal::pso::DescriptorRangeDesc {
                        ty: hal::pso::DescriptorType::Sampler,
                        count: frames,
                    },
                    hal::pso::DescriptorRangeDesc {
                        ty: hal::pso::DescriptorType::SampledImage,
                        count: frames,
                    },
                ],
                hal::pso::DescriptorPoolCreateFlags::empty(),
            )?
        };
        
        let image_sampler = factory
            .create_sampler(SamplerDesc::new(Filter::Nearest, WrapMode::Clamp))
            .unwrap();
            
        let image_handle = ctx
            .get_image(images[0].id)
            .expect("No input image supplied.");

        let image_view = factory
            .create_image_view(
                image_handle.clone(),
                ImageViewInfo {
                    view_kind: ViewKind::D2,
                    format: hal::format::Format::Rgba32Sfloat,
                    swizzle: hal::format::Swizzle::NO,
                    range: images[0].range.clone(),
                },
            )
            .expect("Could not create image view");

        let mut sets = Vec::with_capacity(frames);
        for _ in 0..frames {
            unsafe {
                let set = descriptor_pool.allocate_set(&set_layouts[0].raw()).unwrap();
                factory.write_descriptor_sets(vec![
                    hal::pso::DescriptorSetWrite {
                        set: &set,
                        binding: 0,
                        array_offset: 0,
                        descriptors: Some(hal::pso::Descriptor::Sampler(image_sampler.raw())),
                    },
                    hal::pso::DescriptorSetWrite {
                        set: &set,
                        binding: 1,
                        array_offset: 0,
                        descriptors: Some(hal::pso::Descriptor::Image(
                            image_view.raw(),
                            hal::image::Layout::ShaderReadOnlyOptimal,
                        )),
                    }
                ]);
                sets.push(set);
            }
        }

        Ok( Pipeline {
            sets,
            image_view,
            image_sampler,
            descriptor_pool,
        })
    }
}

impl<B> SimpleGraphicsPipeline<B, Aux<B>> for Pipeline<B>
where
    B: hal::Backend,
{
    type Desc = PipelineDesc;

    fn draw(
        &mut self,
        layout: &<B as hal::Backend>::PipelineLayout,
        mut encoder: RenderPassEncoder<B>,
        index: usize,
        aux: &Aux<B>,
    ) {
        unsafe {
            encoder.bind_graphics_descriptor_sets(
                layout,
                0,
                Some(&self.sets[index]),
                std::iter::empty(),
            );
            encoder.draw(0..3, 0..1);
        }
    }

    fn dispose(mut self, factory: &mut Factory<B>, aux: &Aux<B>) {
        unsafe {
            self.descriptor_pool.reset();
            factory.destroy_descriptor_pool(self.descriptor_pool);
        }
    }
}

