use std::f64::consts::PI;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, Layer, LayerShell, LayerShellHandler, LayerSurface},
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{Connection, Proxy, globals::registry_queue_init, protocol::wl_shm};

fn main() {
    let conn = Connection::connect_to_env().expect("wl connection");

    let (globals, mut event_queue) = registry_queue_init::<State>(&conn).expect("wl queue init");
    let q_handle = event_queue.handle();

    let output_state = OutputState::new(&globals, &q_handle);
    let registry_state = RegistryState::new(&globals);

    let compositor_state = CompositorState::bind(&globals, &q_handle).expect("wl compositor state");
    let layer_shell = LayerShell::bind(&globals, &q_handle).expect("zwlr_layer_shell_v1");

    let surface = compositor_state.create_surface(&q_handle);
    let layer_surface = layer_shell.create_layer_surface(
        &q_handle,
        surface.clone(),
        Layer::Top,
        Option::<String>::None,
        None,
    );

    layer_surface.set_size(128, 64);
    layer_surface.set_anchor(Anchor::TOP | Anchor::RIGHT);
    layer_surface.commit();

    let shm = Shm::bind(&globals, &q_handle).expect("wl_shm");
    let pool = SlotPool::new(128 * 64, &shm).expect("wl_shm_pool");

    let mut state = State {
        count: 0,
        offset: 0,

        registry_state,
        output_state,

        layer_surface,
        width: 128,
        height: 64,

        shm,
        pool,

        done: false,
    };
    while !state.done {
        event_queue
            .blocking_dispatch(&mut state)
            .expect("event_dispatch");
    }
}

pub struct State {
    count: u64,
    offset: u16,

    registry_state: RegistryState,
    output_state: OutputState,

    layer_surface: LayerSurface,
    width: u16,
    height: u16,

    shm: Shm,
    pool: SlotPool,

    done: bool,
}

impl State {
    fn draw(&mut self, qh: &wayland_client::QueueHandle<Self>) {
        // TODO: use double buffer instead of creating one everytime.
        let (buffer, canvas) = self
            .pool
            .create_buffer(
                self.width.into(),
                self.height.into(),
                (self.width * 4).into(),
                wl_shm::Format::Argb8888,
            )
            .expect("create buffer");

        let mut frame_surface = cairo::ImageSurface::create(
            cairo::Format::ARgb32,
            self.width.into(),
            self.height.into(),
        )
        .expect("cairo image surface");

        {
            let frame_context = cairo::Context::new(&frame_surface).expect("cairo context");

            let r = self.height as f64 / 2.0;
            frame_context.new_sub_path();
            frame_context.arc(r, r, r, PI, 3.0 * PI / 2.0);
            frame_context.arc(self.width as f64 - r, r, r, 3.0 * PI / 2.0, 2.0 * PI);
            frame_context.arc(
                self.width as f64 - r,
                self.height as f64 - r,
                r,
                0.0,
                PI / 2.0,
            );
            frame_context.arc(r, self.height as f64 - r, r, PI / 2.0, PI);
            frame_context.close_path();

            frame_context.set_source_rgba(1.0, 1.0, 1.0, 0.55);
            frame_context.fill().expect("cairo fill");

            frame_context.set_source_rgb(0.0, 0.0, 0.0);
            frame_context.select_font_face(
                "Ubuntu Mono",
                cairo::FontSlant::Normal,
                cairo::FontWeight::Normal,
            );
            frame_context.set_font_size(16.0);

            let now = jiff::Zoned::now().strftime("%H:%M:%S").to_string();
            let extents = frame_context
                .text_extents(&now)
                .expect("cairo text extents");

            frame_context.move_to(
                (self.width / 2) as f64 - (extents.width() / 2.0),
                (self.height / 2) as f64 + (extents.height() / 2.0),
            );
            frame_context.show_text(&now).expect("cairo text");
        }

        let data = frame_surface.data().expect("cairo data");
        canvas.copy_from_slice(&data);

        let surface = self.layer_surface.wl_surface();
        buffer.attach_to(surface).expect("attach");
        surface.damage_buffer(0, 0, self.width.into(), self.height.into());
        surface.frame(&qh, surface.clone());
        surface.commit();
    }
}

// Required for compositor delegation.
impl OutputHandler for State {
    fn output_state(&mut self) -> &mut smithay_client_toolkit::output::OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        println!("wl_output: new_output ({:?})", output.id());
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        println!("wl_output: update_output");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _output: wayland_client::protocol::wl_output::WlOutput,
    ) {
        println!("wl_output: output_destroyed");
    }
}
delegate_output!(State);

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        println!("wl_compositor: scale_factor_changed");
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
        println!("wl_compositor: transform_changed");
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _time: u32,
    ) {
        self.count += 1;
        self.offset = (self.offset + 1) % self.width;
        println!("wl_compositor: frame ({})", self.count);
        self.draw(qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _output: &wayland_client::protocol::wl_output::WlOutput,
    ) {
        println!("wl_compositor: surface_enter");
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _output: &wayland_client::protocol::wl_output::WlOutput,
    ) {
        println!("wl_compositor: surface_leave");
    }
}
delegate_compositor!(State);

impl LayerShellHandler for State {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
    ) {
        println!("wl_layer_shell_handler: closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
        _layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
        _configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        println!("wl_layer_shell_handler: configure");
        self.draw(qh);
    }
}
delegate_layer!(State);

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        println!("wl_shm: shm_state");
        &mut self.shm
    }
}
delegate_shm!(State);

// Required to start the queue and keep the globals up to date.
impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!();
}
delegate_registry!(State);
