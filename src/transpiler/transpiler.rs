use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;

use crate::cargo::{CargoRunner, CargoSubcommand};
use crate::project::{
    discover_sources, ensure_project_manifest, write_generated_manifest, ProjectPaths,
};
use crate::transpiler::error::TranspileError;
use crate::transpiler::source::transpile_source;

pub struct Transpiler {
    paths: ProjectPaths,
}

impl Transpiler {
    pub fn new(paths: ProjectPaths) -> Self {
        Self { paths }
    }

    pub fn execute(&self, cargo: CargoSubcommand) -> anyhow::Result<ExitCode> {
        match cargo {
            CargoSubcommand::Add(_) | CargoSubcommand::Remove(_) => {
                ensure_project_manifest(self.paths.root())?;
                self.edit_project_manifest_with_cargo(&cargo)
            }
            CargoSubcommand::Build(_)
            | CargoSubcommand::Run(_)
            | CargoSubcommand::Test(_)
            | CargoSubcommand::Clean(_)
            | CargoSubcommand::Custom { .. } => {
                self.transpile_project()?;
                CargoRunner::new(self.paths.generated_dir().as_path()).execute(&cargo)
            }
        }
    }

    fn transpile_project(&self) -> anyhow::Result<()> {
        let src_dir = self.paths.src_dir();
        if !src_dir.exists() {
            return Err(TranspileError::MissingSourceDirectory(src_dir.display().to_string()).into());
        }

        let files = discover_sources(&src_dir)?;
        if files.is_empty() {
            return Err(TranspileError::NoSources(src_dir.display().to_string()).into());
        }

        let generated_dir = self.paths.generated_dir();
        let generated_src_dir = self.paths.generated_src_dir();
        if generated_dir.exists() {
            std::fs::remove_dir_all(&generated_dir)?;
        }
        std::fs::create_dir_all(&generated_src_dir)?;
        write_generated_manifest(self.paths.root(), &generated_dir)?;

        for file in files {
            let relative = file
                .strip_prefix(&src_dir)
                .with_context(|| format!("failed to compute relative path for {}", file.display()))?;
            let output = generated_src_dir.join(relative).with_extension("rs");
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let source = std::fs::read_to_string(&file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            let transpiled = transpile_source(&source, relative == Path::new("main.zo"))
                .with_context(|| format!("failed to transpile {}", file.display()))?;
            std::fs::write(&output, transpiled)
                .with_context(|| format!("failed to write {}", output.display()))?;
        }

        Ok(())
    }

    fn edit_project_manifest_with_cargo(&self, cargo: &CargoSubcommand) -> anyhow::Result<ExitCode> {
        let workspace = self.create_temp_manifest_workspace()?;
        let result = CargoRunner::new(&workspace).execute(cargo);

        if result.is_ok() {
            std::fs::copy(workspace.join("Cargo.toml"), self.paths.config_path())?;
            let generated_lock = workspace.join("Cargo.lock");
            if generated_lock.exists() {
                std::fs::copy(generated_lock, self.paths.lock_path())?;
            }
        }

        let cleanup_result = std::fs::remove_dir_all(&workspace);
        match (result, cleanup_result) {
            (Ok(status), Ok(())) => Ok(status),
            (Ok(_), Err(error)) => Err(error.into()),
            (Err(error), Ok(())) => Err(error),
            (Err(error), Err(_)) => Err(error),
        }
    }

    fn create_temp_manifest_workspace(&self) -> anyhow::Result<std::path::PathBuf> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!("zoxi-cargo-{}-{unique}", std::process::id()));
        let src_dir = workspace.join("src");

        std::fs::create_dir_all(&src_dir)?;
        std::fs::copy(self.paths.config_path(), workspace.join("Cargo.toml"))?;
        if self.paths.lock_path().exists() {
            std::fs::copy(self.paths.lock_path(), workspace.join("Cargo.lock"))?;
        }
        std::fs::write(src_dir.join("main.rs"), "fn main() {}\n")?;
        Ok(workspace)
    }
}
