use std::{
    path::{PathBuf, Path},
    env::current_dir
};
use anyhow::Result;

pub struct ProjectPaths {
    root: PathBuf,
}

impl ProjectPaths {
    pub fn new(root: Option<PathBuf>) -> Result<Self> {
        let root = match root {
            Some(path) => path,
            None => current_dir()?,
        };

        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn src_dir(&self) -> PathBuf {
        self.root.join("src")
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("zoxi.toml")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join("zoxi.lock")
    }

    pub fn generated_dir(&self) -> PathBuf {
        self.root.join(".zoxi")
    }

    pub fn generated_cache_dir(&self) -> PathBuf {
        self.generated_dir().join(".cache")
    }

    pub fn generated_cache_state_path(&self) -> PathBuf {
        self.generated_cache_dir().join("state")
    }

    pub fn generated_src_dir(&self) -> PathBuf {
        self.generated_dir().join("src")
    }
}
