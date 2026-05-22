use std::{
    future::pending,
    sync::{self, atomic::AtomicU32},
    thread,
};

use anyhow::{Context, Result, anyhow};
use log::{debug, error, warn};
use smithay_client_toolkit::reexports::{
    calloop::{
        self, EventLoop,
        channel::{self, channel},
    },
    calloop_wayland_source::WaylandSource,
};
use wayland_client::Connection;
use zbus::connection;

use crate::{
    dbus::NotificationService,
    ui::app::{App, UIMessage},
};

mod config;
mod dbus;
mod notification;
mod ui;

pub enum ServerMessage {
    Dismiss(u32),
}

fn main() -> Result<()> {
    env_logger::init();

    let (main_tx, main_rx) = std::sync::mpsc::channel::<Result<()>>();
    let (ui_tx, ui_rx) = calloop::channel::channel::<UIMessage>();

    // Start the dbus service in a separate thread with tokio.
    let service_thread = thread::spawn(move || {
        let task = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Could not start a thread")
            .and_then(|rt| rt.block_on(run_service(&main_tx, ui_tx)))
            .context("Could not block on service");

        let _ = main_tx.send(task);
    });

    // Wait for the service to start.
    main_rx
        .recv()
        .context("Could not wait on service startup")?
        .context("Could not start notification dbus service")?;

    // Start the UI on the main thread.
    run_ui(ui_rx).context("Could not run ui")?;

    service_thread
        .join()
        .map_err(|err| anyhow!("Could not join service thread: {err:?}"))?;

    Ok(())
}

// Start the UI loop.
fn run_ui(rx: channel::Channel<UIMessage>) -> Result<()> {
    let conn = Connection::connect_to_env().context("Could not connect to wayland")?;
    let (mut app, event_queue) =
        ui::app::App::init(&conn).context("Could not initialize wayland client")?;

    let mut main_loop = EventLoop::<App>::try_new().context("Could not initialize main loop")?;

    // Add the Wayland events onto the loop.
    let source = WaylandSource::new(conn, event_queue);
    source
        .insert(main_loop.handle())
        .context("Could not insert wayland events")?;

    // Wire the external events onto the loop.
    main_loop
        .handle()
        .insert_source(rx, move |event, _, app| match event {
            channel::Event::Msg(msg) => {
                debug!(target: "uiloop", "received event {msg:?}");
                if let Err(err) = app.handle(msg).context("Could not process event") {
                    error!(target: "uiloop", "could not process event: {err}");
                }
            }
            channel::Event::Closed => {
                warn!(target: "uiloop", "channel closed");
            }
        })
        .map_err(|err| anyhow!("Could not insert external events channel: {err}"))?;

    main_loop
        .run(None, &mut app, |_| {})
        .context("Could not run the main loop")?;

    Ok(())
}

// Starts the dbus service loop.
async fn run_service(
    main_tx: &std::sync::mpsc::Sender<Result<()>>,
    ui_tx: channel::Sender<UIMessage>,
) -> Result<()> {
    let service = NotificationService::new(ui_tx);
    let _conn = connection::Builder::session()?
        .name("org.freedesktop.Notifications")?
        .serve_at("/org/freedesktop/Notifications", service)?
        .build()
        .await
        .context("Could not setup connection")?;

    // zbus queues listener and events onto the runtime itself. It does not
    // require to block the thread on an .await while the events are being
    // processed(?). So, inform the main thread that the connection was setup.
    let _ = main_tx.send(Ok(()));

    pending::<()>().await;
    Ok(())
}
