use anyhow::Context;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Diagnostic {
    pub error: char,
    pub warning: char,
    pub info: char,
    pub notice: char,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SymbolKind {
    pub variable: char,
    pub function: char,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Icons {
    mime_type: HashMap<String, char>,
    pub diagnostic: Diagnostic,
    pub symbol_kind: SymbolKind,
}

pub struct Loader {
    user_dir: PathBuf,
    default_dir: PathBuf,
}

pub static DEFAULT_ICONS: Lazy<Icons> = Lazy::new(|| {
    toml::from_slice(include_bytes!("../../icons.toml")).expect("Failed to parse default icons")
});

impl Icons {
    pub fn mimetype_icon_for_path(&self, path: &Path) -> Option<&char> {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            self.mime_type.get(extension)
        } else {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                self.mime_type.get(filename)
            } else {
                None
            }
        }
    }
}

impl Loader {
    /// Creates a new loader that can load icons flavors from two directories.
    pub fn new<P: AsRef<Path>>(user_dir: P, default_dir: P) -> Self {
        Self {
            user_dir: user_dir.as_ref().join("icons"),
            default_dir: default_dir.as_ref().join("icons"),
        }
    }

    /// Loads icons flavors first looking in the `user_dir` then in `default_dir`
    pub fn load(&self, name: &str) -> Result<Icons, anyhow::Error> {
        if name == "default" {
            return Ok(self.default());
        }
        let filename = format!("{}.toml", name);

        let user_path = self.user_dir.join(&filename);
        let path = if user_path.exists() {
            user_path
        } else {
            self.default_dir.join(filename)
        };

        let data = std::fs::read(&path)?;
        toml::from_slice(data.as_slice()).context("Failed to deserialize icon")
    }

    pub fn read_names(path: &Path) -> Vec<String> {
        std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let path = entry.path();
                        (path.extension()? == "toml")
                            .then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Lists all icons flavors names available in default and user directory
    pub fn names(&self) -> Vec<String> {
        let mut names = Self::read_names(&self.user_dir);
        names.extend(Self::read_names(&self.default_dir));
        names
    }

    /// Returns the default icon flavor
    pub fn default(&self) -> Icons {
        DEFAULT_ICONS.clone()
    }
}
