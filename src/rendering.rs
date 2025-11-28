use anyhow::Result;
use egui::output;
use std::sync;

pub mod buffer;
pub mod renderable;

// const SHADER: &[u8] = include_bytes!("shader.wgsl");

pub type GpuHandle<'window> = sync::Arc<sync::RwLock<Gpu<'window>>>;

pub struct Gpu<'window> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    belt: wgpu::util::StagingBelt,
    belt_encoder: wgpu::CommandEncoder,
    surface: wgpu::Surface<'window>,
    surface_config: wgpu::SurfaceConfiguration,
    output: Option<wgpu::SurfaceTexture>,
    command_buffer: Vec<wgpu::CommandBuffer>,
}
impl<'window> Gpu<'window> {
    pub fn new(window: sync::Arc<winit::window::Window>) -> Result<GpuHandle<'window>> {
        let wgpu_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::from_build_config(),
            backend_options: wgpu::BackendOptions::from_env_or_default(),
        });

        let surface = wgpu_instance.create_surface(window.clone())?;

        let adapter = pollster::block_on(wgpu_instance.request_adapter(
            &wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            },
        ))
        .ok_or(anyhow::anyhow!("No adapters available"))?;

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_capabilities
                .formats
                .iter()
                .find(|format| format.is_srgb())
                .copied()
                .unwrap_or(surface_capabilities.formats[0]),
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: surface_capabilities.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: Vec::new(),
        };

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))?;
        let belt = wgpu::util::StagingBelt::new(16 * 1024);
        let belt_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Gpu belt encoder"),
        });
        surface.configure(&device, &surface_config);
        let output = surface.get_current_texture()?;

        Ok(sync::Arc::new(sync::RwLock::new(Self {
            device,
            queue,
            belt,
            belt_encoder,
            surface,
            surface_config,
            output: Some(output),
            command_buffer: vec![],
        })))
    }
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    pub fn write_buffer(
        &mut self,
        target: &wgpu::Buffer,
        offset: wgpu::BufferAddress,
        size: wgpu::BufferSize,
    ) -> wgpu::BufferViewMut {
        self.belt
            .write_buffer(&mut self.belt_encoder, target, offset, size, &self.device)
    }
    pub fn surface(&self) -> &wgpu::Surface {
        &self.surface
    }
    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.surface_config
    }
    pub fn surface_config_mut(&mut self) -> &mut wgpu::SurfaceConfiguration {
        &mut self.surface_config
    }
    pub fn configure_surface(&self) {
        self.surface.configure(&self.device, &self.surface_config);
    }
    pub fn output(&mut self) -> anyhow::Result<&wgpu::SurfaceTexture> {
        if let Some(ref output) = self.output {
            Ok(output)
        } else {
            let output = self.surface.get_current_texture()?;
            self.output = Some(output);
            Ok(&self
                .output
                .as_ref()
                .expect("output was literally just set to some"))
        }
    }
    pub fn push_command_buffer(&mut self, command_buffer: wgpu::CommandBuffer) {
        self.command_buffer.push(command_buffer)
    }
    pub fn submit_command_buffer(&mut self) {
        self.belt.finish();
        let mut swap_encoder =
            self.device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Gpu belt encoder"),
                });
        std::mem::swap(&mut self.belt_encoder, &mut swap_encoder);
        self.push_command_buffer(swap_encoder.finish());

        self.queue.submit(self.command_buffer.drain(..));

        self.belt.recall();
        if let Some(output) = self.output.take() {
            output.present();
            self.configure_surface();
        }
    }
}
