use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub fn discover_sources(src_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = WalkDir::new(src_dir)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.into_path();
            let is_zoxi = path.extension().is_some_and(|extension| extension == "zo");
            is_zoxi.then_some(path)
        })
        .collect::<Vec<_>>();

    files.sort();
    Ok(files)
}
