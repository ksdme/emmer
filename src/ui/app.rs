use anyhow::{Context, Result};
use log::{debug, trace};
use skia_safe::{ImageInfo, surfaces};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::KeyboardHandler,
        pointer::{BTN_RIGHT, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{
    Connection, EventQueue, Proxy, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_keyboard::WlKeyboard, wl_pointer::WlPointer, wl_seat, wl_shm::Format},
};

use crate::{
    config::{ComputedConfig, Config, Insets, SpreadConfig, StackConfig, ThemeConfig},
    notification,
    ui::items::{LayoutMode, Stack},
};

/// The top level Wayland client.
pub struct App {
    queue_handle: QueueHandle<Self>,

    registry_state: RegistryState,
    output_state: OutputState,

    layer_surface: LayerSurface,
    seat_state: SeatState,

    keyboard: Option<WlKeyboard>,
    pointer: Option<WlPointer>,

    shm: Shm,
    slot_pool: SlotPool,

    // TODO: For some reason, both cairo and smithay expect i32 for dimensions.
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
        debug!(target: "wl_output", "new_output ({:?})", output.id());
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        debug!(target: "wl_output", "update_output");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        debug!(target: "wl_output", "output_destroyed");
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
        debug!(target: "wl_compositor", "scale_factor_changed");
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
        debug!(target: "wl_compositor", "transform_changed");
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _wl_surface: &wayland_client::protocol::wl_surface::WlSurface,
        _time: u32,
    ) {
        trace!(target: "wl_compositor", "frame");
        self.draw().context("Could not draw frame").unwrap();
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _output: &wayland_client::protocol::wl_output::WlOutput,
    ) {
        debug!(target: "wl_compositor", "surface_enter");
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _output: &wayland_client::protocol::wl_output::WlOutput,
    ) {
        debug!(target: "wl_compositor", "surface_leave");
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
        debug!(target: "wl_layer_shell_handler", "closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
        _configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        debug!(target: "wl_layer_shell_handler", "configure");

        self.width = 128 * 3;
        self.height = 128 * 6;

        // Setup size.
        layer.set_size(self.width as u32, self.height as u32);
        layer.commit();

        // Setup keyboard.
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.commit();

        self.draw().context("Could not draw frame").unwrap();
    }
}
delegate_layer!(App);

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        debug!(target: "wl_shm", "shm_state");
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
        trace!(target: "wl_pointer", "frame");

        for e in events {
            match e.kind {
                smithay_client_toolkit::seat::pointer::PointerEventKind::Release {
                    time: _,
                    button,
                    serial: _,
                } => {
                    trace!("wl_pointer: frame click");
                    if button == BTN_RIGHT {
                        self.dismiss((e.position.0 as f32, e.position.1 as f32))
                            .unwrap();
                        break;
                    }
                }
                smithay_client_toolkit::seat::pointer::PointerEventKind::Enter { serial: _ } => {
                    trace!("wl_pointer: frame expanding");
                    self.set_mode(LayoutMode::Spread).unwrap();
                    break;
                }
                smithay_client_toolkit::seat::pointer::PointerEventKind::Leave { serial: _ } => {
                    trace!("wl_pointer: frame contracting");
                    self.set_mode(LayoutMode::Stacked).unwrap();
                    break;
                }
                _ => {}
            }
        }
    }
}
delegate_pointer!(App);

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
        trace!(target: "wl_keyboard", "enter");
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _serial: u32,
    ) {
        trace!(target: "wl_keyboard", "leave");
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        trace!(target: "wl_keyboard", "press_key");
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        trace!(target: "wl_keyboard", "repeat_key");
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _e: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        trace!(target: "wl_keyboard", "release_key");
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
        _raw_modifiers: smithay_client_toolkit::seat::keyboard::RawModifiers,
        _layout: u32,
    ) {
        trace!(target: "wl_keyboard", "update_modifiers");
    }
}
delegate_keyboard!(App);

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
        debug!(target: "wl_seat", "new_seat ({:?})", seat.id());
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        debug!(target: "wl_seat", "new_capability ({:?})", capability);

        match capability {
            Capability::Pointer => {
                self.pointer = Some(
                    self.seat_state
                        .get_pointer(qh, &seat)
                        .context("Could not get_pointer")
                        .unwrap(),
                );
            }
            Capability::Keyboard => {
                self.keyboard = Some(
                    self.seat_state
                        .get_keyboard(qh, &seat, None)
                        .context("Could not get_keyboard")
                        .unwrap(),
                );
            }
            _ => {}
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        debug!(target: "wl_seat", "remove_capability ({:?})", capability);
    }

    fn remove_seat(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        seat: wl_seat::WlSeat,
    ) {
        debug!(target: "wl_seat", "remove_seat ({:?})", seat.id());
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

        // TODO: This size needs to be a sensible size somehow.
        layer_surface.set_size(256, 256);
        layer_surface.set_anchor(Anchor::TOP | Anchor::RIGHT);
        layer_surface.commit();

        surface.commit();

        let seat_state = SeatState::new(&globals, &q_handle);

        let shm = Shm::bind(&globals, &q_handle).expect("wl_shm");
        let slot_pool = SlotPool::new(128 * 64, &shm).expect("wl_shm_pool");

        Ok((
            App {
                queue_handle: q_handle,

                registry_state,
                output_state,

                layer_surface,
                seat_state,

                keyboard: None,
                pointer: None,

                shm,
                slot_pool,

                width: 0,
                height: 0,

                stack: Stack::new(),
                config: ComputedConfig::from(Config {
                    margin: Insets { x: 32., y: 32. },
                    padding: Insets { x: 12., y: 12. },
                    spread: SpreadConfig {
                        gap: 8.,
                        max_count: 8,
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
                }),
            },
            event_queue,
        ))
    }
}

impl App {
    pub fn draw(&mut self) -> Result<()> {
        let mut surface = surfaces::raster_n32_premul((self.width, self.height))
            .context("Could not create skia surface")?;

        let request_callback = self.stack.draw(&self.config, surface.canvas());
        let (frame_buffer, canvas) = self
            .slot_pool
            .create_buffer(self.width, self.height, self.width * 4, Format::Argb8888)
            .context("Could not create buffer on pool")?;

        let image_info = ImageInfo::new_n32_premul((surface.width(), surface.height()), None);
        surface.read_pixels(
            &image_info,
            canvas,
            image_info.bytes_per_pixel() * image_info.width() as usize,
            (0, 0),
        );

        let wl_surface = self.layer_surface.wl_surface();
        frame_buffer
            .attach_to(wl_surface)
            .context("Could not attach buffer")?;

        wl_surface.damage_buffer(0, 0, self.width, self.height);
        if request_callback {
            trace!(target: "draw", "requesting another frame");
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
