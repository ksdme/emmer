use anyhow::{Context, Result};
use log::{debug, trace};
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
    Connection, EventQueue, Proxy,
    globals::registry_queue_init,
    protocol::{wl_keyboard::WlKeyboard, wl_pointer::WlPointer, wl_seat},
};

use crate::{config, engine::items::LayoutMode, ui::state::State};

/// The top level Wayland client.
pub struct App {
    registry_state: RegistryState,
    output_state: OutputState,

    layer_surface: LayerSurface,
    seat_state: SeatState,

    keyboard: Option<WlKeyboard>,
    pointer: Option<WlPointer>,

    shm: Shm,
    slot_pool: SlotPool,

    pub state: State,
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
        qh: &wayland_client::QueueHandle<Self>,
        wl_surface: &wayland_client::protocol::wl_surface::WlSurface,
        _time: u32,
    ) {
        trace!(target: "wl_compositor", "frame");
        self.state
            .draw(qh, wl_surface, &mut self.slot_pool)
            .context("Could not draw frame")
            .unwrap();
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
        qh: &wayland_client::QueueHandle<Self>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
        _configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        debug!(target: "wl_layer_shell_handler", "configure");

        self.state.width = 128 * 3;
        self.state.height = 128 * 3;

        // Setup size.
        layer.set_size(self.state.width as u32, self.state.height as u32);
        layer.commit();

        // Setup keyboard.
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.commit();

        self.state
            .draw(qh, layer.wl_surface(), &mut self.slot_pool)
            .context("Could not draw frame")
            .unwrap();
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
        qh: &wayland_client::QueueHandle<Self>,
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
                        self.state.stack.dismiss(
                            &self.state.config,
                            (e.position.0 as f32, e.position.1 as f32),
                        );
                        self.state
                            .draw(qh, &e.surface, &mut self.slot_pool)
                            .context("Could not draw on dismiss")
                            .unwrap();
                        break;
                    }
                }
                smithay_client_toolkit::seat::pointer::PointerEventKind::Enter { serial: _ } => {
                    trace!("wl_pointer: frame expanding");
                    self.state
                        .stack
                        .set_mode(&self.state.config, LayoutMode::Spread);
                    self.state
                        .draw(qh, &e.surface, &mut self.slot_pool)
                        .context("Could not draw on frame expanding")
                        .unwrap();
                    break;
                }
                smithay_client_toolkit::seat::pointer::PointerEventKind::Leave { serial: _ } => {
                    trace!("wl_pointer: frame contracting");
                    self.state
                        .stack
                        .set_mode(&self.state.config, LayoutMode::Stacked);
                    self.state
                        .draw(qh, &e.surface, &mut self.slot_pool)
                        .context("Could not draw on frame contracting")
                        .unwrap();
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
        qh: &wayland_client::QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        e: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        trace!(target: "wl_keyboard", "release_key");

        let surface = self.layer_surface.wl_surface();
        if e.keysym.key_char() == Some('a') {
            self.state.stack.push(&self.state.config);
            self.state
                .draw(qh, &surface, &mut self.slot_pool)
                .context("Could not draw on push")
                .unwrap();
        }
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
            registry_queue_init::<App>(conn).context("Could not create wayland queue")?;
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
                registry_state,
                output_state,

                layer_surface,
                seat_state,

                keyboard: None,
                pointer: None,

                shm,
                slot_pool,

                state: State::new(config::Config {
                    margin: config::Measure { x: 32., y: 32. },
                    spread: config::Spread {
                        gap: 8.,
                        max_count: 8,
                    },
                    stack: config::Stack {
                        peek: 12.,
                        inset: 8.,
                        max_count: 3,
                    },
                    width: 320.,
                }),
            },
            event_queue,
        ))
    }
}
