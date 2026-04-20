use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use anyhow::{Context, Result};

use crate::build::{BuildOptions, BuildSubcommand, EnvVar, RunOptions, RustcOptions, TestOptions};
use crate::project::{ProjectManifest, ProjectPaths, load_project_manifest};

pub struct RustcRunner<'a> {
    paths: &'a ProjectPaths,
}

impl<'a> RustcRunner<'a> {
    pub fn new(paths: &'a ProjectPaths) -> Self {
        Self { paths }
    }

    pub fn execute(&self, subcommand: &BuildSubcommand) -> Result<ExitCode> {
        match subcommand {
            BuildSubcommand::Build(options) => self.build(options),
            BuildSubcommand::Clean(_) => self.clean(),
            BuildSubcommand::Run(options) => self.run(options),
            BuildSubcommand::Test(options) => self.test(options),
        }
    }

    fn build(&self, options: &BuildOptions) -> Result<ExitCode> {
        let manifest = load_project_manifest(self.paths.root())?;
        let targets = self.compile_targets(&manifest, &options.rustc, BuildMode::Build)?;
        if targets.is_empty() {
            anyhow::bail!("no Rust entrypoint was generated, expected `.zoxi/src/main.rs` or `.zoxi/src/lib.rs`")
        }
        Ok(ExitCode::SUCCESS)
    }

    fn run(&self, options: &RunOptions) -> Result<ExitCode> {
        let manifest = load_project_manifest(self.paths.root())?;
        let targets = self.compile_targets(&manifest, &options.rustc, BuildMode::Build)?;
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

        let status = self
            .command_with_envs(&binary, &options.rustc.envs)
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
        let targets = self.compile_targets(&manifest, &options.rustc, BuildMode::Test)?;
        if targets.is_empty() {
            anyhow::bail!("no Rust entrypoint was generated, expected `.zoxi/src/main.rs` or `.zoxi/src/lib.rs`")
        }

        for target in targets {
            let status = self
                .command_with_envs(&target.output_path, &options.rustc.envs)
                .status()
                .with_context(|| format!("failed to run test binary {}", target.output_path.display()))?;
            if !status.success() {
                anyhow::bail!("tests for `{}` failed with status {status}", target.crate_name)
            }
        }

        Ok(ExitCode::SUCCESS)
    }

    fn clean(&self) -> Result<ExitCode> {
        let project_cache_dir = self.paths.project_cache_dir()?;
        remove_path_if_exists(&self.paths.generated_dir())?;
        remove_path_if_exists(&project_cache_dir)?;
        Ok(ExitCode::SUCCESS)
    }

    fn compile_targets(
        &self,
        manifest: &ProjectManifest,
        options: &RustcOptions,
        mode: BuildMode,
    ) -> Result<Vec<CompiledTarget>> {
        let definitions = self.target_definitions(manifest)?;
        if definitions.is_empty() {
            return Ok(Vec::new());
        }

        fs::create_dir_all(self.paths.profile_artifact_dir(options.release)?)?;
        fs::create_dir_all(self.paths.profile_incremental_dir(options.release)?)?;

        let library = definitions.iter().find(|target| target.kind == TargetKind::Library);
        let mut compiled = Vec::with_capacity(definitions.len());
        let mut compiled_library = None;

        if let Some(target) = library {
            let artifact = self.compile_target(target, manifest, options, mode, None)?;
            compiled_library = Some(artifact.output_path.clone());
            compiled.push(artifact);
        }

        for target in definitions {
            if target.kind == TargetKind::Library {
                continue;
            }

            let artifact = self.compile_target(
                &target,
                manifest,
                options,
                mode,
                compiled_library.as_deref(),
            )?;
            compiled.push(artifact);
        }

        Ok(compiled)
    }

