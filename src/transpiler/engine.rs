use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process,
    process::ExitCode,
    time,
};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::cargo::{CargoRunner, CargoSubcommand};
use crate::project::file_sync::write_if_changed;
use crate::project::{
    CacheEntry, CacheState, ProjectPaths, SourceFingerprint, discover_sources,
    ensure_project_manifest, load_cache_state, write_cache_state, write_generated_manifest,
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

    pub fn execute(&self, cargo: CargoSubcommand) -> Result<ExitCode> {
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

    fn transpile_project(&self) -> Result<()> {
        let src_dir = self.paths.src_dir();
        if !src_dir.exists() {
            return Err(
                TranspileError::MissingSourceDirectory(src_dir.display().to_string()).into(),
            );
        }

        let files = discover_sources(&src_dir)?;
        if files.is_empty() {
            return Err(TranspileError::NoSources(src_dir.display().to_string()).into());
        }

        let generated_dir = self.paths.generated_dir();
        let generated_src_dir = self.paths.generated_src_dir();
        fs::create_dir_all(&generated_src_dir)?;
        fs::create_dir_all(self.paths.generated_cache_dir())?;
        write_generated_manifest(self.paths.root(), &generated_dir)?;
        let previous_state = load_cache_state(&self.paths.generated_cache_state_path())?;
        let mut next_state = CacheState::new();

        for file in files {
            let relative = file.strip_prefix(&src_dir).with_context(|| {
                format!("failed to compute relative path for {}", file.display())
            })?;
            let output = generated_src_dir.join(relative).with_extension("rs");
            let generated_relative = output.strip_prefix(&generated_dir).with_context(|| {
                format!("failed to compute generated path for {}", output.display())
            })?;
            let fingerprint = SourceFingerprint::from_path(&file)?;

            if let Some(entry) = previous_state.get(relative)
                && entry.matches(&fingerprint)
                && generated_dir.join(entry.generated_path()).exists()
            {
                next_state.insert(relative.to_path_buf(), entry.clone());
                continue;
            }

            let source = fs::read_to_string(&file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            let transpiled = transpile_source(&source, relative == Path::new("main.zo"))
                .with_context(|| format!("failed to transpile {}", file.display()))?;
            write_if_changed(&output, transpiled.as_bytes())
                .with_context(|| format!("failed to write {}", output.display()))?;
            next_state.insert(
                relative.to_path_buf(),
                CacheEntry::new(generated_relative.to_path_buf(), fingerprint),
            );
        }

        self.remove_stale_generated_files(&generated_dir, &generated_src_dir, &next_state)?;
        write_cache_state(&self.paths.generated_cache_state_path(), &next_state)?;

        Ok(())
    }

    fn remove_stale_generated_files(
        &self,
        generated_dir: &Path,
        generated_src_dir: &Path,
        next_state: &CacheState,
    ) -> Result<()> {
        let expected = next_state
            .entries()
            .map(|(_, entry)| generated_dir.join(entry.generated_path()))
            .collect::<BTreeSet<_>>();

        for entry in WalkDir::new(generated_src_dir)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let generated_path = entry.path();
            if generated_path
                .extension()
                .is_none_or(|extension| extension != "rs")
            {
                continue;
            }

            if expected.contains(generated_path) {
                continue;
            }

            fs::remove_file(generated_path)?;
            self.remove_empty_generated_dirs(generated_path.parent(), generated_src_dir)?;
        }

        Ok(())
    }

    fn remove_empty_generated_dirs(
        &self,
        mut current: Option<&Path>,
        generated_src_dir: &Path,
    ) -> Result<()> {
        while let Some(path) = current {
            if path == generated_src_dir {
                break;
            }

            if fs::read_dir(path)?.next().is_some() {
                break;
            }

            fs::remove_dir(path)?;
            current = path.parent();
        }

        Ok(())
    }

    fn edit_project_manifest_with_cargo(&self, cargo: &CargoSubcommand) -> Result<ExitCode> {
        let workspace = self.create_temp_manifest_workspace()?;
        let result = CargoRunner::new(&workspace).execute(cargo);

        if result.is_ok() {
            fs::copy(workspace.join("Cargo.toml"), self.paths.config_path())?;
            let generated_lock = workspace.join("Cargo.lock");
            if generated_lock.exists() {
                fs::copy(generated_lock, self.paths.lock_path())?;
            }
        }

        let cleanup_result = fs::remove_dir_all(&workspace);
        match (result, cleanup_result) {
            (Ok(status), Ok(())) => Ok(status),
            (Ok(_), Err(error)) => Err(error.into()),
            (Err(error), Ok(())) => Err(error),
            (Err(error), Err(_)) => Err(error),
        }
    }

    fn create_temp_manifest_workspace(&self) -> Result<PathBuf> {
        let unique = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)?
            .as_nanos();
        let workspace = env::temp_dir().join(format!("zoxi-cargo-{}-{unique}", process::id()));
        let src_dir = workspace.join("src");

        fs::create_dir_all(&src_dir)?;
        fs::copy(self.paths.config_path(), workspace.join("Cargo.toml"))?;
        if self.paths.lock_path().exists() {
            fs::copy(self.paths.lock_path(), workspace.join("Cargo.lock"))?;
        }
        fs::write(src_dir.join("main.rs"), "fn main() {}\n")?;
        Ok(workspace)
    }
}
