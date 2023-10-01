use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use config::{ConfigBuilder, File};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::CONFIGS;

#[derive(Debug)]
pub struct ConfigWatcher {
    config_dir: PathBuf,
}

impl ConfigWatcher {
    pub fn new(config_dir: PathBuf) -> Self {
        ConfigWatcher { config_dir }
    }

    pub fn load_configs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut configs = CONFIGS.lock().unwrap();
        configs.clear();

        // List all files in the config directory
        let entries = std::fs::read_dir(&self.config_dir)?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if ext_str == "json"
                            || ext_str == "toml"
                            || ext_str == "yaml"
                            || ext_str == "yml"
                        {
                            let config_builder: ConfigBuilder<config::builder::DefaultState> =
                                ConfigBuilder::default();
                            let config = config_builder
                                .add_source(File::from(self.config_dir.join("default.yaml")))
                                .add_source(File::from(path.clone()))
                                .build()?;
                            let config_name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or_default()
                                .to_string();
                            configs.insert(config_name, config);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn watch_config_dir(
        &mut self,
        tx: Sender<Result<Event, notify::Error>>,
        rx: Receiver<Result<Event, notify::Error>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut watcher: RecommendedWatcher = Watcher::new(
            tx,
            notify::Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;
        watcher.watch(&self.config_dir, RecursiveMode::NonRecursive)?;

        loop {
            match rx.recv() {
                Ok(event) => {
                    if let Ok(Event {
                        kind: EventKind::Modify(_),
                        ..
                    }) = event
                    {
                        // A file in the config directory has changed; reload configs
                        self.load_configs()?;
                        println!("Configurations reloaded.");
                    }
                }
                Err(e) => println!("Watcher error: {:?}", e),
            }
        }
    }
}
