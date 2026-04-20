use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::ExitCode,
};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::build::{BuildSubcommand, RustcRunner, status};
use crate::project::file_sync::write_if_changed;
use crate::project::{
    CacheEntry, CacheState, ProjectPaths, SourceFingerprint, discover_sources, load_cache_state,
    write_cache_state,
};
use crate::transpiler::compiler::compile_source;
use crate::transpiler::error::TranspileError;

pub struct Transpiler {
    paths: ProjectPaths,
}

impl Transpiler {
    pub fn new(paths: ProjectPaths) -> Self {
        Self { paths }
    }

    pub fn execute(&self, command: BuildSubcommand) -> Result<ExitCode> {
        match command {
            BuildSubcommand::Add(_) | BuildSubcommand::Remove(_) | BuildSubcommand::Clean(_) => {
                RustcRunner::new(&self.paths).execute(&command)
            }
            BuildSubcommand::Build(_)
            | BuildSubcommand::Run(_)
            | BuildSubcommand::Test(_) => {
                self.transpile_project()?;
                RustcRunner::new(&self.paths).execute(&command)
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
        self.remove_legacy_generated_artifacts()?;
        let state_path = self.paths.transpile_cache_state_path();
        let previous_state = load_cache_state(&state_path)?;
        let mut next_state = CacheState::new();
        let mut transpiled_files = 0usize;
        let mut cached_files = 0usize;

        status("Transpiling", src_dir.display());

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
                cached_files += 1;
                continue;
            }

            let source = fs::read_to_string(&file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            let transpiled = compile_source(&source, &relative.display().to_string())
                .map_err(|error| anyhow::anyhow!(error.render(&relative.display().to_string(), &source)))
                .with_context(|| format!("failed to transpile {}", file.display()))?;
            write_if_changed(&output, transpiled.as_bytes())
                .with_context(|| format!("failed to write {}", output.display()))?;
            next_state.insert(
                relative.to_path_buf(),
                CacheEntry::new(generated_relative.to_path_buf(), fingerprint),
            );
            transpiled_files += 1;
        }

        self.remove_stale_generated_files(&generated_dir, &generated_src_dir, &next_state)?;
        write_cache_state(&state_path, &next_state)?;
        status(
            "Finished",
            format!("transpilation ({transpiled_files} changed, {cached_files} cached)"),
        );

        Ok(())
    }

    fn remove_legacy_generated_artifacts(&self) -> Result<()> {
        for path in [
            self.paths.generated_dir().join("Cargo.toml"),
            self.paths.generated_dir().join("Cargo.lock"),
            self.paths.generated_dir().join("target"),
        ] {
            if !path.exists() {
                continue;
            }

            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }

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
}
