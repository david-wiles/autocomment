use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};

use crate::error::Error;

#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct Credentials {
    /// Jira Username
    pub jira_user: String,

    /// Jira Password
    pub jira_pass: String,

    /// Jira Domain
    pub jira_domain: String,

    /// Github User
    pub github_user: String,

    /// Github Password
    pub github_pass: String,

    /// Github Domain
    pub github_domain: String,
}

impl Credentials {
    pub fn from_env() -> Result<Credentials, Error> {
        let f = std::fs::File::open(Self::config_file().as_path())?;
        serde_yaml::from_reader(f).map_err(Error::from)
    }

    pub fn save(&self) -> Result<(), Error> {
        let pathbuf = Self::config_file();
        let p = pathbuf.as_path();

        if !p.exists() {
            // Create directory if it doesn't exist
            if let Some(parent) = p.parent() {
                if !parent.exists() {
                    std::fs::create_dir(parent)?;
                }
            }
            // What if the parent directory is None?
        }

        let f = std::fs::File::create(p)?;
        serde_yaml::to_writer(f, self).map_err(Error::from)
    }

    /// Gets the default config file from the current user's home directory or
    /// from the current directory if there is no home
    fn config_file() -> PathBuf {
        home::home_dir()
            .map(|home_dir| home_dir.join(Path::new(".autocomment/config.yaml")))
            .unwrap_or(PathBuf::from(".autocomment/config.yaml"))
    }
}
