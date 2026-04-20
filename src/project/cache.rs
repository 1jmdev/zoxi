use anyhow::{Result, bail};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::project::file_sync::write_if_changed;
use crate::project::stable_hash_bytes;

const CACHE_VERSION: &str = "v1";

#[derive(Clone)]
pub struct SourceFingerprint {
    size: u64,
    hash: u64,
}

impl SourceFingerprint {
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = fs::read(path)?;
        Ok(Self {
            size: content.len() as u64,
            hash: stable_hash_bytes(&content),
        })
    }
}

#[derive(Clone)]
pub struct CacheEntry {
    generated_path: PathBuf,
    fingerprint: SourceFingerprint,
}

impl CacheEntry {
    pub fn new(generated_path: PathBuf, fingerprint: SourceFingerprint) -> Self {
        Self {
            generated_path,
            fingerprint,
        }
    }

    pub fn generated_path(&self) -> &Path {
        &self.generated_path
    }

    pub fn matches(&self, fingerprint: &SourceFingerprint) -> bool {
        self.fingerprint.size == fingerprint.size
            && self.fingerprint.hash == fingerprint.hash
    }
}

pub struct CacheState {
    entries: BTreeMap<PathBuf, CacheEntry>,
}

impl CacheState {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&CacheEntry> {
        self.entries.get(path)
    }

    pub fn insert(&mut self, path: PathBuf, entry: CacheEntry) {
        self.entries.insert(path, entry);
    }

    pub fn entries(&self) -> impl Iterator<Item = (&PathBuf, &CacheEntry)> {
        self.entries.iter()
    }
}

pub fn load_cache_state(path: &Path) -> Result<CacheState> {
    if !path.exists() {
        return Ok(CacheState::new());
    }

    match parse_cache_state(path) {
        Ok(state) => Ok(state),
        Err(_) => Ok(CacheState::new()),
    }
}

fn parse_cache_state(path: &Path) -> Result<CacheState> {
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines();
    if lines.next() != Some(CACHE_VERSION) {
        return Ok(CacheState::new());
    }

    let mut state = CacheState::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }

        let mut parts = line.split('\t');
        let Some(source_path) = parts.next() else {
            return Ok(CacheState::new());
        };
        let Some(generated_path) = parts.next() else {
            return Ok(CacheState::new());
        };
        let Some(size) = parts.next() else {
            return Ok(CacheState::new());
        };
        let Some(hash) = parts.next() else {
            return Ok(CacheState::new());
        };
        if parts.next().is_some() {
            return Ok(CacheState::new());
        }

        state.insert(
            PathBuf::from(unescape(source_path)?),
            CacheEntry::new(
                PathBuf::from(unescape(generated_path)?),
                SourceFingerprint {
                    size: size.parse()?,
                    hash: hash.parse()?,
                },
            ),
        );
    }

    Ok(state)
}

pub fn write_cache_state(path: &Path, state: &CacheState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut content = String::from(CACHE_VERSION);
    content.push('\n');

    for (source_path, entry) in state.entries() {
        content.push_str(&escape(&source_path.to_string_lossy()));
        content.push('\t');
        content.push_str(&escape(&entry.generated_path.display().to_string()));
        content.push('\t');
        content.push_str(&entry.fingerprint.size.to_string());
        content.push('\t');
        content.push_str(&entry.fingerprint.hash.to_string());
        content.push('\n');
    }

    write_if_changed(path, content.as_bytes())?;
    Ok(())
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn unescape(value: &str) -> Result<String> {
    let mut unescaped = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            unescaped.push(character);
            continue;
        }

        let Some(escaped) = chars.next() else {
            bail!("invalid cache entry");
        };

        match escaped {
            '\\' => unescaped.push('\\'),
            't' => unescaped.push('\t'),
            'n' => unescaped.push('\n'),
            'r' => unescaped.push('\r'),
            _ => bail!("invalid cache entry"),
        }
    }

    Ok(unescaped)
}
