use crate::rendering;
use std::sync;

pub struct Sprite {
    texture: sync::Arc<GpuTexture>,
    frames: u16,
}

impl Sprite {
    pub fn new(texture: sync::Arc<GpuTexture>, frames: u16) -> Self {
        Self { texture, frames }
    }
}

pub struct GpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

impl GpuTexture {
    pub const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'_> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        };
    pub fn new(
        texture_descriptor: wgpu::TextureDescriptor,
        gpu_handle: rendering::GpuHandle,
    ) -> Self {
        let gpu = gpu_handle.read().unwrap();
        let texture = gpu.device().create_texture(&texture_descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &gpu
                .device()
                .create_bind_group_layout(&Self::BIND_GROUP_LAYOUT_DESCRIPTOR),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&gpu.device().create_sampler(
                        &wgpu::SamplerDescriptor {
                            label: Some("Texture Sampler"),
                            address_mode_u: wgpu::AddressMode::ClampToEdge,
                            address_mode_v: wgpu::AddressMode::ClampToEdge,
                            address_mode_w: wgpu::AddressMode::ClampToEdge,
                            mag_filter: wgpu::FilterMode::Nearest,
                            min_filter: wgpu::FilterMode::Nearest,
                            mipmap_filter: wgpu::FilterMode::Nearest,
                            anisotropy_clamp: 1,
                            ..Default::default()
                        },
                    )),
                },
            ],
        });
        Self {
            texture,
            view,
            bind_group,
        }
    }
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
    pub fn texel_copy_texture_info(
        &self,
        mip_level: u32,
        origin: wgpu::Origin3d,
    ) -> wgpu::TexelCopyTextureInfo {
        wgpu::TexelCopyTextureInfo {
            texture: self.texture(),
            mip_level,
            origin,
            aspect: wgpu::TextureAspect::All,
        }
    }
}
