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
    shape: (u32, u32, u32, wl_shm::Format),
}

impl<const N: usize> BufferPool<N> {
    pub fn new(
        shm: &Shm,
        width: u32,
        stride: u32,
        height: u32,
        format: wl_shm::Format,
    ) -> Result<Self> {
        let mut buffer_pool = Self {
            pool: SlotPool::new(1024 * 1024, shm).context("Could not create slot pool")?,
            buffers: Vec::with_capacity(N),
            shape: (width, stride, height, format),
        };

        buffer_pool
            .ensure_buffers(width, stride, height, format)
            .context("Could not init buffer pool")?;

        Ok(buffer_pool)
    }

    fn ensure_buffers(
        &mut self,
        width: u32,
        stride: u32,
        height: u32,
        format: wl_shm::Format,
    ) -> Result<()> {
        if self.shape == (width, stride, height, format) && !self.buffers.is_empty() {
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
                .create_buffer_in(&slot, width as i32, height as i32, stride as i32, format)
                .context("Could not create buffer in slot")?;

            buffers.push((slot, buffer));
        }

        self.buffers = buffers;
        self.shape = (width, stride, height, format);

        Ok(())
    }

    pub fn get(&mut self) -> Result<Option<(&Buffer, &mut [u8])>> {
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
