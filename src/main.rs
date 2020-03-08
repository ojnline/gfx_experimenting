use rendy::{
    command::{Families, QueueId, RenderPassEncoder},
    factory::{Config, Factory, ImageState},
    graph::{
        present::PresentNode, render::*, Graph, GraphBuilder, GraphContext, NodeBuffer, NodeImage,
    },
    hal::{self, device::Device as _},
    init::winit::{
        self,
        event_loop::{ControlFlow, EventLoop},
        event::{Event, WindowEvent},
        window::WindowBuilder
    },
    init::AnyWindowedRendy,
    memory::Dynamic,
    mesh::{Mesh, Position, Model, AsVertex},
    resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
    shader::{ShaderSet, ShaderKind, ShaderSetBuilder, SourceLanguage, SourceShaderInfo, SpirvShader},
    texture::{image::ImageTextureConfig, Texture},
};
use genmesh::generators::{IndexedPolygon, SharedVertex};
use nalgebra::*;
use std::fs::read_to_string;

type Vec3 = nalgebra::Vector3<f32>;

lazy_static::lazy_static! {
    static ref vert_src: String = read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/src/assets/mesh.vert")).expect("Couldn't open shader file.");
    static ref frag_src: String = read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/src/assets/mesh.frag")).expect("Couldn't open shader file.");
}

struct Aux<B: hal::Backend>  {
    mesh: Option<rendy::mesh::Mesh<B>>,
}

#[derive(Debug, Default)]
struct PipelineDesc;

#[derive(Debug)]
struct Pipeline {
    //sets: Vec<Escape<DescriptorSet<B>>>,
}

impl<B> SimpleGraphicsPipelineDesc<B, Aux<B>> for PipelineDesc
where
    B: hal::Backend
{
    type Pipeline = Pipeline;

    fn load_shader_set(&self, factory: &mut Factory<B>, aux: &Aux<B>) -> ShaderSet<B> {

        let vertex: SpirvShader = 
        SourceShaderInfo::new(
            vert_src.as_str(),
            concat!(env!("CARGO_MANIFEST_DIR"), "assets/mesh.vert").into(),
            ShaderKind::Vertex,
            SourceLanguage::GLSL,
            "main",
        ).precompile().unwrap();

        let fragment: SpirvShader = 
        SourceShaderInfo::new(
            frag_src.as_str(),
            concat!(env!("CARGO_MANIFEST_DIR"), "assets/mesh.frag").into(),
            ShaderKind::Fragment,
            SourceLanguage::GLSL,
            "main",
        ).precompile().unwrap();

        ShaderSetBuilder::default()
            .with_vertex(&vertex).unwrap()
            .with_fragment(&fragment).unwrap()
            .build(factory, Default::default()).unwrap()
    }

    fn vertices(
        &self,
    ) -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )> {
        vec![
            Position::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex),
        ]
    }

    fn build(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        aux: &Aux<B>,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>]
    ) -> Result<Self::Pipeline, hal::pso::CreationError> {
        
        //let frames = ctx.frames_in_flight as _;

        
        Ok(Self::Pipeline{})
    }
}

impl<B> SimpleGraphicsPipeline<B, Aux<B>> for Pipeline
where
    B: hal::Backend
{
    type Desc = PipelineDesc;

    fn draw(
        &mut self,
        layout: &<B as hal::Backend>::PipelineLayout,
        mut encoder: RenderPassEncoder<B>,
        index: usize,
        aux: &Aux<B>
    ) {
        if let Some(ref mesh) = aux.mesh {
            mesh.bind_and_draw(0, &[Position::vertex()], 0..1, &mut encoder).unwrap();
        }
    }
    
    fn dispose(self, factory: &mut Factory<B>, aux: &Aux<B>) {
        
    }
}

fn run<B: hal::Backend>(
    event_loop: EventLoop<()>,
    window: winit::window::Window,
    graph: Graph<B, Aux<B>>,
    mut factory: Factory<B>,
    mut families: Families<B>,
    mut aux: Aux<B>
) {
    let mut graph = Some(graph);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {event, ..} => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(_size) => {},
                    _ => {}
                }
                
            }
            Event::MainEventsCleared => {
                window.request_redraw();
                factory.maintain(&mut families);
            },
            Event::RedrawRequested(_) => {
                if let Some(ref mut graph) = graph {
                    graph.run(&mut factory, &mut families, &aux);
                }
            }
            _ => {},
        }

        if *control_flow == ControlFlow::Exit && graph.is_some() {
            graph.take().unwrap().dispose(&mut factory, &aux);
            drop(aux.mesh.take());
        }
    }
    )
}

fn main() {

    let event_loop = EventLoop::new();
    
    let window = WindowBuilder::new()
        .with_title("Hello, triangle!");
    
    let config: Config = Default::default();
    
    let rendy = rendy::init::AnyWindowedRendy::init_auto(&config, window, &event_loop).unwrap();

    rendy::with_any_windowed_rendy!((rendy)
        use back; (mut factory, mut families, surface, window) => {

        let mut aux = Aux{mesh: None};
        let size = window.inner_size();
        let window_kind = hal::image::Kind::D2(size.width as u32, size.height as u32, 1, 1);

        let mut graph_builder = GraphBuilder::<_, Aux<_>>::new();

        let depth = graph_builder.create_image(
            window_kind,
            1,
            hal::format::Format::D32Sfloat,
            Some(hal::command::ClearValue {
                depth_stencil: hal::command::ClearDepthStencil {
                    depth: 1.0,
                    stencil: 0,
                },
            }),
        );

        let pass = graph_builder.add_node(
            Pipeline::builder()
                .into_subpass()
                .with_color_surface()
                .with_depth_stencil(depth)
                .into_pass()
                .with_surface(
                    surface,
                    hal::window::Extent2D {
                        width: size.width,
                        height: size.height,
                    },
                    Some(hal::command::ClearValue {
                        color: hal::command::ClearColor {
                            float32: [0.07, 0.2, 0.33, 1.0],
                        },
                }),
            )
        );

        let graph = graph_builder
            .build(&mut factory, &mut families, &aux)
            .unwrap();  

        aux = {
            let icosphere = genmesh::generators::IcoSphere::subdivide(4);
            let indices: Vec<_> = genmesh::Vertices::vertices(icosphere.indexed_polygon_iter())
                .map(|i| i as u32)
                .collect();
            let vertices: Vec<_> = icosphere
                .shared_vertex_iter()
                .map(|v| Position(
                    v.pos.into()
                ))
                .collect();
            let mesh = Mesh::<back::Backend>::builder()
            .with_indices(&indices[..])
            .with_vertices(&vertices[..])
            .build(graph.node_queue(pass), &factory)
            .unwrap();

            Aux {
                mesh: Some(mesh)
            }
        };

        // no autocompletion in macros so this is what you get
        run(event_loop, window, graph, factory, families, aux);
    })
}