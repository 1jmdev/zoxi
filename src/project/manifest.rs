use std::{
    fmt::Write,
    path::Path,
    fs
};
use anyhow::Result;

use crate::project::file_sync::{copy_if_changed,write_if_changed};

pub fn write_generated_manifest(project_root: &Path, generated_dir: &Path) -> Result<()> {
    let source_manifest = project_root.join("zoxi.toml");
    if source_manifest.exists() {
        fs::create_dir_all(generated_dir)?;
        copy_if_changed(&source_manifest, &generated_dir.join("Cargo.toml"))?;
        let source_lock = project_root.join("zoxi.lock");
        if source_lock.exists() {
            copy_if_changed(&source_lock, &generated_dir.join("Cargo.lock"))?;
        } else {
            let generated_lock = generated_dir.join("Cargo.lock");
            if generated_lock.exists() {
                fs::remove_file(generated_lock)?;
            }
        }
        return Ok(());
    }

    let package_name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("zoxi-app");

    let mut manifest = String::new();
    writeln!(&mut manifest, "[package]")?;
    writeln!(&mut manifest, "name = \"{package_name}\"")?;
    writeln!(&mut manifest, "version = \"0.1.0\"")?;
    writeln!(&mut manifest, "edition = \"2024\"")?;
    writeln!(&mut manifest)?;
    writeln!(&mut manifest, "[dependencies]")?;

    fs::create_dir_all(generated_dir)?;
    write_if_changed(&generated_dir.join("Cargo.toml"), manifest.as_bytes())?;
    Ok(())
}

pub fn ensure_project_manifest(project_root: &Path) -> Result<()> {
    let manifest_path = project_root.join("zoxi.toml");
    if manifest_path.exists() {
        return Ok(());
    }

    let package_name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("zoxi-app");

    let mut manifest = String::new();
    writeln!(&mut manifest, "[package]")?;
    writeln!(&mut manifest, "name = \"{package_name}\"")?;
    writeln!(&mut manifest, "version = \"0.1.0\"")?;
    writeln!(&mut manifest, "edition = \"2024\"")?;
    writeln!(&mut manifest)?;
    writeln!(&mut manifest, "[dependencies]")?;

    fs::write(manifest_path, manifest)?;
    Ok(())
}
