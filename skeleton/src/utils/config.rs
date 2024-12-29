use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use std::{error::Error, fs, path::Path};
use tokio::sync::mpsc;

use toml;

pub fn read_toml<T, U>(path: T) -> Result<U, Box<dyn Error>>
where
    T: AsRef<Path>,
    U: serde::de::DeserializeOwned,
{
    let contents = fs::read_to_string(path)?;
    let config = toml::from_str(&contents)?;
    Ok(config)
}

pub async fn watch_config<T>(
    path: T,
    sender: mpsc::UnboundedSender<impl Clone>,
) -> Result<(), notify::Error>
where
    T: AsRef<Path> + Clone + Send + 'static,
{
  Ok(())
}
