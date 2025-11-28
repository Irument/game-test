use wgpu::util::DeviceExt;

use crate::rendering;
use crate::rendering::renderable::Vertex;
use crate::sprite;

use std::collections;
use std::sync;

const SHADER: &[u8] = include_bytes!("user_interface.wgsl");
static RENDER_PIPLINE: sync::OnceLock<wgpu::RenderPipeline> = sync::OnceLock::new();

fn init_render_pipeline(gpu: rendering::GpuHandle) -> wgpu::RenderPipeline {
    let gpu = gpu.read().unwrap();
    let shader = &gpu
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("user interface shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                core::str::from_utf8(SHADER).unwrap(),
            )),
        });
    gpu.device()
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("user interface render pipeline"),
            layout: Some(
                &gpu.device()
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("user interface render pipeline layout"),
                        bind_group_layouts: &[
                            &gpu.device().create_bind_group_layout(
                                &sprite::GpuTexture::BIND_GROUP_LAYOUT_DESCRIPTOR,
                            ),
                            &gpu.device().create_bind_group_layout(
                                &UserInterfaceProjectionMatrix::BIND_GROUP_LAYOUT_DESCRIPTOR,
                            ),
                        ],
                        push_constant_ranges: &[],
                    }),
            ),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[ColoredVertex::buffer_layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.surface_config().format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview: None,
            cache: None,
        })
}

pub struct UserInterface<'window> {
    context: egui::Context,
    renderer: UserInterfaceRenderer<'window>,
    pub user_interface_input: egui::RawInput,
    pub last_mouse_pos: egui::Pos2,
}

impl<'window> UserInterface<'window> {
    pub fn new(gpu_handle: rendering::GpuHandle<'window>) -> Self {
        let renderer = UserInterfaceRenderer::new(gpu_handle);
        Self {
            context: egui::Context::default(),
            renderer,
            user_interface_input: egui::RawInput::default(),
            last_mouse_pos: egui::Pos2::default(),
        }
    }
    pub fn update<F: FnMut(&egui::Context)>(&mut self, root: F) {
        let mut swap_input = egui::RawInput::default();
        std::mem::swap(&mut self.user_interface_input, &mut swap_input);
        let egui::FullOutput {
            platform_output: _,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output: _,
        } = self.context.run(swap_input, root);

        for (id, image_delta) in textures_delta.set {
            log::info!("Writing texture: {id:?}");
            self.renderer.write_texture(&id, image_delta);
            log::info!("Finished writing texture: {id:?}");
        }

        let data = self
            .context
            .tessellate(shapes, pixels_per_point)
            .into_iter()
            .map(UserInterfaceRenderable::from)
            .collect::<Vec<_>>();
        let gpu = self.renderer.gpu_handle.read().unwrap();
        let surface_config = gpu.surface_config();
        self.renderer.projection_matrix.update(glam::vec2(
            (surface_config.width as f32).recip(),
            (surface_config.height as f32).recip(),
        ));
        drop(gpu);
        self.renderer.render(&data);
        for id in textures_delta.free {
            self.renderer.textures.remove(&id);
        }
    }
}

pub struct UserInterfaceRenderer<'window> {
    gpu_handle: rendering::GpuHandle<'window>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    textures: collections::HashMap<egui::TextureId, sync::Arc<sprite::GpuTexture>>,
    projection_matrix: UserInterfaceProjectionMatrix<'window>,
}