    fn compile_target(
        &self,
        target: &TargetDefinition,
        manifest: &ProjectManifest,
        options: &RustcOptions,
        mode: BuildMode,
        library_path: Option<&Path>,
    ) -> Result<CompiledTarget> {
        let output_path = self.output_path(target, manifest, options.release, mode)?;
        let incremental_dir = self.paths.profile_incremental_dir(options.release)?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut command = self.command_with_envs(Path::new("rustc"), &options.envs);
        command
            .current_dir(self.paths.root())
            .arg(&target.source_path)
            .arg("--crate-name")
            .arg(&target.crate_name)
            .arg("--edition")
            .arg(manifest.edition())
            .arg("-C")
            .arg(format!("incremental={}", incremental_dir.display()))
            .arg("-C")
            .arg("codegen-units=16")
            .arg("-o")
            .arg(&output_path);

        if let Some((linker, arg)) = self.preferred_linker() {
            command.arg("-C").arg(format!("linker={linker}"));
            command.arg("-C").arg(format!("link-arg={arg}"));
        }

        for flag in profile_flags(options.release) {
            command.arg("-C").arg(flag);
        }

        match mode {
            BuildMode::Build => {
                if target.kind == TargetKind::Library {
                    command.arg("--crate-type=rlib");
                }
            }
            BuildMode::Test => {
                command.arg("--test");
            }
        }

        if let Some(path) = library_path {
            command
                .arg("-L")
                .arg(format!("dependency={}", path.parent().unwrap_or(self.paths.root()).display()))
                .arg("--extern")
                .arg(format!("{}={}", manifest.crate_name(), path.display()));
        }

        command.args(options.command_args());

        let status = command.status().with_context(|| {
            format!(
                "failed to invoke rustc for {}",
                target.source_path.display()
            )
        })?;

        if !status.success() {
            anyhow::bail!(
                "rustc failed for {} with status {status}",
                target.source_path.display()
            )
        }

        Ok(CompiledTarget {
            kind: target.kind,
            crate_name: target.crate_name.clone(),
            output_path,
        })
    }

    fn target_definitions(&self, manifest: &ProjectManifest) -> Result<Vec<TargetDefinition>> {
        let crate_name = manifest.crate_name();
        let mut targets = Vec::with_capacity(2);
        let lib_path = self.paths.generated_src_dir().join("lib.rs");
        if lib_path.exists() {
            targets.push(TargetDefinition {
                kind: TargetKind::Library,
                crate_name: crate_name.clone(),
                source_path: lib_path,
            });
        }

        let main_path = self.paths.generated_src_dir().join("main.rs");
        if main_path.exists() {
            targets.push(TargetDefinition {
                kind: TargetKind::Binary,
                crate_name,
                source_path: main_path,
            });
        }

        Ok(targets)
    }

    fn output_path(
        &self,
        target: &TargetDefinition,
        manifest: &ProjectManifest,
        release: bool,
        mode: BuildMode,
    ) -> Result<PathBuf> {
        let artifact_dir = self.paths.profile_artifact_dir(release)?;
        Ok(match (mode, target.kind) {
            (BuildMode::Build, TargetKind::Binary) => artifact_dir.join(manifest.package_name()),
            (BuildMode::Build, TargetKind::Library) => {
                artifact_dir.join(format!("lib{}.rlib", target.crate_name))
            }
            (BuildMode::Test, TargetKind::Binary) => {
                artifact_dir.join(format!("{}-bin-tests", manifest.package_name()))
            }
            (BuildMode::Test, TargetKind::Library) => artifact_dir.join(format!(
                "lib{}-tests",
                target.crate_name
            )),
        })
    }

    fn command_with_envs(&self, program: &Path, envs: &[EnvVar]) -> Command {
        let mut command = Command::new(program);
        envs.iter().for_each(|env_var| {
            command.env(env_var.key(), env_var.value());
        });
        command
    }

    fn preferred_linker(&self) -> Option<(&'static str, &'static str)> {
        let has_clang = command_exists("clang");
        if !has_clang {
            return None;
        }

        if command_exists("mold") {
            return Some(("clang", "-fuse-ld=mold"));
        }

        if command_exists("ld.lld") || command_exists("lld") {
            return Some(("clang", "-fuse-ld=lld"));
        }

        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BuildMode {
    Build,
    Test,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetKind {
    Binary,
    Library,
}

struct TargetDefinition {
    kind: TargetKind,
    crate_name: String,
    source_path: PathBuf,
}

struct CompiledTarget {
    kind: TargetKind,
    crate_name: String,
    output_path: PathBuf,
}

fn profile_flags(release: bool) -> [&'static str; 2] {
    if release {
        ["opt-level=3", "lto=thin"]
    } else {
        ["opt-level=0", "debuginfo=0"]
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

fn command_exists(name: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path).any(|directory| is_executable(&directory.join(name)))
}

fn is_executable(path: &Path) -> bool {
    if path.is_file() {
        return true;
    }

    if cfg!(windows) {
        ["exe", "cmd", "bat"]
            .into_iter()
            .map(|extension| {
                let mut candidate = OsString::from(path.as_os_str());
                candidate.push(".");
                candidate.push(extension);
                PathBuf::from(candidate)
            })
            .any(|candidate| candidate.is_file())
    } else {
        false
    }
}
