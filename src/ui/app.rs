use anyhow::{Context, Result};
use log::debug;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState, pointer::PointerHandler},
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, Layer, LayerShell, LayerShellHandler, LayerSurface},
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{
    Connection, EventQueue, Proxy,
    globals::registry_queue_init,
    protocol::{wl_pointer::WlPointer, wl_seat},
};

use crate::ui::state::State;

/// The top level Wayland client.
pub struct App {
    registry_state: RegistryState,
    output_state: OutputState,

    layer_surface: LayerSurface,
    seat_state: SeatState,
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
        debug!("wl_output: new_output ({:?})", output.id());
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        debug!("wl_output: update_output");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        debug!("wl_output: output_destroyed");
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
        debug!("wl_compositor: scale_factor_changed");
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
        debug!("wl_compositor: transform_changed");
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
        wl_surface: &wayland_client::protocol::wl_surface::WlSurface,
        _time: u32,
    ) {
        debug!("wl_compositor: frame");
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
        debug!("wl_compositor: surface_enter");
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _output: &wayland_client::protocol::wl_output::WlOutput,
    ) {
        debug!("wl_compositor: surface_leave");
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
        debug!("wl_layer_shell_handler: closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
        _configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        debug!("wl_layer_shell_handler: configure");

        self.state.width = 128 * 3;
        self.state.height = 128 * 3;
        self.state.add_item();

        layer.set_size(self.state.width as u32, self.state.height as u32);
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
        debug!("wl_shm: shm_state");
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
        _events: &[smithay_client_toolkit::seat::pointer::PointerEvent],
    ) {
        debug!("wl_pointer: frame");
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
        debug!("wl_seat: new_seat ({:?})", seat.id());
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        debug!("wl_seat: new_capability ({:?})", capability);

        if capability == Capability::Pointer {
            self.pointer = Some(
                self.seat_state
                    .get_pointer(qh, &seat)
                    .context("Could not get_pointer")
                    .unwrap(),
            );
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        debug!("wl_seat: remove_capability ({:?})", capability);
    }

    fn remove_seat(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        seat: wl_seat::WlSeat,
    ) {
        debug!("wl_seat: remove_seat ({:?})", seat.id());
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

        let seat_state = SeatState::new(&globals, &q_handle);

        let shm = Shm::bind(&globals, &q_handle).expect("wl_shm");
        let slot_pool = SlotPool::new(128 * 64, &shm).expect("wl_shm_pool");

        Ok((
            App {
                registry_state,
                output_state,

                layer_surface,
                seat_state,
                pointer: None,

                shm,
                slot_pool,

                state: State::new(),
            },
            event_queue,
        ))
    }
}
