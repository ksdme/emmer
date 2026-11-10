use std::{path::PathBuf, thread, time::Duration};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use miette::WrapErr;
use smithay_client_toolkit::reexports::{
    calloop::{self, EventLoop, channel},
    calloop_wayland_source::WaylandSource,
};
use wayland_client::Connection;
use zbus::{connection, object_server::SignalEmitter};

use crate::{
    config::{ComputedConfig, Config, TimeoutsConfig},
    dbus::service::{NotificationService, ServerMessage},
    ui::app::{App, UIMessage},
};

mod config;
mod dbus;
mod sound;
mod ui;
mod utils;

#[derive(clap::Parser)]
#[command(version, about, long_about = None, args_conflicts_with_subcommands = true)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// The path to the config file. Defaults to $XDG_CONFIG_HOME/emmer/config.kdl
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Run the app in debug mode
    #[cfg(debug_assertions)]
    #[arg(long, default_value_t = false)]
    debug_mode: bool,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Print the default config
    DefaultConfig,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    if let Some(Command::DefaultConfig) = args.command {
        print!("{}", Config::default().to_kdl());
        return Ok(());
    }

    let config = match load_config(&args) {
        Ok(config) => config,
        Err(report) => {
            eprintln!("{report:?}");
            std::process::exit(1);
        }
    };

    let (startup_tx, startup_rx) = std::sync::mpsc::channel::<Result<()>>();
    let (ui_tx, ui_rx) = calloop::channel::channel::<UIMessage>();
    let (server_tx, mut server_rx) = tokio::sync::mpsc::unbounded_channel::<ServerMessage>();

    // Start the dbus service in a separate thread with tokio.
    let timeouts = config.global.timeouts.clone();
    let service_thread = thread::spawn(move || {
        let task = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Could not start a thread")
            .and_then(|rt| {
                rt.block_on(run_server(
                    rt.handle().clone(),
                    &startup_tx,
                    ui_tx,
                    &mut server_rx,
                    timeouts,
                ))
            })
            .context("Could not block on service");

        let _ = startup_tx.send(task);
    });

    // Wait for the service to start.
    startup_rx
        .recv()
        .context("Could not wait on service startup")?
        .context("Could not start notification dbus service")?;

    // Start the UI on the main thread.
    run_ui(&args, config, ui_rx, server_tx).context("Could not run ui")?;

    service_thread
        .join()
        .map_err(|err| anyhow!("Could not join service thread: {err:?}"))?;

    Ok(())
}

// Reads the config file, falling back to the defaults if the default config file
// does not exist.
fn load_config(args: &Args) -> miette::Result<Config> {
    let path = match &args.config {
        Some(path) => path.clone(),
        None => match config::default_path() {
            Some(path) if path.exists() => path,
            _ => return Ok(Config::default()),
        },
    };

    log::info!("loading config from {}", path.display());
    Config::load(&path).wrap_err_with(|| format!("Could not load config: {}", path.display()))
}

// Start the UI loop.
fn run_ui(
    #[cfg_attr(not(debug_assertions), allow(unused_variables))] args: &Args,
    config: Config,
    ui_rx: channel::Channel<UIMessage>,
    server_tx: tokio::sync::mpsc::UnboundedSender<ServerMessage>,
) -> Result<()> {
    #[cfg(debug_assertions)]
    let debug_mode = args.debug_mode;
    #[cfg(not(debug_assertions))]
    let debug_mode = false;

    let config = ComputedConfig::new(debug_mode, config);

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
    rt: tokio::runtime::Handle,
    startup_tx: &std::sync::mpsc::Sender<Result<()>>,
    ui_tx: channel::Sender<UIMessage>,
    server_rx: &mut tokio::sync::mpsc::UnboundedReceiver<ServerMessage>,
    timeouts: TimeoutsConfig,
) -> Result<()> {
    let service = NotificationService::new(rt, ui_tx, timeouts);

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
