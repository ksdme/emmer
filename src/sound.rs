use std::{
    env,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

/// Finds a named sound in the freedesktop sound theme.
/// https://specifications.freedesktop.org/sound-naming/latest/
pub fn find(name: &str) -> Option<PathBuf> {
    let dirs = env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());

    dirs.split(':')
        .flat_map(|dir| {
            ["oga", "ogg", "wav"].map(|extension| {
                Path::new(dir).join(format!("sounds/freedesktop/stereo/{name}.{extension}"))
            })
        })
        .find(|path| path.is_file())
}

/// Plays the sound file to completion. This blocks, so, it should be run off the
/// UI thread. The audio device is only held open while the sound plays.
pub fn play(path: &Path) -> Result<()> {
    let mut device =
        rodio::DeviceSinkBuilder::open_default_sink().context("Could not open audio device")?;
    device.log_on_drop(false);

    let file = File::open(path).context("Could not open sound file")?;
    rodio::play(device.mixer(), BufReader::new(file))
        .context("Could not play sound file")?
        .sleep_until_end();

    Ok(())
}
