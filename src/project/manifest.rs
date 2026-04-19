use std::fmt::Write;
use std::path::Path;

pub fn write_generated_manifest(project_root: &Path, generated_dir: &Path) -> anyhow::Result<()> {
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

    std::fs::create_dir_all(generated_dir)?;
    std::fs::write(generated_dir.join("Cargo.toml"), manifest)?;
    Ok(())
}
