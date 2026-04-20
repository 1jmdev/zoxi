use std::{fs, path::Path, process::ExitCode};

use anyhow::{Context, Result};

use crate::build::compiler::{BuildMode, TargetKind, compile_targets};
use crate::build::dependencies::{add_packages, prepare_dependency_artifacts, remove_packages};
use crate::build::{
    BuildOptions, BuildSubcommand, CleanOptions, RemoveOptions, RunOptions, TestOptions, status,
};
use crate::project::{ProjectPaths, load_project_manifest};

pub struct RustcRunner<'a> {
    paths: &'a ProjectPaths,
}

impl<'a> RustcRunner<'a> {
    pub fn new(paths: &'a ProjectPaths) -> Self {
        Self { paths }
    }

    pub fn execute(&self, subcommand: &BuildSubcommand) -> Result<ExitCode> {
        match subcommand {
            BuildSubcommand::Add(options) => self.add(options.packages.as_slice()),
            BuildSubcommand::Build(options) => self.build(options),
            BuildSubcommand::Clean(options) => self.clean(options),
            BuildSubcommand::Remove(options) => self.remove(options),
            BuildSubcommand::Run(options) => self.run(options),
            BuildSubcommand::Test(options) => self.test(options),
        }
    }

    fn add(&self, packages: &[String]) -> Result<ExitCode> {
        add_packages(self.paths, packages)?;
        Ok(ExitCode::SUCCESS)
    }

    fn remove(&self, options: &RemoveOptions) -> Result<ExitCode> {
        remove_packages(self.paths, options.packages.as_slice())?;
        Ok(ExitCode::SUCCESS)
    }

    fn build(&self, options: &BuildOptions) -> Result<ExitCode> {
        let manifest = load_project_manifest(self.paths.root())?;
        let dependencies = prepare_dependency_artifacts(self.paths, &manifest, options.rustc.release)?;
        let targets = compile_targets(
            self.paths,
            &manifest,
            &options.rustc,
            BuildMode::Build,
            &dependencies,
        )?;
        if targets.is_empty() {
            anyhow::bail!("no Rust entrypoint was generated, expected `.zoxi/src/main.rs` or `.zoxi/src/lib.rs`")
        }
        status("Finished", profile_label(options.rustc.release));
        Ok(ExitCode::SUCCESS)
    }

    fn run(&self, options: &RunOptions) -> Result<ExitCode> {
        let manifest = load_project_manifest(self.paths.root())?;
        let dependencies = prepare_dependency_artifacts(self.paths, &manifest, options.rustc.release)?;
        let targets = compile_targets(
            self.paths,
            &manifest,
            &options.rustc,
            BuildMode::Build,
            &dependencies,
        )?;

        let binary = targets
            .into_iter()
            .find(|target| target.kind == TargetKind::Binary)
            .map(|target| target.output_path)
            .with_context(|| {
                format!(
                    "cannot run `{}`, no binary entrypoint was found at {}",
                    manifest.package_name(),
                    self.paths.generated_src_dir().join("main.rs").display()
                )
            })?;

        status("Finished", profile_label(options.rustc.release));
        status("Running", binary.display());
        let status = std::process::Command::new(&binary)
            .args(&options.app_args)
            .status()
            .with_context(|| format!("failed to run {}", binary.display()))?;

        if status.success() {
            Ok(ExitCode::SUCCESS)
        } else {
            anyhow::bail!("compiled program exited with status {status}")
        }
    }

    fn test(&self, options: &TestOptions) -> Result<ExitCode> {
        let manifest = load_project_manifest(self.paths.root())?;
        let dependencies = prepare_dependency_artifacts(self.paths, &manifest, options.rustc.release)?;
        let targets = compile_targets(
            self.paths,
            &manifest,
            &options.rustc,
            BuildMode::Test,
            &dependencies,
        )?;

        if targets.is_empty() {
            anyhow::bail!("no Rust entrypoint was generated, expected `.zoxi/src/main.rs` or `.zoxi/src/lib.rs`")
        }

        status("Finished", profile_label(options.rustc.release));

        for target in targets {
            status("Running", target.output_path.display());
            let status = std::process::Command::new(&target.output_path)
                .status()
                .with_context(|| {
                    format!("failed to run test binary {}", target.output_path.display())
                })?;
            if !status.success() {
                anyhow::bail!("tests for `{}` failed with status {status}", target.crate_name)
            }
        }

        Ok(ExitCode::SUCCESS)
    }

    fn clean(&self, _: &CleanOptions) -> Result<ExitCode> {
        remove_path_if_exists(&self.paths.generated_dir())?;
        Ok(ExitCode::SUCCESS)
    }
}

fn profile_label(release: bool) -> &'static str {
    if release {
        "release profile [optimized]"
    } else {
        "dev profile [unoptimized]"
    }
}

fn remove_path_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }

    Ok(())
}
