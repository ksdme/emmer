use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use smithay_client_toolkit::{
    activation::{ActivationHandler, ActivationState, RequestData},
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_activation, delegate_compositor, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{BTN_LEFT, BTN_RIGHT, CursorIcon, PointerHandler, ThemeSpec, ThemedPointer},
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
    protocol::{
        wl_seat::{self, WlSeat},
        wl_shm, wl_surface,
    },
};

use crate::{
    config::ComputedConfig,
    dbus::{CloseReason, ServerMessage},
    logged, notification,
    ui::{
        activation::ActivationRequestData,
        buffers::BufferPool,
        items::{Stack, stack::AppCommand},
    },
};

/// The top level Wayland client.
pub struct App {
    server_tx: tokio::sync::mpsc::UnboundedSender<ServerMessage>,

    conn: Connection,
    queue_handle: QueueHandle<Self>,

    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,

    layer_surface: LayerSurface,
    seat_state: SeatState,

    seat: Option<WlSeat>,
    pointer: Option<ThemedPointer>,
    cursor_icon: Option<CursorIcon>,
    activation_state: ActivationState,

    shm: Shm,
    buffer_pool: Mutex<BufferPool<3>>,

    config: Arc<ComputedConfig>,

    width: i32,
    height: i32,

    stack: Stack,
    stack_hovering: bool,
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
        qh: &wayland_client::QueueHandle<Self>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
        configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        log::debug!(target: "emmer::wl::layer_shell", "configure: {configure:?}");

        let (w, h) = configure.new_size;
        self.width = w as i32;
        self.height = h as i32;

        // Buffers should be recreated on the frame callback.
        let surface = layer.wl_surface();
        surface.frame(qh, surface.clone());
        surface.commit();
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
            let hit = self.stack.bounds().is_some_and(|b| b.contains(e.position));

            if hit {
                match e.kind {
                    smithay_client_toolkit::seat::pointer::PointerEventKind::Release {
                        time: _,
                        button,
                        serial,
                    } => {
                        log::trace!(target: "emmer::wl::pointer", "frame release");
                        self.stack_hovering = true;

                        let commands = match button {
                            BTN_LEFT => self.stack.on_left_click(e),
                            BTN_RIGHT => self.stack.on_right_click(e),
                            _ => continue,
                        };

                        if let Some(seat) = self.seat.as_ref() {
                            let _ = self.handle_commands(
                                commands,
                                Some((serial, seat.clone(), e.surface.clone())),
                            );
                        }
                    }
                    smithay_client_toolkit::seat::pointer::PointerEventKind::Motion { time: _ } => {
                        log::trace!(target: "emmer::wl::pointer", "motion");
                        self.stack_hovering = true;

                        let commands = self.stack.on_hover(e);
                        let _ = self.handle_commands(commands, None);
                    }
                    _ => {}
                }
            }

            if self.stack_hovering && !hit {
                self.stack_hovering = false;

                // TODO: We should check the type of the event?
                let commands = self.stack.on_leave();
                let _ = self.handle_commands(commands, None);
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

        // It doesn't make too much sense for this application to support multiple users,
        // or... does it?
        if let Some(ref current) = self.seat {
            log::warn!(target: "emmer::wl::seat", "ignoring seat: {seat:?} for: {current:?}")
        } else {
            self.seat = Some(seat);
        }
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
            let current = self.seat.get_or_insert(seat.clone());
            if current == &seat {
                let pointer = logged!(
                    self.seat_state
                        .get_pointer_with_theme(
                            qh,
                            &seat,
                            self.shm.wl_shm(),
                            self.layer_surface.wl_surface().clone(),
                            ThemeSpec::default(),
                        )
                        .context("Could not get pointer with theme")
                );

                if let Ok(pointer) = pointer {
                    self.pointer = Some(pointer);
                }
            }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        log::debug!(target: "emmer::wl::seat", "remove_capability: {capability:?}");

        if capability == Capability::Pointer && self.seat.as_ref() == Some(&seat) {
            self.pointer = None;
        }
    }

    fn remove_seat(
        &mut self,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        seat: wl_seat::WlSeat,
    ) {
        log::debug!(target: "emmer::wl::seat", "remove_seat: {seat:?}");

        if self.seat.as_ref() == Some(&seat) {
            self.seat = None;
        }
    }
}
delegate_seat!(App);

impl ActivationHandler for App {
    type RequestData = ActivationRequestData;

    fn new_token(&mut self, token: String, req: &Self::RequestData) {
        log::debug!(target: "emmer::wl::activation", "new_token: {req:?}");

        let _ = logged!(
            self.server_tx
                .send(ServerMessage::ActivationToken {
                    id: req.id(),
                    token
                })
                .context("Could not send activation token message")
        );

        if let Some(action) = req.action() {
            let _ = logged!(
                self.server_tx
                    .send(ServerMessage::ActionInvoked {
                        id: req.id(),
                        key: action.to_string(),
                    })
                    .context("Could not send action message")
            );

            let _ = logged!(
                self.server_tx
                    .send(ServerMessage::Closed {
                        id: req.id(),
                        reason: CloseReason::Manual,
                    })
                    .context("Could not send action message")
            );
        }
    }
}
delegate_activation!(App, ActivationRequestData);

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
    pub fn init(
        config: ComputedConfig,
        conn: &Connection,
        server_tx: tokio::sync::mpsc::UnboundedSender<ServerMessage>,
    ) -> Result<(Self, EventQueue<Self>)> {
        let config = Arc::new(config);

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

        let activation_state = ActivationState::bind(&globals, &q_handle)
            .context("Could not bind activation state")?;

        let seat_state = SeatState::new(&globals, &q_handle);
        let shm = Shm::bind(&globals, &q_handle).context("Could not bind shm")?;

        let buffer_pool = BufferPool::<3>::new(&shm).context("Could not initialize buffer pool")?;

        Ok((
            App {
                server_tx,

                conn: conn.clone(),
                queue_handle: q_handle,

                registry_state,
                output_state,
                compositor_state,

                layer_surface,
                seat_state,

                seat: None,
                pointer: None,
                cursor_icon: None,
                activation_state,

                shm,
                buffer_pool: Mutex::new(buffer_pool),

                config: config.clone(),

                width: 0,
                height: 0,

                stack: Stack::new(config),
                stack_hovering: false,
            },
            event_queue,
        ))
    }
}

