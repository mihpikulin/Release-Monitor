use tokio::time::{sleep, Duration};
use std::path::PathBuf;
use crate::common::Release;
use crate::executor::Executor;
use crate::database::Database;
use crate::Config;

pub struct Monitor {
    config: Config,
    database: Database,
    running: bool
}

impl Monitor {
    pub fn new(config: Config, database: Database, running: bool) -> Self {
        Monitor {
            config,
            database,
            running
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize the database and sync with the config
        self.database = match Database::new(PathBuf::from("repositories.db"), self.config.clone()) {
            Ok(db) => {
                println!("[INFO] Database initialized successfully");
                db
            },
            Err(e) => return Err(e),
        };
        match self.database.sync_with_config(&self.config) {
            Ok(()) => println!("[INFO] Database synchronized successfully"),
            Err(e) => return Err(e),
        }

        while self.running {
            println!("[INFO] Waiting for database updates");
            for repo in &self.database.get_repositories()? {
                let release = match Self::request(&repo.name, &repo.owner).await {
                    Ok(release) => {
                        println!("[INFO] Fetched latest release for {}: {}", repo.name, release.tag_name);
                        release
                    },
                    Err(e) => return Err(e)
                };

                let executor = Executor::new(repo.on_release.clone());
                if release.tag_name != repo.version {
                    println!("[INFO] New release detected for {}: {}", repo.name, release.tag_name);
                    match executor.execute() {
                        Ok(()) => println!("[INFO] Executed command for {} successfully", repo.name),
                        Err(e) => return Err(e),
                    }
                    self.database.update_version_in_repo(repo, release.tag_name).expect("[ERROR] Couldn't update database");
                } else {
                    println!("[INFO] No new release for {}. Current version: {}", repo.name, repo.version);
                }
            }

            sleep(Duration::from_secs(self.config.refresh_interval)).await;
        }

        Ok(())
    }
    
    async fn request(name: &String, owner: &String) -> Result<Release, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = format!("https://api.github.com/repos/{}/{}/releases/latest", owner, name);

        let release = client
            .get(&url)
            .header("User-Agent", "release-monitor")
            .send()
            .await?
            .json::<Release>()
            .await?;

        Ok(release)
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}