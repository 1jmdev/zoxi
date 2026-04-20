use anyhow::Result;
use std::{
    env::{current_dir, var_os},
    path::{Path, PathBuf},
};

pub struct ProjectPaths {
    root: PathBuf,
}

impl ProjectPaths {
    pub fn new(root: Option<PathBuf>) -> Result<Self> {
        let cwd = current_dir()?;
        let root = match root {
            Some(path) if path.is_absolute() => path,
            Some(path) => cwd.join(path),
            None => cwd,
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

    pub fn project_cache_dir(&self) -> PathBuf {
        self.generated_dir().join(".cache")
    }

    pub fn transpile_cache_state_path(&self) -> PathBuf {
        self.project_cache_dir().join("transpile-state")
    }

    pub fn build_cache_state_path(&self) -> PathBuf {
        self.project_cache_dir().join("build-state")
    }

    pub fn profile_artifact_dir(&self, release: bool) -> PathBuf {
        self.project_cache_dir().join("artifacts").join(profile_name(release))
    }

    pub fn profile_incremental_dir(&self, release: bool) -> PathBuf {
        self.project_cache_dir()
            .join("incremental")
            .join(profile_name(release))
    }

    pub fn global_root_dir(&self) -> Result<PathBuf> {
        let home = var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
        Ok(PathBuf::from(home).join(".zoxi"))
    }

    pub fn global_cache_dir(&self) -> Result<PathBuf> {
        Ok(self.global_root_dir()?.join("cache"))
    }

    pub fn version_cache_dir(&self) -> Result<PathBuf> {
        Ok(self.global_cache_dir()?.join("crates"))
    }

    pub fn source_cache_dir(&self) -> Result<PathBuf> {
        Ok(self.global_cache_dir()?.join("sources"))
    }

    pub fn registry_cache_dir(&self) -> Result<PathBuf> {
        Ok(self.global_cache_dir()?.join("registry"))
    }
}

fn profile_name(release: bool) -> &'static str {
    if release {
        "release"
    } else {
        "debug"
    }
}
