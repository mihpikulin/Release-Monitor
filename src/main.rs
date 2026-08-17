extern crate core;
pub mod monitor;
pub mod database;
pub mod executor;
pub mod common;

use crate::common::{Config, RepositoryConfig};
use crate::database::Database;
use crate::monitor::Monitor;
use clap::Parser;
use comfy_table::{ContentArrangement, Table};
use serde_yaml::{self};
use std::path::PathBuf;
use std::process::Command;
use std::fs;

static LONG_ABOUT: &str = "Release monitor for GitHub repositories. \n\
                            This tool monitors specified GitHub repositories \n\
                            for new releases and executes a command when a new release is detected. \n\
                            The configuration is provided via a YAML file, which includes the \n\
                            list of repositories to monitor, the refresh interval, \n\
                            and the command to execute on new releases.";

static SHORT_ABOUT: &str = "Release monitor for GitHub repositories";
#[derive(Parser, Debug)]
#[command(author, version, about = SHORT_ABOUT, long_about = Some(LONG_ABOUT))]
struct Cli {
    #[arg(help = "Path to custom config", value_name = "PATH", short, long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    #[command(about = "Starts monitor service")]
    Start,

    #[command(about = "Stops monitor service")]
    Stop,

    #[command(about = "Restarts monitor service")]
    Restart,

    #[command(about = "Prints status about the monitor")]
    Status,

    #[command(about = "Runs the monitor in current session")]
    Run,

    #[command(about = "Add repository to monitor. (Can be also done in /etc/release-monitor/config.yaml)")]
    Add {
        #[arg(long)]
        name: Option<String>,

        #[arg(long)]
        owner: Option<String>,

        #[arg(long)]
        on_release: Option<String>,
    },

    #[command(about = "Remove repository from monitor. (Can be also done in /etc/release-monitor/config.yaml)")]
    Remove {
        #[arg(long)]
        name: Option<String>,

        #[arg(long)]
        owner: Option<String>,
    }
}

#[derive(Debug)]
struct ServiceStatus {
    active_state: String,
    sub_state: String,
    main_pid: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let mut monitor: Monitor;
    let database: Database;
    let config: Config;

    const SYSTEMD_SERVICE: &str = "release-monitor.service";
    const CONFIG_PATH: &str = "/etc/release-monitor/config.yaml";
    const DB_PATH: &str = "/var/lib/release-monitor/repositories.db";

    init_config_in_etc()?;
    init_db_in_var_lib()?;


