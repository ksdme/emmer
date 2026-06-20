use std::{thread, time::Duration};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use smithay_client_toolkit::reexports::{
    calloop::{self, EventLoop, channel},
    calloop_wayland_source::WaylandSource,
};
use wayland_client::Connection;
use zbus::{connection, object_server::SignalEmitter};

use crate::{
    config::{ComputedConfig, Config, Insets, SpreadConfig, StackConfig, ThemeConfig},
    dbus::{NotificationService, ServerMessage},
    ui::app::{App, UIMessage},
};

mod config;
mod dbus;
mod notification;
mod ui;
mod utils;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Run the app in debug mode
    #[cfg(debug_assertions)]
    #[arg(long, default_value_t = false)]
    debug_mode: bool,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let (startup_tx, startup_rx) = std::sync::mpsc::channel::<Result<()>>();
    let (ui_tx, ui_rx) = calloop::channel::channel::<UIMessage>();
    let (server_tx, mut server_rx) = tokio::sync::mpsc::unbounded_channel::<ServerMessage>();

    // Start the dbus service in a separate thread with tokio.
    let service_thread = thread::spawn(move || {
        let task = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Could not start a thread")
            .and_then(|rt| rt.block_on(run_server(&startup_tx, ui_tx, &mut server_rx)))
            .context("Could not block on service");

        let _ = startup_tx.send(task);
    });

    // Wait for the service to start.
    startup_rx
        .recv()
        .context("Could not wait on service startup")?
        .context("Could not start notification dbus service")?;

    // Start the UI on the main thread.
    run_ui(&args, ui_rx, server_tx).context("Could not run ui")?;

    service_thread
        .join()
        .map_err(|err| anyhow!("Could not join service thread: {err:?}"))?;

    Ok(())
}

// Start the UI loop.
fn run_ui(
    args: &Args,
    ui_rx: channel::Channel<UIMessage>,
    server_tx: tokio::sync::mpsc::UnboundedSender<ServerMessage>,
) -> Result<()> {
    let config = ComputedConfig::new(
        args.debug_mode,
        Config {
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
                title_font_description: "JetBrainsMono Nerd Font Mono Bold 10".to_string(),
                body_font_description: "JetBrainsMono Nerd Font Mono 10".to_string(),
            },
            width: 320.,
        },
    );

    let conn = Connection::connect_to_env().context("Could not connect to wayland")?;
    let (mut app, event_queue) = ui::app::App::init(config, &conn, server_tx)
        .context("Could not initialize wayland client")?;

    let mut main_loop = EventLoop::<App>::try_new().context("Could not initialize main loop")?;

    // Add the Wayland events onto the loop.
    let source = WaylandSource::new(conn, event_queue);
    source
        .insert(main_loop.handle())
        .context("Could not insert wayland events")?;

    // Wire the external events onto the loop.
    main_loop
        .handle()
        .insert_source(ui_rx, move |event, _, app| match event {
            channel::Event::Msg(msg) => {
                log::debug!("received event: {msg:?}");
                let _ = logged!(app.handle(msg).context("Could not process event"));
            }
            channel::Event::Closed => {
                log::warn!("channel closed");
            }
        })
        .map_err(|err| anyhow!("Could not insert external events channel: {err}"))?;

    main_loop
        .handle()
        .insert_source(
            calloop::timer::Timer::from_duration(Duration::from_secs(1)),
            |_, _, app| {
                let _ = logged!(
                    app.dismiss_expired()
                        .context("Could not dismiss expired events")
                );
                calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(1))
            },
        )
        .map_err(|err| anyhow!("Could not insert timer event source: {err}"))?;

    main_loop
        .run(None, &mut app, |_| {})
        .context("Could not run the main loop")?;

    Ok(())
}

// Starts the dbus service loop.
async fn run_server(
    startup_tx: &std::sync::mpsc::Sender<Result<()>>,
    ui_tx: channel::Sender<UIMessage>,
    server_rx: &mut tokio::sync::mpsc::UnboundedReceiver<ServerMessage>,
) -> Result<()> {
    let service = NotificationService::new(ui_tx);

    let conn = connection::Builder::session()?
        .name("org.freedesktop.Notifications")?
        .serve_at("/org/freedesktop/Notifications", service)?
        .build()
        .await
        .context("Could not setup connection")?;

    let signal_emitter = SignalEmitter::new(&conn, "/org/freedesktop/Notifications")
        .context("Could not create signal emitter")?;

    // zbus queues listener and events onto the runtime itself. It does not
    // require to block the thread on an .await while the events are being
    // processed(?). So, inform the main thread that the connection was setup.
    let _ = startup_tx.send(Ok(()));

    while let Some(msg) = server_rx.recv().await {
        match msg {
            ServerMessage::Closed { id, reason } => {
                // https://specifications.freedesktop.org/notification/latest/protocol.html#id-1.10.4.2.4
                let _ = logged!(
                    NotificationService::notification_closed(
                        &signal_emitter,
                        id,
                        u32::from(reason),
                    )
                    .await
                    .context("Could not send notification closed message: {id}")
                );
            }
            ServerMessage::ActivationToken { id, token } => {
                let _ = logged!(
                    NotificationService::activation_token(&signal_emitter, id, token)
                        .await
                        .context("Could not send activation token to: {id}")
                );
            }
            ServerMessage::ActionInvoked { id, key } => {
                let _ = logged!(
                    NotificationService::action_invoked(&signal_emitter, id, key)
                        .await
                        .context("Could not send action invocation to: {id}")
                );
            }
        }
    }

    log::info!("dbus server ended");
    Ok(())
}
