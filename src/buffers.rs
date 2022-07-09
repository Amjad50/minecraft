use std::{cell::Cell, sync::Arc};

use vulkano::{
    buffer::{
        cpu_pool::CpuBufferPoolChunk, BufferContents, BufferUsage, CpuBufferPool, DeviceLocalBuffer,
    },
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    device::Queue,
    memory::pool::StdMemoryPool,
};

#[derive(Clone)]
pub struct MirroredBuffer<T>
where
    [T]: BufferContents,
{
    queue: Arc<Queue>,
    buffer_usage: BufferUsage,
    buffers: Vec<Arc<DeviceLocalBuffer<[T]>>>,
    staging_buffer_pool: CpuBufferPool<T>,
    staging_buffer: Arc<CpuBufferPoolChunk<T, Arc<StdMemoryPool>>>,
    current_buffer: Arc<Cell<usize>>,
    dirty: Arc<Cell<bool>>,
    instances: usize,
}

impl<T> MirroredBuffer<T>
where
    [T]: BufferContents,
{
    pub fn from_iter<I>(
        queue: &Arc<Queue>,
        instances: usize,
        buffer_usage: BufferUsage,
        data: I,
    ) -> Self
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        assert!(instances > 0);

        let iter = data.into_iter();

        let mut buffers = Vec::with_capacity(instances);
        if iter.len() > 0 {
            for _ in 0..instances {
                buffers.push(
                    DeviceLocalBuffer::array(
                        queue.device().clone(),
                        iter.len() as _,
                        BufferUsage {
                            transfer_destination: true,
                            ..buffer_usage
                        },
                        [queue.family()],
                    )
                    .unwrap(),
                );
            }
        }

        let staging_buffer_pool = CpuBufferPool::new(
            queue.device().clone(),
            BufferUsage {
                transfer_source: true,
                ..buffer_usage
            },
        );
        let staging_buffer = staging_buffer_pool.chunk(iter).unwrap();
        MirroredBuffer {
            queue: queue.clone(),
            buffer_usage,
            buffers,
            staging_buffer_pool,
            staging_buffer,
            instances,
            current_buffer: Arc::new(Cell::new(0)),
            dirty: Arc::new(Cell::new(true)),
        }
    }

    pub fn update_data<I>(&mut self, data: I)
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let iter = data.into_iter();

        self.buffers.clear();
        for _ in 0..self.instances {
            self.buffers.push(
                DeviceLocalBuffer::array(
                    self.queue.device().clone(),
                    iter.len() as _,
                    BufferUsage {
                        transfer_destination: true,
                        ..self.buffer_usage
                    },
                    [self.queue.family()],
                )
                .unwrap(),
            );
        }

        self.staging_buffer = self.staging_buffer_pool.chunk(iter).unwrap();

        self.dirty.set(true);
    }
}

impl<T> MirroredBuffer<T>
where
    [T]: BufferContents,
    T: BufferContents,
{
    pub fn update_buffers(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        if self.dirty.get() {
            self.dirty.set(false);
            for b in &self.buffers {
                builder
                    .copy_buffer(self.staging_buffer.clone(), b.clone())
                    .unwrap();
            }
        }
    }

    pub fn move_to_next(&self) {
        self.current_buffer
            .set((self.current_buffer.get() + 1) % self.buffers.len());
    }

    pub fn current_buffer(&self) -> &Arc<DeviceLocalBuffer<[T]>> {
        &self.buffers[self.current_buffer.get()]
    }
}
