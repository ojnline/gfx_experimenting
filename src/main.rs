#![allow(warnings)]


pub mod pipelines;

use pipelines::*;
use genmesh::generators::{IndexedPolygon, SharedVertex};
use nalgebra::*;
use rendy::{
    command::{Families, QueueId, RenderPassEncoder},
    factory::{Config, Factory, ImageState},
    graph::{
        present::PresentNode, render::*, Graph, GraphBuilder, GraphContext, NodeBuffer, NodeImage,
    },
    hal::{self, adapter::PhysicalDevice, pso::ShaderStageFlags},
    init::winit::{
        self,
        dpi::{Size,PhysicalSize},
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
use std::{fs::read_to_string, time::Instant};

enum Direction {
    Left = 0,
    Right = 1,
    Up = 2,
    Down = 3,
    Forward = 4,
    Backward = 5,
}

#[derive(Clone, Copy)]
pub struct Camera {
    pitch: f32,
    yaw: f32,
    pos: Point3<f32>,
    aspect: f32,
    fov: f32,
    near: f32,
    far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pitch: 0.,
            yaw: 0.,
            pos: Point3::new(0., 0., 0.),
            aspect: 1.,
            fov: 45.,
            near: 0.1,
            far: 1.,
        }
    }
}

impl Camera {
    pub fn get_view_direction(&self) -> Vector3<f32> {
        Vector3::new(
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.sin(),
        )
    }
    pub fn get_view(&self) -> Matrix4<f32> {
        Matrix4::look_at_lh(
            &self.pos,
            &Point3::from(self.get_view_direction()+self.pos.coords),
            &Vector3::new(0., 0., 1.),
        )
    }
    pub fn get_projection(&self) -> Matrix4<f32> {
        Matrix4::new_perspective(self.aspect, self.fov, self.near, self.far)
    }
    pub fn get_transform(&self) -> Matrix4<f32> {
        self.get_projection() * self.get_view()
    }
}

pub struct Aux<B: hal::Backend> {
    pub mesh: Option<rendy::mesh::Mesh<B>>,
    pub camera: Camera,
    pub size: [u32; 2],
    pub keys: [bool; 6],
    pub last_update: Instant,
}

fn run<B: hal::Backend>(
    event_loop: EventLoop<()>,
    window: winit::window::Window,
    graph: Graph<B, Aux<B>>,
    mut factory: Factory<B>,
    mut families: Families<B>,
    mut aux: Aux<B>,
) {
    let mut graph = Some(graph);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_size) => {}
                WindowEvent::KeyboardInput { input, .. } => {
                    use rendy::init::winit::event::VirtualKeyCode::*;
                    let pressed = input.state == winit::event::ElementState::Pressed;
                    match input.virtual_keycode {
                        Some(W) => aux.keys[Direction::Forward as usize] = pressed,
                        Some(S) => aux.keys[Direction::Backward as usize] = pressed,
                        Some(A) => aux.keys[Direction::Left as usize] = pressed,
                        Some(D) => aux.keys[Direction::Right as usize] = pressed,
                        Some(Space) => aux.keys[Direction::Up as usize] = pressed,
                        Some(LShift) => aux.keys[Direction::Down as usize] = pressed,
                        _ => {}
                    }
                }
                _ => {}
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    aux.camera.yaw += (delta.0 * 0.005) as f32;
                    aux.camera.pitch -= (delta.1 * 0.005) as f32;
                    aux.camera.pitch = aux.camera.pitch.min(1.57).max(-1.57);
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                window.request_redraw();
                factory.maintain(&mut families);
            }
            Event::RedrawRequested(_) => {
                if let Some(ref mut graph) = graph {
                    graph.run(&mut factory, &mut families, &aux);
                }

                let delta = aux.last_update.elapsed().as_secs_f32();
                println!("FPS: {}", 1. / delta);
                aux.last_update = Instant::now();

                let speed = delta * 2.;

                let forward_vec = aux.camera.get_view_direction().normalize();
                let sideways_vec = forward_vec
                    .cross(&nalgebra::Vector3::new(0., 0., 1.))
                    .normalize();

                if aux.keys[Direction::Forward as usize] {
                    aux.camera.pos -= forward_vec * speed;
                }
                if aux.keys[Direction::Backward as usize] {
                    aux.camera.pos += forward_vec * speed;
                }
                if aux.keys[Direction::Right as usize] {
                    aux.camera.pos -= sideways_vec * speed;
                }
                if aux.keys[Direction::Left as usize] {
                    aux.camera.pos += sideways_vec * speed;
                }
                if aux.keys[Direction::Down as usize] {
                    aux.camera.pos.z += speed;
                }
                if aux.keys[Direction::Up as usize] {
                    aux.camera.pos.z -= speed;
                }
            }
            _ => {}
        }

        if *control_flow == ControlFlow::Exit && graph.is_some() {
            graph.take().unwrap().dispose(&mut factory, &aux);
            drop(aux.mesh.take());
        }
    })
}

fn main() {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Hello, triangle!")
        .with_inner_size(Size::new(PhysicalSize::new(512, 512)));

    let config: Config = Default::default();

    let rendy = rendy::init::AnyWindowedRendy::init_auto(&config, window, &event_loop).unwrap();

    rendy::with_any_windowed_rendy!((rendy)
        use back; (mut factory, mut families, surface, window) => {

        let size = window.inner_size();
        let mut aux = Aux{
                mesh: None,
                camera: Camera {
                    aspect: size.width as f32 / size.height as f32,
                    far: 20.,
                    ..Default::default()
                },
                size: [size.width, size.height],
                keys: [false; 6],
                last_update: Instant::now()
        };

        let mut graph_builder = GraphBuilder::<_, Aux<_>>::new();

        let window_kind = hal::image::Kind::D2(size.width as u32, size.height as u32, 1, 1);

        let color = graph_builder.create_image(
            window_kind,
            1,
            factory.get_surface_format(&surface),
            Some(hal::command::ClearValue {
                color: hal::command::ClearColor {
                    float32: [0.1, 0.3, 0.4, 1.0],
                },
            }),
        );
        
        let hdr = graph_builder.create_image(
            window_kind,
            1,
            hal::format::Format::Rgba32Sfloat,
            Some(hal::command::ClearValue {
                color: hal::command::ClearColor {
                    float32: [0.1, 0.3, 0.4, 1.0],
                },
            }),
        );

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

        let mesh_pass = graph_builder.add_node(
            mesh::MeshPipeline::builder()
                .into_subpass()
                .with_color(hdr)
                .with_depth_stencil(depth)
                .into_pass()
        );

        let posteffect_pass = graph_builder.add_node(
            post_effect::PostPipeline::builder()
                .with_image(hdr)
                .into_subpass()
                .with_dependency(mesh_pass)
                .with_color(color)
                .into_pass()
        );

        graph_builder.add_node(
            PresentNode::builder(&factory, surface, color)
                .with_dependency(posteffect_pass)
        );

        let graph = graph_builder
            .build(&mut factory, &mut families, &aux)
            .unwrap();

        aux.mesh = {
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
            .build(graph.node_queue(mesh_pass), &factory)
            .unwrap();

            Some(mesh)
        };

        // no autocompletion in macros so this is what you get
        run(event_loop, window, graph, factory, families, aux);
    })
}
