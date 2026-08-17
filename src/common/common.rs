use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub name: String,
    pub owner: String,
    pub on_release: String,
}

pub struct Repository {
    pub id: i64,
    pub name: String,
    pub owner: String,
    pub version: String,
    pub on_release: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub refresh_interval: u64,
    pub repositories: Vec<RepositoryConfig>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Release {
    pub(crate) tag_name: String,
    name: Option<String>,
    prerelease: bool,
    published_at: String,
}

impl Repository {
    pub fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            owner: row.get("owner")?,
            version: row.get("release")?,
            on_release: row.get("on_release")?,
        })
    }
}

impl Config {
    pub fn default() -> Self {
        Self {
            refresh_interval: 3600,
            repositories: Vec::new(),
        }
    }
}

impl RepositoryConfig {
    pub fn new(name: String, owner: String, on_release: String) -> Self {
        Self {
            name,
            owner,
            on_release,
        }
    }
}