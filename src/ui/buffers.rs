use anyhow::{Context, Result};
use smithay_client_toolkit::shm::{
    Shm,
    slot::{Buffer, Slot, SlotPool},
};
use wayland_client::protocol::wl_shm;

#[derive(Debug)]
pub struct BufferPool<const N: usize> {
    pool: SlotPool,
    buffers: Vec<(Slot, Buffer)>,

    width: i32,
    height: i32,
    stride: i32,
    format: wl_shm::Format,
}

impl<const N: usize> BufferPool<N> {
    pub fn new(
        shm: &Shm,
        width: i32,
        stride: i32,
        height: i32,
        format: wl_shm::Format,
    ) -> Result<Self> {
        let mut buffer_pool = Self {
            pool: SlotPool::new(1024 * 1024, shm).context("Could not create slot pool")?,
            buffers: Vec::with_capacity(N),

            width,
            height,
            stride,
            format,
        };

        buffer_pool
            .ensure_buffers(width, stride, height, format)
            .context("Could not init buffer pool")?;

        Ok(buffer_pool)
    }

    fn ensure_buffers(
        &mut self,
        width: i32,
        stride: i32,
        height: i32,
        format: wl_shm::Format,
    ) -> Result<()> {
        if self.width == width
            && self.height == height
            && self.stride == stride
            && self.format == format
            && !self.buffers.is_empty()
        {
            return Ok(());
        }

        let mut buffers = vec![];
        for _ in 0..N {
            let slot = self
                .pool
                .new_slot(height as usize * stride as usize)
                .context("Could not create slot")?;

            let buffer = self
                .pool
                .create_buffer_in(&slot, width, height, stride, format)
                .context("Could not create buffer in slot")?;

            buffers.push((slot, buffer));
        }

        self.buffers = buffers;
        self.width = width;
        self.height = height;
        self.stride = stride;
        self.format = format;

        Ok(())
    }

    pub fn get(
        &mut self,
        width: i32,
        stride: i32,
        height: i32,
        format: wl_shm::Format,
    ) -> Result<Option<(&Buffer, &mut [u8])>> {
        self.ensure_buffers(width, stride, height, format)
            .context("Could not ensure buffers")?;

        let buffer = self
            .buffers
            .iter_mut()
            .find(|i| !i.0.has_active_buffers())
            .map(|(_, buffer)| (buffer.canvas(&mut self.pool), buffer))
            .and_then(|(byts, buffer)| byts.map(|byts| (buffer, byts)));

        if let Some((buffer, byts)) = buffer {
            Ok(Some((buffer, byts)))
        } else {
            Ok(None)
        }
    }
}
