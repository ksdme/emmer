use anyhow::{Context, Result};
use skia_safe::{ImageInfo, surfaces};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{BTN_RIGHT, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, Layer, LayerShell, LayerShellHandler, LayerSurface},
    },
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    Connection, EventQueue, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_pointer::WlPointer, wl_seat, wl_shm},
};

use crate::{
    config::{ComputedConfig, Config, Insets, SpreadConfig, StackConfig, ThemeConfig},
    logged, notification,
    ui::{
        buffers::BufferPool,
        items::{LayoutMode, Stack},
    },
};

/// The top level Wayland client.
pub struct App {
    queue_handle: QueueHandle<Self>,

    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,

    layer_surface: LayerSurface,
    seat_state: SeatState,

    pointer: Option<WlPointer>,

    shm: Shm,
    buffer_pool: Option<BufferPool<3>>,

    width: i32,
    height: i32,

    stack: Stack,
    config: ComputedConfig,
}

// Required for compositor delegation.
impl OutputHandler for App {
    fn output_state(&mut self) -> &mut smithay_client_toolkit::output::OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        log::debug!(target: "emmer::wl::output", "new_output: {output:?}");
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        log::debug!(target: "emmer::wl::output", "update_output");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        log::debug!(target: "emmer::wl::output", "output_destroyed");
    }
}
delegate_output!(App);

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        log::debug!(target: "emmer::wl::compositor", "scale_factor_changed");
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
        log::debug!(target: "emmer::wl::compositor", "transform_changed");
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _wl_surface: &wayland_client::protocol::wl_surface::WlSurface,
        _time: u32,
    ) {
        log::trace!(target: "emmer::wl::compositor", "frame");
        let _ = logged!(self.draw().context("Could not draw frame"));
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        surface: &wayland_client::protocol::wl_surface::WlSurface,
        output: &wayland_client::protocol::wl_output::WlOutput,
    ) {
        log::debug!(target: "emmer::wl::compositor", "surface_enter");
        dbg!(surface, output);
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _output: &wayland_client::protocol::wl_output::WlOutput,
    ) {
        log::debug!(target: "emmer::wl::compositor", "surface_leave");
    }
}
delegate_compositor!(App);

impl LayerShellHandler for App {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
    ) {
        log::debug!(target: "emmer::wl::layer_shell", "closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
        configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        log::debug!(target: "emmer::wl::layer_shell", "configure: {configure:?}");

        let (w, h) = configure.new_size;
        self.width = w as i32;
        self.height = h as i32;

        // FIXME: Because we do not refresh render caches, accepting lower widths will
        // cause visual artifacts.
        layer.set_size(w, h);
        layer.commit();

        // Update the buffer pool.
        self.buffer_pool = logged!(
            BufferPool::new(&self.shm, w, w * 4, h, wl_shm::Format::Argb8888,)
                .context("Could not initialize buffer pool")
        )
        .ok();

        let _ = logged!(self.draw().context("Could not draw frame"));
    }
}
delegate_layer!(App);

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        log::debug!(target: "emmer::wl::shm", "shm_state");
        &mut self.shm
    }
}
delegate_shm!(App);

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _pointer: &wayland_client::protocol::wl_pointer::WlPointer,
        events: &[smithay_client_toolkit::seat::pointer::PointerEvent],
    ) {
        log::trace!(target: "emmer::wl::pointer", "shm_state");

        for e in events {
            match e.kind {
                smithay_client_toolkit::seat::pointer::PointerEventKind::Release {
                    time: _,
                    button,
                    serial: _,
                } => {
                    log::trace!(target: "emmer::wl::pointer", "frame release");
                    if button == BTN_RIGHT {
                        let _ = logged!(self.dismiss((e.position.0 as f32, e.position.1 as f32)));
                        break;
                    }
                }
                smithay_client_toolkit::seat::pointer::PointerEventKind::Enter { serial: _ } => {
                    log::trace!(target: "emmer::wl::pointer", "switched to spread");
                    let _ = logged!(self.set_mode(LayoutMode::Spread));
                    break;
                }
                smithay_client_toolkit::seat::pointer::PointerEventKind::Leave { serial: _ } => {
                    log::trace!(target: "emmer::wl::pointer", "switching to stacked");
                    let _ = logged!(self.set_mode(LayoutMode::Stacked));

                    break;
                }
                _ => {}
            }
        }
    }
}
delegate_pointer!(App);

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        seat: wl_seat::WlSeat,
    ) {
        log::debug!(target: "emmer::wl::seat", "new_seat: {seat:?}");
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        log::debug!(target: "emmer::wl::seat", "new_capability: {capability:?}");

        if capability == Capability::Pointer {
            if let Ok(pointer) = logged!(self.seat_state.get_pointer(qh, &seat)) {
                self.pointer = Some(pointer);
            };
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        log::debug!(target: "emmer::wl::seat", "remove_capability: {capability:?}");
    }

    fn remove_seat(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        seat: wl_seat::WlSeat,
    ) {
        log::debug!(target: "emmer::wl::seat", "remove_seat: {seat:?}");
    }
}
delegate_seat!(App);

