use anyhow::{Context, Result};
use wayland_client::Connection;

mod config;
mod engine;
mod renderer;
mod ui;

fn main() -> Result<()> {
    env_logger::init();

    let wayland_conn = Connection::connect_to_env().context("Could not connect to wayland")?;

    let (mut app, mut event_queue) =
        ui::app::App::init(&wayland_conn).context("Could not initialize wayland client")?;

    loop {
        event_queue
            .blocking_dispatch(&mut app)
            .context("Could not process event")?;
    }
}
