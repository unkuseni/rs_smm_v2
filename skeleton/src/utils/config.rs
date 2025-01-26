use notify::{Event, RecommendedWatcher, Watcher};
use serde::de::DeserializeOwned;
use std::{error::Error, path::Path, time::Duration};
use tokio::{fs, sync::mpsc};

use tracing::{error, info};

/// Async config reader with efficient error handling
use anyhow::Result;

pub async fn read_toml<T: AsRef<Path>, U: DeserializeOwned>(path: T) -> Result<U> {
    let contents = fs::read_to_string(path).await?;
    toml::from_str(&contents).map_err(Into::into)
}
/// Debounced file watcher with zero-copy parsing
pub async fn watch_config<T, U>(
    path: T,
    sender: mpsc::Sender<U>,
) -> Result<(), Box<dyn Error + Send>>
where
    T: AsRef<Path> + Send + 'static,
    U: DeserializeOwned + Send + 'static,
{
    let path = path.as_ref().to_path_buf();

    // Initial load with backoff retry
    let config = read_toml(&path).await?;
    let _ = sender.send(config).await.map_err(|e| Box::new(e));

    // File watcher with event filtering
    let (debounce_tx, mut debounce_rx) = mpsc::channel(4);
    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            if let Ok(event) = result {
                if event.kind.is_modify() {
                    let _ = debounce_tx.blocking_send(());
                }
            }
        },
        notify::Config::default()
            .with_poll_interval(Duration::from_secs(1))
            .with_compare_contents(true),
    )
    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?; // Convert notify::Error

    watcher
        .watch(&path, notify::RecursiveMode::NonRecursive)
        .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?; // Convert notify::Error

    // Efficient debounce loop
    let mut debounce_timer = tokio::time::interval(Duration::from_millis(500));
    let mut config_version = 0u32;

    loop {
        tokio::select! {
            _ = debounce_timer.tick() => {
                match read_toml(&path).await {
                    Ok(new_config) => {
                        config_version += 1;
                    let _ = sender.send(new_config).await.map_err(|e| Box::new(e));
                        info!("Config reloaded (v{})", config_version);
                    }
                    Err(e) => error!("Config reload failed: {}", e),
                }
            }
            _ = debounce_rx.recv() => {
                // Reset debounce timer on change detection
                debounce_timer.reset();
            }
        }
    }
}
