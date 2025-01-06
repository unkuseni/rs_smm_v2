use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use std::{error::Error, fs, path::Path};
use tokio::sync::mpsc;

use toml;

    /// Reads a configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// If the file does not exist or cannot be read, or if the TOML is
    /// invalid, an error is returned.
pub fn read_toml<T, U>(path: T) -> Result<U, Box<dyn Error>>
where
    T: AsRef<Path>,
    U: serde::de::DeserializeOwned,
{
    let contents = fs::read_to_string(path)?;
    let config = toml::from_str(&contents)?;
    Ok(config)
}

    /// Watches a configuration file for changes and sends the updated
    /// configuration over the given channel.
    ///
    /// This function will first read the configuration from the given file
    /// and send it over the channel. It will then start watching the file
    /// for changes. When a change is detected, it will read the file again
    /// and send the new configuration over the channel.
    ///
    /// If there is an error reading the file, it will print an error message
    /// and continue watching the file.
    ///
    /// The function will return an error if there is a problem setting up the
    /// file watcher.
    ///
    /// This function is intended to be used with a `tokio::spawn` call, as it
    /// will block until the file watcher is stopped.
    ///
    /// # Examples
    ///
    /// 
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
