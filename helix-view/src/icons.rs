use anyhow::Context;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::graphics::{Color, Style};
use crate::Theme;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Icon {
    #[serde(rename = "icon")]
    pub icon_char: char,
    #[serde(default)]
    #[serde(deserialize_with = "icon_color_to_style", rename = "color")]
    pub style: Option<Style>,
}

impl Icon {
    pub fn plain(icon_char: char) -> Self {
        Self {
            icon_char,
            style: None,
        }
    }

    pub fn with_base_style(&mut self, style: Style) {
        if self.style.is_none() {
            self.style = Some(style);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Icons {
    pub mime_type: Option<HashMap<String, Icon>>,
    pub diagnostic: Diagnostic,
    pub symbol_kind: Option<SymbolKind>,
}

impl Icons {
    pub fn set_diagnostic_icons_base_style(mut self, theme: &Theme) -> Self {
        self.diagnostic.error.with_base_style(theme.get("error"));
        self.diagnostic.info.with_base_style(theme.get("info"));
        self.diagnostic.hint.with_base_style(theme.get("hint"));
        self.diagnostic
            .warning
            .with_base_style(theme.get("warning"));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Diagnostic {
    pub error: Icon,
    pub warning: Icon,
    pub info: Icon,
    pub hint: Icon,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SymbolKind {
    pub file: Icon,
    pub module: Icon,
    pub namespace: Icon,
    pub package: Icon,
    pub class: Icon,
    pub method: Icon,
    pub property: Icon,
    pub field: Icon,
    pub constructor: Icon,
    pub enumeration: Icon,
    pub interface: Icon,
    pub function: Icon,
    pub variable: Icon,
    pub constant: Icon,
    pub string: Icon,
    pub number: Icon,
    pub boolean: Icon,
    pub array: Icon,
    pub object: Icon,
    pub key: Icon,
    pub null: Icon,
    pub enum_member: Icon,
    pub structure: Icon,
    pub event: Icon,
    pub operator: Icon,
    pub type_parameter: Icon,
}

fn icon_color_to_style<'de, D>(deserializer: D) -> Result<Option<Style>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let mut style = Style::default();
    if !s.is_empty() {
        match hex_string_to_rgb(s) {
            Ok(c) => {
                style = style.fg(c);
            }
            Err(e) => {
                log::error!("{}", e);
            }
        };
        Ok(Some(style))
    } else {
        Ok(None)
    }
}

pub fn hex_string_to_rgb(s: &str) -> Result<Color, String> {
    if s.starts_with('#') && s.len() >= 7 {
        if let (Ok(red), Ok(green), Ok(blue)) = (
            u8::from_str_radix(&s[1..3], 16),
            u8::from_str_radix(&s[3..5], 16),
            u8::from_str_radix(&s[5..7], 16),
        ) {
            return Ok(Color::Rgb(red, green, blue));
        }
    }
    Err(format!("Icon color: malformed hexcode: {}", s))
}

pub struct Loader {
    user_dir: PathBuf,
    default_dir: PathBuf,
}

pub static DEFAULT_ICONS: Lazy<Icons> = Lazy::new(|| {
    toml::from_slice(include_bytes!("../../icons.toml")).expect("Failed to parse default icons")
});

impl Loader {
    /// Creates a new loader that can load icons flavors from two directories.
    pub fn new<P: AsRef<Path>>(user_dir: P, default_dir: P) -> Self {
        Self {
            user_dir: user_dir.as_ref().join("icons"),
            default_dir: default_dir.as_ref().join("icons"),
        }
    }

    /// Loads icons flavors first looking in the `user_dir` then in `default_dir`.
    /// The `theme` is needed in order to load default styles for diagnostic icons.
    pub fn load(&self, name: &str, theme: &Theme) -> Result<Icons, anyhow::Error> {
        if name == "default" {
            return Ok(self.default(theme));
        }
        let filename = format!("{}.toml", name);

        let user_path = self.user_dir.join(&filename);
        let path = if user_path.exists() {
            user_path
        } else {
            self.default_dir.join(filename)
        };

        let data = std::fs::read(&path)?;
        toml::from_slice(data.as_slice())
            .map(|icons: Icons| icons.set_diagnostic_icons_base_style(theme))
            .context("Failed to deserialize icon")
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

    /// Returns the default icon flavor.
    /// The `theme` is needed in order to load default styles for diagnostic icons.
    pub fn default(&self, theme: &Theme) -> Icons {
        DEFAULT_ICONS.clone().set_diagnostic_icons_base_style(theme)
    }
}
