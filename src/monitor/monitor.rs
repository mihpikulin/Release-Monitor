use crate::Config;
use crate::common::Release;
use crate::database::Database;
use crate::executor::Executor;
use colored::Colorize;
use tokio::time::{Duration, sleep};

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
                    Err(e) => {
                        println!("[ERROR] Failed to fetch release for {}/{}: {}. Continuing...", repo.owner, repo.name, e);
                        continue;
                    }
                };

                let executor = Executor::new(repo.on_release.clone());
                if release.tag_name != repo.version {
                    println!("[INFO] New release detected for {}: {}", repo.name, release.tag_name);
                    match executor.execute().await {
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

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running = false;
        Ok(())
    }

    pub async fn get_status(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.running {
            println!("{}", "Active".green());
        } else {
            println!("[INFO] Monitor is stopped");
        }
        Ok(())
    }
}