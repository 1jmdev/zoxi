use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use semver::{Version, VersionReq};
use serde::Deserialize;
use tar::Archive;

use crate::build::compiler::DependencyArtifacts;
use crate::build::{print_command_output, status};
use crate::project::file_sync::write_if_changed;
use crate::project::{
    ProjectManifest, ProjectPaths, add_dependencies, remove_dependencies, stable_hash_bytes,
    stable_hash_str,
};

pub fn add_packages(paths: &ProjectPaths, packages: &[String]) -> Result<()> {
    if packages.is_empty() {
        anyhow::bail!("add requires at least one package")
    }

    let resolved = packages
        .iter()
        .map(|package| resolve_package_spec(paths, package))
        .collect::<Result<Vec<_>>>()?;
    status("Updating", "zoxi.toml");
    add_dependencies(paths.root(), &resolved)?;
    Ok(())
}

pub fn remove_packages(paths: &ProjectPaths, packages: &[String]) -> Result<()> {
    if packages.is_empty() {
        anyhow::bail!("remove requires at least one package")
    }

    status("Removing", "packages from zoxi.toml");
    remove_dependencies(paths.root(), packages)?;
    Ok(())
}

pub fn prepare_dependency_artifacts(
    paths: &ProjectPaths,
    manifest: &ProjectManifest,
    release: bool,
) -> Result<DependencyArtifacts> {
    let project_dependencies = parse_project_dependencies(manifest)?;
    if project_dependencies.is_empty() {
        return Ok(DependencyArtifacts {
            externs: Vec::new(),
            search_dirs: Vec::new(),
            fingerprint: 0,
        });
    }

    let mut resolver = RegistryResolver::new(paths, release);
    let root_ids = project_dependencies
        .iter()
        .map(|dependency| {
            resolver
                .resolve_dependency(dependency)
                .map(|package_id| (dependency.crate_name.clone(), package_id))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut compiled = BTreeMap::new();
    let mut fingerprint_seed = String::new();
    for (_, package_id) in &root_ids {
        let artifact = resolver.compile_package(package_id, &mut compiled)?;
        if !fingerprint_seed.is_empty() {
            fingerprint_seed.push('|');
        }
        fingerprint_seed.push_str(package_id);
        fingerprint_seed.push('|');
        fingerprint_seed.push_str(&artifact.display().to_string());
    }

    let externs = root_ids
        .iter()
        .map(|(extern_name, package_id)| {
            let artifact = compiled.get(package_id).with_context(|| {
                format!("missing compiled artifact for resolved dependency `{package_id}`")
            })?;
            Ok((extern_name.clone(), artifact.clone()))
        })
        .collect::<Result<Vec<_>>>()?;

    let search_dirs = compiled
        .values()
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(DependencyArtifacts {
        externs,
        search_dirs,
        fingerprint: stable_hash_str(&fingerprint_seed),
    })
}

struct RegistryResolver<'a> {
    paths: &'a ProjectPaths,
    release: bool,
    packages: BTreeMap<String, PackageNode>,
}

impl<'a> RegistryResolver<'a> {
    fn new(paths: &'a ProjectPaths, release: bool) -> Self {
        Self {
            paths,
            release,
            packages: BTreeMap::new(),
        }
    }

    fn resolve_dependency(&mut self, dependency: &DependencySpec) -> Result<String> {
        let version = resolve_required_version(self.paths, &dependency.registry_name, &dependency.version_req)?;
        let package_id = package_id(&dependency.registry_name, &version);
        if self.packages.contains_key(&package_id) {
            return Ok(package_id);
        }

        let source_dir = ensure_source_downloaded(self.paths, &dependency.registry_name, &version)?;
        let package = parse_registry_manifest(&source_dir, &dependency.registry_name, version.clone())?;
        let mut resolved_dependencies = Vec::new();
        for child in &package.dependencies {
            resolved_dependencies.push(ResolvedDependency {
                extern_name: child.crate_name.clone(),
                package_id: self.resolve_dependency(child)?,
            });
        }

        self.packages.insert(
            package_id.clone(),
            PackageNode {
                package,
                resolved_dependencies,
            },
        );
        Ok(package_id)
    }

    fn compile_package(
        &self,
        package_id: &str,
        compiled: &mut BTreeMap<String, PathBuf>,
    ) -> Result<PathBuf> {
        if let Some(path) = compiled.get(package_id) {
            return Ok(path.clone());
        }

        let node = self
            .packages
            .get(package_id)
            .with_context(|| format!("missing resolved package `{package_id}`"))?;

        let mut dependency_artifacts = Vec::with_capacity(node.resolved_dependencies.len());
        for dependency in &node.resolved_dependencies {
            dependency_artifacts.push((
                dependency.extern_name.clone(),
                dependency.package_id.clone(),
                self.compile_package(&dependency.package_id, compiled)?,
            ));
        }

        let artifact = compile_registry_package(self.paths, &node.package, self.release, &dependency_artifacts)?;
        compiled.insert(package_id.to_string(), artifact.clone());
        Ok(artifact)
    }
}

struct PackageNode {
    package: RegistryPackage,
    resolved_dependencies: Vec<ResolvedDependency>,
}

struct ResolvedDependency {
    extern_name: String,
    package_id: String,
}

struct RegistryPackage {
    registry_name: String,
    crate_name: String,
    version: Version,
    edition: String,
    root_dir: PathBuf,
    lib_path: PathBuf,
    crate_type: RegistryCrateType,
    dependencies: Vec<DependencySpec>,
}

#[derive(Clone, Copy)]
enum RegistryCrateType {
    Rlib,
    ProcMacro,
}

#[derive(Clone)]
struct DependencySpec {
    registry_name: String,
    crate_name: String,
    version_req: VersionReq,
}

fn compile_registry_package(
    paths: &ProjectPaths,
    package: &RegistryPackage,
    release: bool,
    compiled_dependencies: &[(String, String, PathBuf)],
) -> Result<PathBuf> {
    let profile = if release { "release" } else { "debug" };
    let package_dir = paths
        .registry_cache_dir()?
        .join(&package.registry_name)
        .join(package.version.to_string())
        .join(profile);
    let deps_dir = package_dir.join("deps");
    let incremental_dir = package_dir.join("incremental");
    fs::create_dir_all(&deps_dir)?;
    fs::create_dir_all(&incremental_dir)?;

    let output_path = match package.crate_type {
        RegistryCrateType::Rlib => deps_dir.join(format!("lib{}.rlib", package.crate_name)),
        RegistryCrateType::ProcMacro => deps_dir.join(format!(
            "lib{}.{}",
            package.crate_name,
            dylib_extension()
        )),
    };
    let fingerprint_path = package_dir.join("fingerprint");
    let fingerprint = package_fingerprint(package, release, compiled_dependencies)?;

    if output_path.exists() && read_fingerprint(&fingerprint_path)? == Some(fingerprint) {
        status(
            "Fresh",
            format!("{} v{}", package.registry_name, package.version),
        );
        return Ok(output_path);
    }

    status(
        "Compiling",
        format!("{} v{}", package.registry_name, package.version),
    );
    let mut command = Command::new("rustc");
    command
        .current_dir(&package.root_dir)
        .arg(&package.lib_path)
        .arg("--crate-name")
        .arg(&package.crate_name)
        .arg("--edition")
        .arg(&package.edition)
        .arg("-C")
        .arg(format!("incremental={}", incremental_dir.display()))
        .arg("-C")
        .arg("codegen-units=16")
        .arg("-o")
        .arg(&output_path);

    if release {
        command.arg("-C").arg("opt-level=3");
        command.arg("-C").arg("lto=thin");
    } else {
        command.arg("-C").arg("opt-level=0");
        command.arg("-C").arg("debuginfo=0");
    }

    match package.crate_type {
        RegistryCrateType::Rlib => {
            command.arg("--crate-type=rlib");
        }
        RegistryCrateType::ProcMacro => {
            command.arg("--crate-type=proc-macro");
        }
    }

    let search_dirs = compiled_dependencies
        .iter()
        .filter_map(|(_, _, path)| path.parent().map(Path::to_path_buf))
        .collect::<BTreeSet<_>>();
    for directory in search_dirs {
        command.arg("-L").arg(format!("dependency={}", directory.display()));
    }

    for (extern_name, _, path) in compiled_dependencies {
        command
            .arg("--extern")
            .arg(format!("{extern_name}={}", path.display()));
    }

    let output = command.output().with_context(|| {
        format!(
            "failed to invoke rustc for dependency {} {}",
            package.registry_name, package.version
        )
    })?;
    if !output.status.success() {
        print_command_output(&output)?;
        anyhow::bail!(
            "rustc failed for dependency {} {} with status {}",
            package.registry_name,
            package.version,
            output.status
        )
    }

    write_if_changed(&fingerprint_path, fingerprint.to_string().as_bytes())?;
    Ok(output_path)
}

fn package_fingerprint(
    package: &RegistryPackage,
    release: bool,
    compiled_dependencies: &[(String, String, PathBuf)],
) -> Result<u64> {
    let source = fs::read(&package.lib_path)
        .with_context(|| format!("failed to read {}", package.lib_path.display()))?;
    let mut seed = stable_hash_bytes(&source).to_string();
    seed.push('|');
    seed.push_str(&package.version.to_string());
    seed.push('|');
    seed.push_str(if release { "release" } else { "debug" });
    seed.push('|');
    seed.push_str(&package.edition);
    compiled_dependencies.iter().for_each(|(extern_name, id, path)| {
        seed.push('|');
        seed.push_str(extern_name);
        seed.push('|');
        seed.push_str(id);
        seed.push('|');
        seed.push_str(&path.display().to_string());
    });
    Ok(stable_hash_str(&seed))
}

fn read_fingerprint(path: &Path) -> Result<Option<u64>> {
    if !path.exists() {
        return Ok(None);
    }

    let value = fs::read_to_string(path)?;
    Ok(Some(value.trim().parse()?))
}

fn ensure_source_downloaded(paths: &ProjectPaths, crate_name: &str, version: &Version) -> Result<PathBuf> {
    let source_root = paths
        .source_cache_dir()?
        .join(crate_name)
        .join(version.to_string());
    let manifest_path = source_root.join("Cargo.toml");
    if manifest_path.exists() {
        return Ok(source_root);
    }

    if let Some(parent) = source_root.parent() {
        fs::create_dir_all(parent)?;
    }

    let download_url = format!(
        "https://crates.io/api/v1/crates/{crate_name}/{version}/download"
    );
    status("Downloading", format!("{crate_name} v{version}"));
    let bytes = ureq::get(&download_url)
        .call()
        .map_err(|error| anyhow::anyhow!("failed to download {download_url}: {error}"))?
        .into_body()
        .read_to_vec()
        .map_err(|error| anyhow::anyhow!("failed to read crate download: {error}"))?;

    let temp_dir = source_root.with_extension("tmp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    let decoder = GzDecoder::new(bytes.as_slice());
    let mut archive = Archive::new(decoder);
    archive.unpack(&temp_dir)?;

    let extracted_root = fs::read_dir(&temp_dir)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .with_context(|| format!("archive for {crate_name} {version} was empty"))?;

    if source_root.exists() {
        fs::remove_dir_all(&source_root)?;
    }
    fs::rename(extracted_root, &source_root)?;
    fs::remove_dir_all(temp_dir)?;
    Ok(source_root)
}

fn parse_registry_manifest(source_dir: &Path, crate_name: &str, version: Version) -> Result<RegistryPackage> {
    let manifest_path = source_dir.join("Cargo.toml");
    let content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest: toml::Value = toml::from_str(&content)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

    if source_dir.join("build.rs").exists() {
        anyhow::bail!(
            "dependency `{crate_name}` uses build.rs, which is not supported by the rustc-only dependency pipeline yet"
        )
    }

    let package = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .with_context(|| format!("missing [package] in {}", manifest_path.display()))?;
    let edition = package
        .get("edition")
        .and_then(toml::Value::as_str)
        .unwrap_or("2021")
        .to_string();

    let lib_table = manifest.get("lib").and_then(toml::Value::as_table);
    let lib_rel_path = lib_table
        .and_then(|table| table.get("path"))
        .and_then(toml::Value::as_str)
        .unwrap_or("src/lib.rs");
    let lib_path = source_dir.join(lib_rel_path);
    if !lib_path.exists() {
        anyhow::bail!(
            "dependency `{crate_name}` has no library target at {}",
            lib_path.display()
        )
    }

    let lib_crate_name = lib_table
        .and_then(|table| table.get("name"))
        .and_then(toml::Value::as_str)
        .unwrap_or(crate_name)
        .replace('-', "_");
    let crate_type = if lib_table
        .and_then(|table| table.get("proc-macro"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false)
    {
        RegistryCrateType::ProcMacro
    } else {
        RegistryCrateType::Rlib
    };

    let dependencies = collect_manifest_dependencies(&manifest)?;
    Ok(RegistryPackage {
        registry_name: crate_name.to_string(),
        crate_name: lib_crate_name,
        version,
        edition,
        root_dir: source_dir.to_path_buf(),
        lib_path,
        crate_type,
        dependencies,
    })
}

fn collect_manifest_dependencies(manifest: &toml::Value) -> Result<Vec<DependencySpec>> {
    let mut dependencies = Vec::new();
    if let Some(table) = manifest.get("dependencies").and_then(toml::Value::as_table) {
        for (name, value) in table {
            if let Some(spec) = parse_manifest_dependency(name, value)? {
                dependencies.push(spec);
            }
        }
    }
    Ok(dependencies)
}

fn parse_manifest_dependency(name: &str, value: &toml::Value) -> Result<Option<DependencySpec>> {
    match value {
        toml::Value::String(version) => Ok(Some(DependencySpec {
            registry_name: name.to_string(),
            crate_name: name.replace('-', "_"),
            version_req: VersionReq::parse(version)?,
        })),
        toml::Value::Table(table) => {
            if table
                .get("optional")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false)
            {
                return Ok(None);
            }

            if table
                .get("path")
                .or_else(|| table.get("git"))
                .or_else(|| table.get("workspace"))
                .is_some()
            {
                anyhow::bail!("non-registry dependencies are not supported in rustc-only mode")
            }

            let registry_name = table
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(name)
                .to_string();
            let version_req = table
                .get("version")
                .and_then(toml::Value::as_str)
                .unwrap_or("*");
            Ok(Some(DependencySpec {
                registry_name,
                crate_name: name.replace('-', "_"),
                version_req: VersionReq::parse(version_req)?,
            }))
        }
        _ => Ok(None),
    }
}

fn parse_project_dependencies(manifest: &ProjectManifest) -> Result<Vec<DependencySpec>> {
    if manifest.dependency_section().trim().is_empty() {
        return Ok(Vec::new());
    }

    let content = format!("[dependencies]\n{}\n", manifest.dependency_section());
    let value: toml::Value = toml::from_str(&content)?;
    collect_manifest_dependencies(&value)
}

fn resolve_package_spec(paths: &ProjectPaths, input: &str) -> Result<String> {
    let input = input.trim();
    if input.is_empty() {
        anyhow::bail!("dependency specification cannot be empty")
    }

    if input.contains('@') {
        let (name, version) = input.split_once('@').unwrap_or((input, ""));
        let version = version.trim();
        if version.is_empty() {
            anyhow::bail!("missing version in dependency specification `{input}`")
        }
        return Ok(format!("{} = \"{}\"", name.trim(), version));
    }

    if input.contains('=') {
        let (name, version) = input.split_once('=').unwrap_or((input, ""));
        let version = version.trim().trim_matches('"');
        if version.is_empty() {
            anyhow::bail!("missing version in dependency specification `{input}`")
        }
        return Ok(format!("{} = \"{}\"", name.trim(), version));
    }

    let latest = resolve_latest_version(paths, input)?;
    Ok(format!("{input} = \"{latest}\""))
}

fn resolve_latest_version(paths: &ProjectPaths, crate_name: &str) -> Result<String> {
    let cache_file = paths
        .version_cache_dir()?
        .join(crate_name)
        .join("latest.json");
    if cache_file.exists() {
        let content = fs::read_to_string(&cache_file)?;
        let cached: LatestVersionCache = serde_json::from_str(&content)?;
        if !cached.version.trim().is_empty() {
            return Ok(cached.version);
        }
    }

    let url = format!("https://crates.io/api/v1/crates/{crate_name}");
    status("Updating", format!("crates.io metadata for {crate_name}"));
    let body = ureq::get(&url)
        .call()
        .map_err(|error| anyhow::anyhow!("failed to query {url}: {error}"))?
        .into_body()
        .read_to_string()
        .map_err(|error| anyhow::anyhow!("failed to read crates.io response: {error}"))?;

    let response: CrateResponse = serde_json::from_str(&body)
        .with_context(|| format!("failed to decode crates.io response for `{crate_name}`"))?;

    let version = response
        .krate
        .max_stable_version
        .filter(|value| !value.trim().is_empty())
        .or(response.krate.max_version)
        .with_context(|| format!("crates.io did not return a version for `{crate_name}`"))?;

    if let Some(parent) = cache_file.parent() {
        fs::create_dir_all(parent)?;
    }
    let cache = serde_json::to_string(&LatestVersionCache {
        version: version.clone(),
    })?;
    write_if_changed(&cache_file, cache.as_bytes())?;
    Ok(version)
}

fn resolve_required_version(paths: &ProjectPaths, crate_name: &str, requirement: &VersionReq) -> Result<Version> {
    let cache_file = paths
        .version_cache_dir()?
        .join(crate_name)
        .join("versions.json");
    let versions = if cache_file.exists() {
        let content = fs::read_to_string(&cache_file)?;
        serde_json::from_str::<VersionListCache>(&content)?.versions
    } else {
        let url = format!("https://crates.io/api/v1/crates/{crate_name}/versions");
        status("Updating", format!("version list for {crate_name}"));
        let body = ureq::get(&url)
            .call()
            .map_err(|error| anyhow::anyhow!("failed to query {url}: {error}"))?
            .into_body()
            .read_to_string()
            .map_err(|error| anyhow::anyhow!("failed to read crates.io response: {error}"))?;
        let response: VersionListResponse = serde_json::from_str(&body)?;
        let versions = response
            .versions
            .into_iter()
            .filter(|item| !item.yanked)
            .map(|item| item.num)
            .collect::<Vec<_>>();
        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let cache = serde_json::to_string(&VersionListCache {
            versions: versions.clone(),
        })?;
        write_if_changed(&cache_file, cache.as_bytes())?;
        versions
    };

    let mut parsed = versions
        .into_iter()
        .filter_map(|value| Version::parse(&value).ok())
        .filter(|value| requirement.matches(value))
        .collect::<Vec<_>>();
    parsed.sort();
    parsed.pop().with_context(|| {
        format!(
            "no crates.io version matched `{}` for dependency `{crate_name}`",
            requirement
        )
    })
}

fn package_id(crate_name: &str, version: &Version) -> String {
    format!("{crate_name}@{version}")
}

fn dylib_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(windows) {
        "dll"
    } else {
        "so"
    }
}

#[derive(Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    krate: CrateInfo,
}

#[derive(Deserialize)]
struct CrateInfo {
    max_version: Option<String>,
    max_stable_version: Option<String>,
}

#[derive(Deserialize)]
struct VersionListResponse {
    versions: Vec<VersionInfo>,
}

#[derive(Deserialize)]
struct VersionInfo {
    num: String,
    yanked: bool,
}

#[derive(Deserialize, serde::Serialize)]
struct LatestVersionCache {
    version: String,
}

#[derive(Deserialize, serde::Serialize)]
struct VersionListCache {
    versions: Vec<String>,
}
