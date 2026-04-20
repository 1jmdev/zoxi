use anyhow::{Context, Result};
use std::{fs, path::Path};

#[derive(Clone, Debug)]
pub struct ProjectManifest {
    package_name: String,
    edition: String,
}

impl ProjectManifest {
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn crate_name(&self) -> String {
        self.package_name.replace('-', "_")
    }

    pub fn edition(&self) -> &str {
        &self.edition
    }
}

pub fn load_project_manifest(project_root: &Path) -> Result<ProjectManifest> {
    let default_name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("zoxi-app")
        .to_string();
    let manifest_path = project_root.join("zoxi.toml");

    if !manifest_path.exists() {
        return Ok(ProjectManifest {
            package_name: default_name,
            edition: String::from("2024"),
        });
    }

    parse_manifest(&fs::read_to_string(&manifest_path).with_context(|| {
        format!("failed to read {}", manifest_path.display())
    })?, default_name)
}

fn parse_manifest(content: &str, default_name: String) -> Result<ProjectManifest> {
    let mut section = "";
    let mut package_name = None;
    let mut edition = None;

    for raw_line in content.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = &line[1..line.len() - 1];
            continue;
        }

        if section == "dependencies" {
            anyhow::bail!(
                "`zoxi.toml` dependencies are not supported by the rustc-only builder yet"
            )
        }

        if section != "package" {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim().trim_matches('"');
        match key {
            "name" if !value.is_empty() => package_name = Some(value.to_string()),
            "edition" => {
                validate_edition(value)?;
                edition = Some(value.to_string());
            }
            _ => {}
        }
    }

    Ok(ProjectManifest {
        package_name: package_name.unwrap_or(default_name),
        edition: edition.unwrap_or_else(|| String::from("2024")),
    })
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#').map_or(line, |(value, _)| value)
}

fn validate_edition(edition: &str) -> Result<()> {
    match edition {
        "2015" | "2018" | "2021" | "2024" => Ok(()),
        _ => anyhow::bail!("unsupported Rust edition `{edition}` in zoxi.toml"),
    }
}
