use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;

use crate::project::{discover_sources, write_generated_manifest, ProjectPaths};
use crate::transpiler::error::TranspileError;
use crate::transpiler::source::transpile_source;

pub struct Transpiler {
    paths: ProjectPaths,
}

impl Transpiler {
    pub fn new(paths: ProjectPaths) -> Self {
        Self { paths }
    }

    pub fn build(&self, cargo_args: &[String]) -> anyhow::Result<ExitCode> {
        self.transpile_project()?;
        run_cargo(self.paths.generated_dir().as_path(), "build", cargo_args)
    }

    pub fn run(&self, cargo_args: &[String]) -> anyhow::Result<ExitCode> {
        self.transpile_project()?;
        run_cargo(self.paths.generated_dir().as_path(), "run", cargo_args)
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
}

fn run_cargo(generated_dir: &Path, subcommand: &str, cargo_args: &[String]) -> anyhow::Result<ExitCode> {
    let mut command = std::process::Command::new("cargo");
    command.current_dir(generated_dir).arg(subcommand).args(cargo_args);

    let status = command.status()?;
    if status.success() {
        Ok(ExitCode::SUCCESS)
    } else {
        anyhow::bail!("cargo {subcommand} failed with status {status}")
    }
}