// Required to start the queue and keep the globals up to date.
impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!();
}
delegate_registry!(App);

impl App {
    /// Initialize the app using a wayland connection.
    pub fn init(conn: &Connection) -> Result<(Self, EventQueue<Self>)> {
        let config = ComputedConfig::from(Config {
            margin: Insets { x: 32., y: 32. },
            padding: Insets { x: 12., y: 12. },
            spread: SpreadConfig {
                gap: 8.,
                max_count: 20,
            },
            stack: StackConfig {
                peek: 8.,
                inset: 8.,
                max_count: 3,
            },
            theme: ThemeConfig {
                font_family: "Ubuntu".into(),
            },
            width: 320.,
        });

        let (globals, event_queue) =
            registry_queue_init::<Self>(conn).context("Could not create wayland queue")?;
        let q_handle = event_queue.handle();

        let output_state = OutputState::new(&globals, &q_handle);
        let registry_state = RegistryState::new(&globals);

        let compositor_state = CompositorState::bind(&globals, &q_handle)
            .context("Could not bind for compositor events")?;
        let layer_shell = LayerShell::bind(&globals, &q_handle)
            .context("Could not bind for zwlr_layer_shell_v1 events")?;

        let surface = compositor_state.create_surface(&q_handle);
        let layer_surface = layer_shell.create_layer_surface(
            &q_handle,
            surface.clone(),
            Layer::Top,
            Option::<String>::None,
            None,
        );

        let (w, h) = (config.width + 2. * config.margin.x, 0);
        layer_surface.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::RIGHT);
        layer_surface.set_size(w as u32, h);

        let region = Region::new(&compositor_state).context("Could not create initial region")?;
        layer_surface.set_input_region(Some(region.wl_region()));

        layer_surface.commit();
        surface.commit();

        let seat_state = SeatState::new(&globals, &q_handle);
        let shm = Shm::bind(&globals, &q_handle).context("Could not bind shm")?;

        Ok((
            App {
                queue_handle: q_handle,

                registry_state,
                output_state,
                compositor_state,

                layer_surface,
                seat_state,

                pointer: None,

                shm,
                buffer_pool: None,

                width: 0,
                height: 0,

                stack: Stack::new(),
                config,
            },
            event_queue,
        ))
    }
}

impl App {
    pub fn draw(&mut self) -> Result<()> {
        let wl_surface = self.layer_surface.wl_surface();

        // Try acquiring a buffer and if we can't find one, queue another draw call
        // and skip this frame.
        let buffer = self
            .buffer_pool
            .as_mut()
            .context("BufferPool unavailable")?
            .get()
            .context("Could not get buffer from pool")?;

        let Some((frame_buffer, canvas)) = buffer else {
            log::debug!("dropping frame: could not acquire a buffer");
            wl_surface.frame(&self.queue_handle, wl_surface.clone());
            wl_surface.commit();
            return Ok(());
        };

        // Render to a skia surface.
        let mut surface = surfaces::raster_n32_premul((self.width, self.height))
            .context("Could not create skia surface")?;
        let (transitions_settled, bounds) = self.stack.render(&self.config, surface.canvas());

        let image_info = ImageInfo::new_n32_premul((surface.width(), surface.height()), None);
        surface.read_pixels(
            &image_info,
            canvas,
            image_info.bytes_per_pixel() * image_info.width() as usize,
            (0, 0),
        );

        // Update the input region.
        let region = Region::new(&self.compositor_state).context("Could not create region")?;
        region.add(
            bounds.x() as i32,
            bounds.y() as i32,
            bounds.width() as i32,
            (bounds.height() as i32).min(self.height),
        );
        wl_surface.set_input_region(Some(region.wl_region()));

        // Request an update to the frame.
        frame_buffer
            .attach_to(wl_surface)
            .context("Could not attach buffer")?;

        wl_surface.damage_buffer(0, 0, self.width, self.height);
        if !transitions_settled {
            wl_surface.frame(&self.queue_handle, wl_surface.clone());
        }

        wl_surface.commit();

        Ok(())
    }

    pub fn push(&mut self, notification: notification::Notification) -> Result<()> {
        self.stack.push(&self.config, notification);
        self.draw().context("Could not draw frame")
    }

    pub fn dismiss(&mut self, at: (f32, f32)) -> Result<()> {
        self.stack.dismiss(&self.config, at);
        self.draw().context("Could not draw frame")
    }

    pub fn set_mode(&mut self, mode: LayoutMode) -> Result<()> {
        self.stack.set_mode(&self.config, mode);
        self.draw().context("Could not draw frame")
    }
}

/// Represents a message passed from outside the UI thread.
#[derive(Debug)]
pub enum UIMessage {
    Push(notification::Notification),
}

impl App {
    pub fn handle(&mut self, msg: UIMessage) -> Result<()> {
        match msg {
            UIMessage::Push(notification) => self
                .push(notification)
                .context("Could not push notification"),
        }
    }
}
