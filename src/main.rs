extern crate core;
pub mod monitor;
pub mod database;
pub mod executor;
pub mod common;

use std::path::PathBuf;
use std::fs;
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use crate::common::Config;
use crate::database::Database;
use crate::executor::Executor;
use crate::monitor::Monitor;
/*
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    name: Option<String>,
    prerelease: bool,
    published_at: String,
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let url =
        "https://api.github.com/repos/netbirdio/netbird/releases/latest";

    let client = reqwest::Client::new();

    let release = client
        .get(url)
        .header("User-Agent", "release-monitor")
        .send()
        .await?
        .json::<Release>()
        .await?;

    println!("Latest version: {}", release.tag_name);
    println!("Prerelease: {}", release.prerelease);
    println!("Published: {}", release.published_at);

    Ok(())
}
*/

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    config_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let config = Result::<Config, Box<dyn std::error::Error>>::Ok(Config {
        refresh_interval: 3600,
        repositories: Vec::new(),
    });

    match args.config_file {
        Some(config_file) => {
            let config = load_config(PathBuf::from(config_file))?;
            let database = Database::new(PathBuf::from("repositories.db"), config.clone())?;
            let mut monitor = Monitor::new(config, database, true);
            monitor.run().await.expect("[ERROR] Monitor failed to run");
            Ok(())
        }
        None => {
            let config = load_config(PathBuf::from("release_monitor_config.yaml"))?;
            let database = Database::new(PathBuf::from("repositories.db"), config.clone())?;
            let mut monitor = Monitor::new(config, database, true);
            monitor.run().await.expect("[ERROR] Monitor failed to run");
            Ok(())
        }
    }
}

fn load_config(path: PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(config)
}