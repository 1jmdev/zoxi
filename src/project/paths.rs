use std::path::PathBuf;

pub struct ProjectPaths {
    root: PathBuf,
}

impl ProjectPaths {
    pub fn new(root: Option<PathBuf>) -> anyhow::Result<Self> {
        let root = match root {
            Some(path) => path,
            None => std::env::current_dir()?,
        };

        Ok(Self { root })
    }

    pub fn root(&self) -> &std::path::Path {
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
}