impl App {
    pub fn draw(&mut self) -> Result<()> {
        let wl_surface = self.layer_surface.wl_surface();

        let mut buffer_pool = self
            .buffer_pool
            .lock()
            .map_err(|err| anyhow!("Could not lock buffer pool: {err:?}"))?;

        // Try acquiring a free buffer and if we can't find one,
        // queue another frame callback and skip this frame.
        let (frame_buffer, mem) = match buffer_pool
            .get(
                self.width as u32,
                self.width as u32 * 4,
                self.height as u32,
                wl_shm::Format::Argb8888,
            )
            .context("Could not get a buffer")?
            .context("Could not find a buffer")
        {
            Err(err) => {
                wl_surface.frame(&self.queue_handle, wl_surface.clone());
                wl_surface.commit();
                return Err(err);
            }
            Ok(buff) => buff,
        };

        // The buffer might have data left from a previous render. Clearing it via
        // cairo operations is often more involved. So, instead, we can just reset the mem.
        mem.fill(0);

        // Render to a cairo surface.
        //
        // The unsafe here is acceptable because,
        // 1. The cairo surface/context does not leave this method.
        // 2. The method holds a lock on the buffer pool, so no changes can happen there.
        // 3. SCTK Slots are reference counted, so even if the next frame drops the buffers,
        //    they will be around until Wayland releases them.
        //
        // Without the unsafe, we will end up having to first draw to a cairo surface and
        // then read out the entire buffer into Wayland SHM buffer.
        let surface = unsafe {
            cairo::ImageSurface::create_for_data_unsafe(
                mem.as_mut_ptr(),
                cairo::Format::ARgb32,
                self.width,
                self.height,
                self.width * 4,
            )
        }
        .context("Could not create cairo surface")?;
        let cx = cairo::Context::new(&surface).context("Could not create cairo context")?;

        let (bounds, settled) = self.stack.render(&cx).context("Could not render stack")?;

        // Update the input region.
        if let Some(bounds) = bounds {
            let (x, y) = (bounds.x1 as i32 - 8, bounds.y1 as i32 - 8);
            let (w, h) = (
                bounds.w() as i32 + 16,
                self.height.min(bounds.h() as i32 + 16),
            );

            let region = Region::new(&self.compositor_state).context("Could not create region")?;
            region.add(x, y, w, h);
            wl_surface.set_input_region(Some(region.wl_region()));

            #[cfg(debug_assertions)]
            if self.config.debug_mode {
                cx.new_path();
                cx.set_source_rgba(0., 255., 0., 0.5);
                cx.rectangle(x as f64, y as f64, w as f64, h as f64);
                let _ = cx.stroke();
            }
        }

        // Request an update to the frame.
        surface.flush();
        frame_buffer
            .attach_to(wl_surface)
            .context("Could not attach buffer")?;

        wl_surface.damage_buffer(0, 0, self.width, self.height);
        if !settled {
            wl_surface.frame(&self.queue_handle, wl_surface.clone());
        }

        wl_surface.commit();

        Ok(())
    }

    pub fn push(&mut self, notification: notification::Notification) -> Result<()> {
        let commands = self.stack.push(notification);
        self.handle_commands(commands, None)
    }

    pub fn dismiss_expired(&mut self) -> Result<()> {
        let commands = self.stack.dismiss_expired();
        self.handle_commands(commands, None)
    }
}

impl App {
    pub fn handle_commands(
        &mut self,
        commands: Vec<AppCommand>,
        serial_seat_surface: Option<(u32, wl_seat::WlSeat, wl_surface::WlSurface)>,
    ) -> Result<()> {
        for c in commands {
            match c {
                AppCommand::Redraw => {
                    let _ = logged!(self.draw().context("Could not process draw command"));
                }
                AppCommand::SetCursor(cursor_icon) => {
                    if let Some(pointer) = self.pointer.as_ref()
                        && self.cursor_icon != Some(cursor_icon)
                    {
                        let _ = logged!(
                            pointer
                                .set_cursor(&self.conn, cursor_icon)
                                .context("Could not update the cursor")
                        );
                        self.cursor_icon = Some(cursor_icon);
                    }
                }
                AppCommand::NotifyClosed(id, close_reason) => {
                    let _ = logged!(
                        self.server_tx
                            .send(ServerMessage::Closed {
                                id,
                                reason: close_reason,
                            })
                            .context("Could not send closed signal")
                    );
                }
                AppCommand::NotifyAction(id, key) => {
                    if let Some((serial, seat, surface)) = &serial_seat_surface {
                        let req = ActivationRequestData::new(
                            id,
                            Some(key),
                            RequestData {
                                app_id: None,
                                seat_and_serial: Some((seat.clone(), *serial)),
                                surface: Some(surface.clone()),
                            },
                        );

                        self.activation_state
                            .request_token_with_data(&self.queue_handle, req);
                    }
                }
            }
        }

        Ok(())
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
