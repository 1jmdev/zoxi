use anyhow::Result;
use std::{fs, path::Path};

pub fn write_if_changed(path: &Path, content: &[u8]) -> Result<bool> {
    if let Ok(existing) = fs::read(path)
        && existing == content
    {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)?;
    Ok(true)
}
