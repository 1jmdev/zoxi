use anyhow::Result;
use std::{
    env::{current_dir, var_os},
    path::{Path, PathBuf},
};

use crate::project::stable_hash_str;

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

    pub fn generated_dir(&self) -> PathBuf {
        self.root.join(".zoxi")
    }

    pub fn generated_src_dir(&self) -> PathBuf {
        self.generated_dir().join("src")
    }

    pub fn global_root_dir(&self) -> Result<PathBuf> {
        let home = var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
        Ok(PathBuf::from(home).join(".zoxi"))
    }

    pub fn global_cache_dir(&self) -> Result<PathBuf> {
        Ok(self.global_root_dir()?.join("cache"))
    }

    pub fn project_cache_dir(&self) -> Result<PathBuf> {
        Ok(self
            .global_cache_dir()?
            .join("projects")
            .join(self.project_cache_key()))
    }

    pub fn transpile_cache_state_path(&self) -> Result<PathBuf> {
        Ok(self.project_cache_dir()?.join("transpile-state"))
    }

    pub fn profile_artifact_dir(&self, release: bool) -> Result<PathBuf> {
        Ok(self
            .project_cache_dir()?
            .join("artifacts")
            .join(profile_name(release)))
    }

    pub fn profile_incremental_dir(&self, release: bool) -> Result<PathBuf> {
        Ok(self
            .project_cache_dir()?
            .join("incremental")
            .join(profile_name(release)))
    }

    fn project_cache_key(&self) -> String {
        let root = self.root.to_string_lossy();
        format!("{:016x}", stable_hash_str(&root))
    }
}

fn profile_name(release: bool) -> &'static str {
    if release {
        "release"
    } else {
        "debug"
    }
}
