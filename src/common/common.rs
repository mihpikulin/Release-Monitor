use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub owner: String,
    pub url: String,
    pub version: String,
    pub on_release: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub refresh_interval: u64,
    pub repositories: Vec<Repository>,
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
            name: row.get("name")?,
            owner: row.get("owner")?,
            url: row.get("url")?,
            version: row.get("release")?,
            on_release: row.get("on_release")?,
        })
    }
}