    match args.command {
        Commands::Run => {
            println!("[INFO] Starting the release monitor...");
            //start_monitor(args).await?;
            match args.config {
                Some(config_file) => {
                    config = load_config(PathBuf::from(config_file))?;
                    database = Database::new(PathBuf::from(DB_PATH), config.clone())?;
                    monitor = Monitor::new(config, database, true);
                    monitor.run().await.expect("[ERROR] Monitor failed to run");
                    Ok(())
                }
                None => {
                    config = load_config(PathBuf::from(CONFIG_PATH))?;
                    database = Database::new(PathBuf::from(DB_PATH), config.clone())?;
                    monitor = Monitor::new(config, database, true);
                    monitor.run().await.expect("[ERROR] Monitor failed to run");
                    Ok(())
                }
            }
        }

        Commands::Start => {
            println!("[INFO] Starting the release monitor...");
            Command::new("systemctl")
                .args(["start", SYSTEMD_SERVICE])
                .status()?;
            println!("[INFO] Started the release monitor.");
            Ok(())
        }

        Commands::Restart => {
            println!("[INFO] Restart the release monitor...");
            Command::new("systemctl")
                .args(["restart", SYSTEMD_SERVICE])
                .status()?;
            println!("[INFO] Restarted the release monitor.");
            Ok(())
        }

        Commands::Stop => {
            println!("[INFO] Stopping the release monitor...");
            Command::new("systemctl")
                .args(["stop", SYSTEMD_SERVICE])
                .status()?;
            println!("[INFO] Stopped the release monitor.");
            Ok(())
        }

        Commands::Status => {
            println!("[INFO] Checking the status of the release monitor...");
            let config = load_config(PathBuf::from(CONFIG_PATH))?;
            let database = Database::new(PathBuf::from(DB_PATH), config.clone())?;
            let status = get_service_status(SYSTEMD_SERVICE)?;

            let mut table = Table::new();
            table
                .set_header(vec!["ID", "Owner", "Name", "Release", "On Release"])
                .set_content_arrangement(ContentArrangement::Dynamic);

            match database.get_repositories() {
                Ok(repositories) => {
                    for repository in repositories {
                        table.add_row(vec![repository.id.to_string(), repository.owner, repository.name, repository.version, repository.on_release]);
                    }
                }
                Err(e) => {
                    println!("[WARN] Failed to get repositories: {}", e);
                }
            }
            println!("\n");
            println!("Service: {}", status.active_state);
            println!("Status:  {}", status.sub_state);

            if let Some(pid) = status.main_pid {
                println!("PID:     {}", pid);
            }
            println!("{}", table);
            Ok(())
        }
        Commands::Add {name, owner, on_release} => {
            println!("[INFO] Adding a new repository to monitor...");
            let (Some(name), Some(owner), Some(on_release)) = (name, owner, on_release) else {
                return Err("[ERROR] name, owner and on_release are required. (release-monitor add --name <NAME> --owner <OWNER> --on_release <ON_RELEASE>)".into());
            };

            let content = fs::read_to_string(CONFIG_PATH)?;
            let mut config: Config = serde_yaml::from_str(&content)?;

            config.repositories.push(RepositoryConfig::new(name, owner, on_release));

            let yaml = serde_yaml::to_string(&config)?;
            fs::write(CONFIG_PATH, yaml)?;

            database = Database::new(PathBuf::from(DB_PATH), config.clone())?;
            database.sync_with_config(&config).expect("[ERROR] Failed to sync database and config");

            Ok(())
        }

        Commands::Remove { name, owner } => {
            println!("[INFO] Removing repository from monitor...");

            let (Some(name), Some(owner)) = (name, owner) else {
                return Err("[ERROR] name and owner are required. Release-monitor remove --name  --owner <OWNER>".into());
            };

            let content = fs::read_to_string(CONFIG_PATH)?;
            let mut config: Config = serde_yaml::from_str(&content)?;

            let original_len = config.repositories.len();

            config.repositories.retain(|repo| {
                !(repo.name == name && repo.owner == owner)
            });

            if config.repositories.len() == original_len {
                return Err(
                    format!(
                        "[ERROR] Repository {}/{} was not found in configuration",
                        owner, name
                    )
                        .into()
                );
            }

            let yaml = serde_yaml::to_string(&config)?;
            fs::write(CONFIG_PATH, yaml)?;

            let database = Database::new(
                PathBuf::from(DB_PATH),
                config.clone(),
            )?;

            database
                .sync_with_config(&config)
                .expect("[ERROR] Failed to sync database and config");

            println!(
                "[INFO] Removed repository {}/{} successfully",
                owner, name
            );

            Ok(())
        }
    }
}

fn load_config(path: PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(config)
}

fn init_config_in_etc() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = PathBuf::from("/etc/release-monitor/config.yaml");
    if !config_path.exists() {
        let default_config = Config::default();
        let yaml_string = serde_yaml::to_string(&default_config)?;
        fs::create_dir_all(config_path.parent().unwrap())?;
        fs::write(config_path, yaml_string)?;
        println!("[INFO] Default configuration file created at /etc/release-monitor/config.yaml");
    } else {
        println!("[INFO] Configuration file already exists at /etc/release-monitor/config.yaml");
    }
    Ok(())
}

fn init_db_in_var_lib() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = PathBuf::from("/var/lib/release-monitor/repositories.db");
    if !db_path.exists() {
        fs::create_dir_all(db_path.parent().unwrap())?;
        //Database::new(db_path, Config::default())?;
        println!("[INFO] Database initialized at /var/lib/release-monitor/repositories.db");
    } else {
        println!("[INFO] Database already exists at /var/lib/release-monitor/repositories.db");
    }
    Ok(())
}

fn get_service_status(
    service: &str,
) -> Result<ServiceStatus, Box<dyn std::error::Error>> {
    let output = Command::new("systemctl")
        .args([
            "show",
            service,
            "--property=ActiveState",
            "--property=SubState",
            "--property=MainPID",
        ])
        .output()?;

    if !output.status.success() {
        return Err(
            String::from_utf8_lossy(&output.stderr)
                .trim()
                .to_string()
                .into()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut active_state = String::new();
    let mut sub_state = String::new();
    let mut main_pid = None;

    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "ActiveState" => active_state = value.to_string(),
                "SubState" => sub_state = value.to_string(),
                "MainPID" => {
                    let pid: u32 = value.parse()?;

                    if pid != 0 {
                        main_pid = Some(pid);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(ServiceStatus {
        active_state,
        sub_state,
        main_pid,
    })
}