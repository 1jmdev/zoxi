use std::{
    path::Path,
    fs
};
use anyhow::Result;

pub fn write_if_changed(path: &Path, content: &[u8]) -> Result<bool> {
    if let Ok(existing) = fs::read(path) {
        if existing == content {
            return Ok(false);
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)?;
    Ok(true)
}

pub fn copy_if_changed(source: &Path, destination: &Path) -> Result<bool> {
    let content = fs::read(source)?;
    write_if_changed(destination, &content)
}