impl<'window> UserInterfaceRenderer<'window> {
    pub fn new(gpu_handle: rendering::GpuHandle<'window>) -> Self {
        let gpu = gpu_handle.read().unwrap();
        let vertex_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("User Interface Vertex Buffer"),
            size: 100000,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("User Interface Index Buffer"),
            size: 100000,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        drop(gpu);
        Self {
            gpu_handle: gpu_handle.clone(),
            vertex_buffer,
            index_buffer,
            textures: collections::HashMap::new(),
            projection_matrix: UserInterfaceProjectionMatrix::new(gpu_handle.clone()),
        }
    }
    fn write_texture(&mut self, id: &egui::TextureId, image_delta: egui::epaint::ImageDelta) {
        if !self.textures.contains_key(id) {
            self.allocate_texture(
                id,
                wgpu::Extent3d {
                    width: image_delta.image.width() as u32,
                    height: image_delta.image.height() as u32,
                    depth_or_array_layers: 1,
                },
            );
        }

        let gpu = self.gpu_handle.write().unwrap();
        let texture = self.textures.get(id).unwrap();
        let image = match image_delta.image.clone() {
            egui::ImageData::Color(color_image) => color_image,
        };
        let texel_copy_info = if let Some([x, y]) = image_delta.pos {
            texture.texel_copy_texture_info(
                0,
                wgpu::Origin3d {
                    x: x as u32,
                    y: y as u32,
                    z: 0,
                },
            )
        } else {
            texture.texture().as_image_copy()
        };
        let texel_layout = wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some((image_delta.image.bytes_per_pixel() * image.width()) as u32),
            rows_per_image: Some(image.height() as u32),
        };
        gpu.queue().write_texture(
            texel_copy_info,
            bytemuck::cast_slice(&image.pixels),
            texel_layout,
            wgpu::Extent3d {
                width: image.width() as u32,
                height: image.height() as u32,
                depth_or_array_layers: 1,
            },
        );
    }
    fn allocate_texture(&mut self, id: &egui::TextureId, size: wgpu::Extent3d) {
        let texture_label = format!("gui texture id: {id:?}");
        let texture_descriptor = wgpu::TextureDescriptor {
            label: Some(&texture_label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };
        self.textures.insert(
            *id,
            sync::Arc::new(sprite::GpuTexture::new(
                texture_descriptor,
                self.gpu_handle.clone(),
            )),
        );
    }
    fn render(&self, data: &[UserInterfaceRenderable]) {
        let mut gpu = self.gpu_handle.write().unwrap();
        let mut command_encoder =
            gpu.device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("user interface command encoder"),
                });
        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("user interface render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &gpu
                    .output()
                    .unwrap()
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        drop(gpu);
        render_pass.set_pipeline(
            RENDER_PIPLINE.get_or_init(|| init_render_pipeline(self.gpu_handle.clone())),
        );
        let mut gpu = self.gpu_handle.write().unwrap();
        let vertices = data
            .iter()
            .flat_map(|renderable| bytemuck::cast_slice(&renderable.verticies))
            .copied()
            .collect::<Vec<u8>>();

        let indices = data
            .iter()
            .flat_map(|renderable| bytemuck::cast_slice(&renderable.indicies))
            .copied()
            .collect::<Vec<u8>>();

        gpu.write_buffer(
            &self.vertex_buffer,
            0,
            std::num::NonZero::new(vertices.len() as u64).unwrap(),
        )
        .copy_from_slice(&vertices);
        gpu.write_buffer(
            &self.index_buffer,
            0,
            std::num::NonZero::new(indices.len() as u64).unwrap(),
        )
        .copy_from_slice(&indices);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(1, &self.projection_matrix.bind_group, &[]);

        let mut base_vertex = 0;
        for renderable in data {
            render_pass.set_scissor_rect(
                renderable.clip.min.x as u32,
                renderable.clip.min.y as u32,
                (renderable.clip.max.x as u32).min(gpu.surface_config().width),
                (renderable.clip.max.y as u32).min(gpu.surface_config().height),
            );

            render_pass.set_bind_group(
                0,
                self.textures
                    .get(&renderable.texture)
                    .expect("texture id should be valid")
                    .bind_group(),
                &[],
            );

            render_pass.draw_indexed(0..renderable.indicies.len() as u32, base_vertex, 0..1);

            base_vertex += renderable.indicies.len() as i32;
        }
        drop(render_pass);
        gpu.push_command_buffer(command_encoder.finish());
    }
}

#[derive(Debug)]
pub struct UserInterfaceRenderable {
    verticies: Vec<ColoredVertex>,
    indicies: Vec<u32>,
    texture: egui::TextureId,
    clip: egui::Rect,
}

impl From<egui::epaint::ClippedPrimitive> for UserInterfaceRenderable {
    fn from(value: egui::epaint::ClippedPrimitive) -> Self {
        let egui::epaint::ClippedPrimitive {
            clip_rect,
            primitive,
        } = value;

        let mesh = match primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh,
            egui::epaint::Primitive::Callback(_paint_callback) => egui::Mesh::default(),
        };

        let verticies = mesh
            .vertices
            .iter()
            .map(|egui::epaint::Vertex { pos, uv, color }| ColoredVertex {
                position: glam::Vec2 { x: pos.x, y: pos.y },
                uv: glam::Vec2 { x: uv.x, y: uv.y },
                color: *color,
            })
            .collect::<Vec<ColoredVertex>>();

        let ret = UserInterfaceRenderable {
            verticies,
            indicies: mesh.indices,
            texture: mesh.texture_id,
            clip: clip_rect,
        };
        ret
    }
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct ColoredVertex {
    position: glam::Vec2,
    uv: glam::Vec2,
    color: egui::epaint::Color32,
}
impl Vertex for ColoredVertex {
    const ATTRIBUTES: &[wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: std::mem::size_of::<glam::Vec2>() as u64,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Unorm8x4,
            offset: std::mem::size_of::<glam::Vec2>() as u64 * 2,
            shader_location: 2,
        },
    ];
}

struct UserInterfaceProjectionMatrix<'window> {
    gpu_handle: rendering::GpuHandle<'window>,
    matrix: glam::Mat4,
    bind_group: wgpu::BindGroup,
    buffer: wgpu::Buffer,
}
impl<'a, 'window> UserInterfaceProjectionMatrix<'window> {
    const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'a> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("user interface projection matrix bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        };
    pub fn new(gpu_handle: rendering::GpuHandle<'window>) -> Self {
        let gpu = gpu_handle.read().unwrap();
        let matrix = glam::Mat4::IDENTITY;
        let buffer = gpu
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("User interface projection matrix buffer"),
                contents: bytemuck::cast_slice(&[matrix]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let bind_group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("User interface projection matrix bind group"),
            layout: &gpu
                .device()
                .create_bind_group_layout(&Self::BIND_GROUP_LAYOUT_DESCRIPTOR),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        Self {
            gpu_handle: gpu_handle.clone(),
            matrix,
            bind_group,
            buffer,
        }
    }
    pub fn update(&mut self, new_scale: glam::Vec2) {
        let gpu = self.gpu_handle.read().unwrap();
        self.matrix = glam::Mat4::IDENTITY;
        // glam::Mat4::from_translation(glam::Vec3::new(-1.0, -1.0, 0.0))
        // * glam::Mat4::from_scale(glam::Vec3::new(new_scale.x, new_scale.y, 1.0));
        gpu.queue()
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.matrix]));
    }
}
