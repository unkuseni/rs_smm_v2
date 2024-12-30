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

pub async fn watch_config<T, U>(
    path: T,
    sender: mpsc::UnboundedSender<U>,
) -> Result<(), Box<dyn Error>>
where
    T: AsRef<Path> + Clone + Send + 'static,
    U: serde::de::DeserializeOwned + Send + 'static,
{
    match read_toml::<T, U>(path.clone()) {
        Ok(config) => {
            sender.send(config)?;
        }
        Err(e) => {
            println!("Error reading config file: {}", e);
        }
    }

    // Channel for file system events
    let (fs_tx, mut fs_rx) = tokio::sync::mpsc::channel(1);

    // Create watcher
    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            if let Ok(event) = result {
                if matches!(event.kind, EventKind::Modify(_)) {
                    let _ = fs_tx.blocking_send(());
                }
            }
        },
        notify::Config::default(),
    )?;

    // Start watching the file
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    while let Some(_) = fs_rx.recv().await {
        match read_toml(path.clone()) {
            Ok(new_config) => {
                if let Err(e) = sender.send(new_config) {
                    println!("Failed to send updated config: {}", e);
                } else {
                    println!("Config successfully updated");
                }
            }
            Err(e) => println!("Failed to read updated config: {}", e),
        }
    }

    Ok(())
}
