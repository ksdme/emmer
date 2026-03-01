use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_registry, delegate_shm, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        WaylandSurface,
        xdg::{
            self, XdgShell, XdgSurface,
            window::{Window, WindowDecorations},
        },
    },
    shm::{
        Shm, ShmHandler,
        slot::{Buffer, SlotPool},
    },
};
use wayland_client::{Connection, Proxy, globals::registry_queue_init, protocol::wl_shm};

fn main() {
    let conn = Connection::connect_to_env().expect("wl connection");

    let (globals, mut event_queue) = registry_queue_init::<State>(&conn).expect("wl queue init");
    let q_handle = event_queue.handle();

    let output_state = OutputState::new(&globals, &q_handle);
    let registry_state = RegistryState::new(&globals);

    let compositor_state = CompositorState::bind(&globals, &q_handle).expect("wl compositor state");
    let xdg_shell = XdgShell::bind(&globals, &q_handle).expect("xdgshell");

    let surface = compositor_state.create_surface(&q_handle);
    let window = xdg_shell.create_window(surface, WindowDecorations::ServerDefault, &q_handle);

    window.commit();

    let shm = Shm::bind(&globals, &q_handle).expect("wl_shm");
    let pool = SlotPool::new(256 * 256, &shm).expect("wl_shm_pool");

    let mut state = State {
        count: 0,
        offset: 0,

        registry_state,
        output_state,

        window,
        width: 0,
        height: 0,

        shm,
        pool,
        buffer: None,
    };
    loop {
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

    window: Window,
    width: u16,
    height: u16,

    shm: Shm,
    pool: SlotPool,
    buffer: Option<Buffer>,
}

impl State {
    fn draw(&mut self, qh: &wayland_client::QueueHandle<Self>) {
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
            frame_context.set_source_rgb(1.0, 1.0, 1.0);
            frame_context.paint().expect("cairo background paint");

            frame_context.set_source_rgb(0.0, 0.0, 0.0);
            frame_context.move_to(self.offset.into(), 0.0);
            frame_context.line_to((self.width - self.offset).into(), (self.height - 1).into());

            frame_context.set_line_width(2.0);
            frame_context.stroke().expect("cairgo stroke");
        }

        let data = frame_surface.data().expect("cairo data");
        canvas.copy_from_slice(&data);

        let surface = self.window.wl_surface();
        buffer.attach_to(surface).expect("attach");
        surface.damage_buffer(0, 0, self.width.into(), self.height.into());
        surface.frame(qh, surface.clone());
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

impl xdg::window::WindowHandler for State {
    fn request_close(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _window: &xdg::window::Window,
    ) {
        println!("xdgshell: request_close");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
        window: &xdg::window::Window,
        _configure: xdg::window::WindowConfigure,
        _serial: u32,
    ) {
        println!("xdgshell: configure");

        window.set_window_geometry(0, 0, 256, 256);
        window.set_app_id("xyz.ksdme.wl");
        self.width = 256;
        self.height = 256;

        self.draw(qh);
    }
}
delegate_xdg_shell!(State);
delegate_xdg_window!(State);

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
