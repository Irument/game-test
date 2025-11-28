use std::collections;

pub struct GpuBuffer {
    buffer: wgpu::Buffer,
    allocations: collections::BTreeMap<wgpu::BufferAddress, wgpu::BufferSize>,
}

impl GpuBuffer {
    fn new(size: wgpu::BufferSize, device: &wgpu::Device) -> Self {
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("GpuBuffer"),
                size: size.into(),
                usage: wgpu::BufferUsages::all(),
                mapped_at_creation: false,
            }),
            allocations: collections::BTreeMap::new(),
        }
    }
    fn allocate(&self, size: wgpu::BufferSize) -> wgpu::BufferAddress {
        let mut start: u64 = 0;
        for allocation in self.allocations.iter() {
            if allocation.0 - start >= size.get() {}
            start = allocation.0 + allocation.1.get()
        }
        todo!()
    }
    //bitvec = + 1/8 size
    //(address, size) = n_allocations * 8
    //
}
