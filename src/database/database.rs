use std::path::PathBuf;
use rusqlite::Connection;
use crate::{Config};
use crate::common::Repository;

pub struct Database {
    database: PathBuf,
    connection: Connection,
}

impl Database {
    pub fn new(database: PathBuf, config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::open(&database).expect("[ERROR] Failed to open database");
        connection.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    owner TEXT NOT NULL,
                    url TEXT NOT NULL,
                    release TEXT NOT NULL,
                    on_release TEXT NOT NULL
            )",
            []
        )?;

        Ok(Self { database, connection })
    }

    pub fn sync_with_config(&self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        let config_list: Vec<Repository> = config.repositories.clone();
        let db_list: Vec<Repository> = self.get_repositories()?;

        // Check for new repositories in the config that are not in the database and insert them
        for repo in &config_list {
            if !db_list.iter().any(
                |r|
                    r.name == repo.name &&
                        r.owner == repo.owner &&
                        r.url == repo.url &&
                        r.on_release == repo.on_release &&
                        r.version == repo.version
            ) {
                self.connection.execute(
                    "INSERT INTO repositories (name, owner, url, release, on_release) VALUES (?1, ?2, ?3, ?4, ?5)",
                    &[&repo.name, &repo.owner, &repo.url, &repo.version, &repo.on_release],
                )?;
            }
        }

        // Check for repositories in the database that are not in the config and delete them
        for repo in &db_list {
            if !config_list.iter().any(
                |r|
                    r.name == repo.name &&
                        r.owner == repo.owner &&
                        r.url == repo.url &&
                        r.on_release == repo.on_release &&
                        r.version == repo.version
            ) {
                self.connection.execute(
                    "DELETE FROM repositories WHERE name = ?1 AND owner = ?2 AND url = ?3 AND on_release = ?4 AND release = ?5",
                    &[&repo.name, &repo.owner, &repo.url, &repo.on_release, &repo.version],
                )?;
            }
        }

        Ok(())
    }

    pub(crate) fn get_repositories(&self) -> Result<Vec<Repository>, Box<dyn std::error::Error>> {
        let mut result_list: Vec<Repository> = Vec::<Repository>::new();
        let mut stmt = self.connection.prepare("SELECT * FROM repositories")?;
        let rows = stmt.query_map([], Repository::from_row)?;

        for row in rows {
            let repo = row?;
            result_list.push(repo);
        }
        Ok(result_list)
    }

    pub fn update_version_in_repo(&self, repository: &Repository, version: String) -> Result<(), Box<dyn std::error::Error>> {
        println!("[INFO] Updating version for {}: {} to {}", repository.name, repository.version, version);
        self.connection.execute(
            "UPDATE repositories SET release = ?1 WHERE owner = ?2 AND name = ?3",
            &[version.as_str(), repository.owner.as_str(), repository.name.as_str()],
        )?;

        Ok(())
    }
}