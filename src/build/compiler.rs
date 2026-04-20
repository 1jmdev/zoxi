use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};

use crate::build::{EnvVar, RustcOptions, status};
use crate::project::{
    BuildCacheEntry, BuildCacheState, ProjectManifest, ProjectPaths, load_build_cache_state,
    stable_hash_bytes, stable_hash_str, write_build_cache_state,
};

pub struct DependencyArtifacts {
    pub externs: Vec<(String, PathBuf)>,
    pub search_dirs: Vec<PathBuf>,
    pub fingerprint: u64,
}

pub struct CompiledTarget {
    pub kind: TargetKind,
    pub crate_name: String,
    pub output_path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildMode {
    Build,
    Test,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Binary,
    Library,
}

struct TargetDefinition {
    kind: TargetKind,
    crate_name: String,
    source_path: PathBuf,
}

pub fn compile_targets(
    paths: &ProjectPaths,
    manifest: &ProjectManifest,
    options: &RustcOptions,
    mode: BuildMode,
    dependencies: &DependencyArtifacts,
) -> Result<Vec<CompiledTarget>> {
    let definitions = target_definitions(paths, manifest);
    if definitions.is_empty() {
        return Ok(Vec::new());
    }

    fs::create_dir_all(paths.profile_artifact_dir(options.release))?;
    fs::create_dir_all(paths.profile_incremental_dir(options.release))?;

    let state_path = paths.build_cache_state_path();
    let previous_state = load_build_cache_state(&state_path)?;
    let mut next_state = BuildCacheState::new();

    let library = definitions.iter().find(|target| target.kind == TargetKind::Library);
    let mut compiled = Vec::with_capacity(definitions.len());
    let mut compiled_library = None;

    if let Some(target) = library {
        let artifact = compile_target(
            paths,
            target,
            manifest,
            options,
            mode,
            None,
            dependencies,
            &previous_state,
            &mut next_state,
        )?;
        compiled_library = Some(artifact.output_path.clone());
        compiled.push(artifact);
    }

    for target in definitions {
        if target.kind == TargetKind::Library {
            continue;
        }

        let artifact = compile_target(
            paths,
            &target,
            manifest,
            options,
            mode,
            compiled_library.as_deref(),
            dependencies,
            &previous_state,
            &mut next_state,
        )?;
        compiled.push(artifact);
    }

    write_build_cache_state(&state_path, &next_state)?;
    Ok(compiled)
}

#[allow(clippy::too_many_arguments)]
fn compile_target(
    paths: &ProjectPaths,
    target: &TargetDefinition,
    manifest: &ProjectManifest,
    options: &RustcOptions,
    mode: BuildMode,
    local_library_path: Option<&Path>,
    dependencies: &DependencyArtifacts,
    previous_state: &BuildCacheState,
    next_state: &mut BuildCacheState,
) -> Result<CompiledTarget> {
    let output_path = output_path(paths, target, manifest, options.release, mode);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let fingerprint = target_fingerprint(target, manifest, options, mode, dependencies)?;
    let target_key = target_key(target, options.release, mode);
    if let Some(entry) = previous_state.get(&target_key)
        && entry.matches(&output_path, fingerprint)
        && output_path.exists()
    {
        status("Fresh", target.source_path.display());
        next_state.insert(
            target_key,
            BuildCacheEntry::new(output_path.clone(), fingerprint),
        );
        return Ok(CompiledTarget {
            kind: target.kind,
            crate_name: target.crate_name.clone(),
            output_path,
        });
    }

    status("Compiling", target.source_path.display());
    let mut command = command_with_envs(Path::new("rustc"), &options.envs);
    command
        .current_dir(paths.root())
        .arg(&target.source_path)
        .arg("--crate-name")
        .arg(&target.crate_name)
        .arg("--edition")
        .arg(manifest.edition())
        .arg("-C")
        .arg(format!(
            "incremental={}",
            paths.profile_incremental_dir(options.release).display()
        ))
        .arg("-C")
        .arg("codegen-units=16")
        .arg("-o")
        .arg(&output_path);

    if let Some((linker, arg)) = preferred_linker() {
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

    if let Some(path) = local_library_path {
        command
            .arg("-L")
            .arg(format!(
                "dependency={}",
                path.parent().unwrap_or(paths.root()).display()
            ))
            .arg("--extern")
            .arg(format!("{}={}", manifest.crate_name(), path.display()));
    }

    for directory in &dependencies.search_dirs {
        command.arg("-L").arg(format!("dependency={}", directory.display()));
    }

    for (name, path) in &dependencies.externs {
        command.arg("--extern").arg(format!("{name}={}", path.display()));
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

    next_state.insert(target_key, BuildCacheEntry::new(output_path.clone(), fingerprint));
    Ok(CompiledTarget {
        kind: target.kind,
        crate_name: target.crate_name.clone(),
        output_path,
    })
}

fn target_fingerprint(
    target: &TargetDefinition,
    manifest: &ProjectManifest,
    options: &RustcOptions,
    mode: BuildMode,
    dependencies: &DependencyArtifacts,
) -> Result<u64> {
    let source = fs::read(&target.source_path)
        .with_context(|| format!("failed to read {}", target.source_path.display()))?;
    let mut seed = stable_hash_bytes(&source).to_string();
    seed.push('|');
    seed.push_str(manifest.edition());
    seed.push('|');
    seed.push_str(manifest.package_name());
    seed.push('|');
    seed.push_str(if options.release { "release" } else { "debug" });
    seed.push('|');
    seed.push_str(match mode {
        BuildMode::Build => "build",
        BuildMode::Test => "test",
    });
    seed.push('|');
    seed.push_str(&dependencies.fingerprint.to_string());
    options.args.iter().for_each(|arg| {
        seed.push('|');
        seed.push_str(arg);
    });
    Ok(stable_hash_str(&seed))
}

fn target_key(target: &TargetDefinition, release: bool, mode: BuildMode) -> String {
    let mut key = String::new();
    key.push_str(match mode {
        BuildMode::Build => "build",
        BuildMode::Test => "test",
    });
    key.push('|');
    key.push_str(if release { "release" } else { "debug" });
    key.push('|');
    key.push_str(match target.kind {
        TargetKind::Binary => "bin",
        TargetKind::Library => "lib",
    });
    key.push('|');
    key.push_str(&target.source_path.to_string_lossy());
    key
}

fn target_definitions(paths: &ProjectPaths, manifest: &ProjectManifest) -> Vec<TargetDefinition> {
    let crate_name = manifest.crate_name();
    let mut targets = Vec::with_capacity(2);
    let lib_path = paths.generated_src_dir().join("lib.rs");
    if lib_path.exists() {
        targets.push(TargetDefinition {
            kind: TargetKind::Library,
            crate_name: crate_name.clone(),
            source_path: lib_path,
        });
    }

    let main_path = paths.generated_src_dir().join("main.rs");
    if main_path.exists() {
        targets.push(TargetDefinition {
            kind: TargetKind::Binary,
            crate_name,
            source_path: main_path,
        });
    }

    targets
}

fn output_path(
    paths: &ProjectPaths,
    target: &TargetDefinition,
    manifest: &ProjectManifest,
    release: bool,
    mode: BuildMode,
) -> PathBuf {
    let artifact_dir = paths.profile_artifact_dir(release);
    match (mode, target.kind) {
        (BuildMode::Build, TargetKind::Binary) => artifact_dir.join(manifest.package_name()),
        (BuildMode::Build, TargetKind::Library) => {
            artifact_dir.join(format!("lib{}.rlib", target.crate_name))
        }
        (BuildMode::Test, TargetKind::Binary) => {
            artifact_dir.join(format!("{}-bin-tests", manifest.package_name()))
        }
        (BuildMode::Test, TargetKind::Library) => {
            artifact_dir.join(format!("lib{}-tests", target.crate_name))
        }
    }
}

fn profile_flags(release: bool) -> [&'static str; 2] {
    if release {
        ["opt-level=3", "lto=thin"]
    } else {
        ["opt-level=0", "debuginfo=0"]
    }
}

fn command_with_envs(program: &Path, envs: &[EnvVar]) -> Command {
    let mut command = Command::new(program);
    envs.iter().for_each(|env_var| {
        command.env(env_var.key(), env_var.value());
    });
    command
}

fn preferred_linker() -> Option<(&'static str, &'static str)> {
    if !command_exists("clang") {
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
