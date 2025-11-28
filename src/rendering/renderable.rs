pub trait Renderable<'window>: Sized {
    fn render(
        data: &[Self],
        gpu: crate::rendering::GpuHandle<'window>,
        target: wgpu::Surface,
    ) -> wgpu::CommandBuffer;
}
pub struct OldRenderable<T: Vertex> {
    vertex_buffer: Vec<T>,
    index_buffer: Vec<u32>,
    texture: crate::sprite::Sprite,
    instance_buffer: Vec<Instance>,
    clip: Option<egui::Rect>,
}

impl<T: Vertex> OldRenderable<T> {
    pub fn new(
        vertex_buffer: Vec<T>,
        index_buffer: Vec<u32>,
        texture: crate::sprite::Sprite,
        instance_buffer: Option<Vec<Instance>>,
        clip: Option<egui::Rect>,
    ) -> Self {
        Self {
            vertex_buffer,
            index_buffer,
            texture,
            instance_buffer: instance_buffer.unwrap_or(vec![Instance::NOOP]),
            clip,
        }
    }
    pub fn vertex_byte_slice(renderables: &[Self]) -> Vec<u8> {
        renderables
            .iter()
            .flat_map(|renderable| bytemuck::cast_slice(renderable.vertex_buffer.as_slice()))
            .copied()
            .collect::<Vec<u8>>()
    }
    pub fn index_byte_slice(renderables: &[Self]) -> Vec<u8> {
        let mut bytes = vec![];
        let mut current_len: usize;
        for indecies in renderables
            .iter()
            .map(|renderable| renderable.index_buffer.clone())
        {
            current_len = bytes.len();
            for index in indecies {
                bytes.extend(bytemuck::bytes_of(&(index + current_len as u32)));
            }
        }
        bytes
    }
    pub fn clip(&self) -> Option<egui::Rect> {
        self.clip
    }
}

pub trait Vertex: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable {
    const ATTRIBUTES: &[wgpu::VertexAttribute];
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawVertex {
    pub position: glam::Vec3,
    pub texture_coordinates: glam::Vec2,
}

impl Vertex for RawVertex {
    const ATTRIBUTES: &[wgpu::VertexAttribute] = &[wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    }];
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Instance {
    position: glam::Vec3,
    rotation: glam::Quat,
}
impl Instance {
    pub const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<glam::Mat4>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::size_of::<[glam::Vec4; 0]>() as u64,
                shader_location: 5,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::size_of::<[glam::Vec4; 1]>() as u64,
                shader_location: 6,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::size_of::<[glam::Vec4; 2]>() as u64,
                shader_location: 7,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::size_of::<[glam::Vec4; 3]>() as u64,
                shader_location: 8,
            },
        ],
    };
    pub const NOOP: Instance = Instance {
        position: glam::Vec3::ZERO,
        rotation: glam::Quat::IDENTITY,
    };
    pub fn get_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_rotation_translation(self.rotation, self.position)
    }
}
