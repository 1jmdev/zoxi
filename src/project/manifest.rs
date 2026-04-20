use anyhow::{Context, Result};
use std::{fmt::Write, fs, path::Path};

use crate::project::file_sync::write_if_changed;

#[derive(Clone, Debug)]
pub struct ProjectManifest {
    package_name: String,
    edition: String,
    dependency_section: String,
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

    pub fn dependency_section(&self) -> &str {
        &self.dependency_section
    }
}

pub fn load_project_manifest(project_root: &Path) -> Result<ProjectManifest> {
    let default_name = default_package_name(project_root);
    let manifest_path = project_root.join("zoxi.toml");

    if !manifest_path.exists() {
        return Ok(ProjectManifest {
            package_name: default_name,
            edition: String::from("2024"),
            dependency_section: String::new(),
        });
    }

    parse_manifest(
        &fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?,
        default_name,
    )
}

pub fn add_dependencies(project_root: &Path, packages: &[String]) -> Result<()> {
    if packages.is_empty() {
        anyhow::bail!("add requires at least one package")
    }

    let manifest_path = project_root.join("zoxi.toml");
    let mut content = read_or_create_manifest(&manifest_path, project_root)?;
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();

    let (start, end) = dependency_section_range(&lines);
    let insert_at = if let Some((_, end)) = start.zip(end) {
        end
    } else {
        if !lines.is_empty() && lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(String::from("[dependencies]"));
        lines.len()
    };

    let mut updates = packages
        .iter()
        .map(|package| parse_dependency_spec(package))
        .collect::<Result<Vec<_>>>()?;
    updates.sort_by(|left, right| left.name.cmp(&right.name));

    for dependency in updates {
        let rendered = dependency.render_line();
        if let Some(index) = find_dependency_line(&lines, &dependency.name) {
            lines[index] = rendered;
            continue;
        }

        let position = lines[insert_at..]
            .iter()
            .position(|line| {
                dependency_line_name(line).is_some_and(|name| name > dependency.name.as_str())
            })
            .map_or(insert_at, |offset| insert_at + offset);
        lines.insert(position, rendered);
    }

    content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    write_if_changed(&manifest_path, content.as_bytes())?;
    Ok(())
}

pub fn remove_dependencies(project_root: &Path, packages: &[String]) -> Result<()> {
    if packages.is_empty() {
        anyhow::bail!("remove requires at least one package")
    }

    let manifest_path = project_root.join("zoxi.toml");
    if !manifest_path.exists() {
        anyhow::bail!("cannot remove dependencies, zoxi.toml does not exist")
    }

    let content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    let names = packages
        .iter()
        .map(|package| parse_dependency_name(package))
        .collect::<Result<Vec<_>>>()?;

    lines.retain(|line| {
        dependency_line_name(line)
            .map(|name| !names.iter().any(|candidate| candidate == &name))
            .unwrap_or(true)
    });

    let content = lines.join("\n");
    let content = if content.is_empty() {
        String::new()
    } else {
        format!("{content}\n")
    };
    write_if_changed(&manifest_path, content.as_bytes())?;
    Ok(())
}

fn parse_manifest(content: &str, default_name: String) -> Result<ProjectManifest> {
    let mut section = "";
    let mut package_name = None;
    let mut edition = None;
    let mut dependency_lines = Vec::new();

    for raw_line in content.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            if section == "dependencies" {
                dependency_lines.push(String::new());
            }
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = &line[1..line.len() - 1];
            continue;
        }

        if section == "package" {
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
            continue;
        }

        if section == "dependencies" {
            dependency_lines.push(raw_line.to_string());
        }
    }

    Ok(ProjectManifest {
        package_name: package_name.unwrap_or(default_name),
        edition: edition.unwrap_or_else(|| String::from("2024")),
        dependency_section: dependency_lines.join("\n"),
    })
}

fn read_or_create_manifest(manifest_path: &Path, project_root: &Path) -> Result<String> {
    if manifest_path.exists() {
        return fs::read_to_string(manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()));
    }

    let mut manifest = String::new();
    writeln!(&mut manifest, "[package]")?;
    writeln!(&mut manifest, "name = \"{}\"", default_package_name(project_root))?;
    writeln!(&mut manifest, "version = \"0.1.0\"")?;
    writeln!(&mut manifest, "edition = \"2024\"")?;
    writeln!(&mut manifest)?;
    writeln!(&mut manifest, "[dependencies]")?;
    Ok(manifest)
}

fn default_package_name(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("zoxi-app")
        .to_string()
}

fn dependency_section_range(lines: &[String]) -> (Option<usize>, Option<usize>) {
    let start = lines
        .iter()
        .position(|line| strip_comment(line).trim() == "[dependencies]")
        .map(|index| index + 1);
    let end = start.map(|start_index| {
        lines[start_index..]
            .iter()
            .position(|line| {
                let trimmed = strip_comment(line).trim();
                trimmed.starts_with('[') && trimmed.ends_with(']')
            })
            .map_or(lines.len(), |offset| start_index + offset)
    });
    (start, end)
}

fn find_dependency_line(lines: &[String], name: &str) -> Option<usize> {
    let (Some(start), Some(end)) = dependency_section_range(lines) else {
        return None;
    };

    lines[start..end]
        .iter()
        .position(|line| dependency_line_name(line) == Some(name))
        .map(|offset| start + offset)
}

fn dependency_line_name(line: &str) -> Option<&str> {
    let trimmed = strip_comment(line).trim();
    if trimmed.is_empty() || trimmed.starts_with('[') {
        return None;
    }

    trimmed
        .split_once('=')
        .map(|(name, _)| name.trim())
        .filter(|name| !name.is_empty())
}

fn parse_dependency_name(input: &str) -> Result<String> {
    let input = input.trim();
    if input.is_empty() {
        anyhow::bail!("dependency name cannot be empty")
    }

    Ok(input
        .split(['@', '='])
        .next()
        .unwrap_or(input)
        .trim()
        .to_string())
}

fn parse_dependency_spec(input: &str) -> Result<DependencySpec> {
    let input = input.trim();
    if input.is_empty() {
        anyhow::bail!("dependency specification cannot be empty")
    }

    if let Some((name, version)) = input.split_once('@') {
        return Ok(DependencySpec {
            name: parse_dependency_name(name)?,
            version: Some(version.trim().to_string()).filter(|value| !value.is_empty()),
        });
    }

    if let Some((name, version)) = input.split_once('=') {
        return Ok(DependencySpec {
            name: parse_dependency_name(name)?,
            version: Some(version.trim().trim_matches('"').to_string())
                .filter(|value| !value.is_empty()),
        });
    }

    Ok(DependencySpec {
        name: parse_dependency_name(input)?,
        version: None,
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

struct DependencySpec {
    name: String,
    version: Option<String>,
}

impl DependencySpec {
    fn render_line(&self) -> String {
        match &self.version {
            Some(version) => format!("{} = \"{}\"", self.name, version),
            None => format!("{} = \"*\"", self.name),
        }
    }
}
