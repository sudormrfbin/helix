use helix_loader::{merge_toml_values, read_loadable_toml_names, FlavorLoader};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use toml::Value;

use crate::graphics::{Color, Style};
use crate::Theme;

/// The style of an icon can either be defined by the TOML file, or by the theme.
/// We need to remember that in order to reload the icons colors when the theme changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IconStyle {
    Custom(Style),
    Default(Style),
}

impl Default for IconStyle {
    fn default() -> Self {
        IconStyle::Default(Style::default())
    }
}

impl From<IconStyle> for Style {
    fn from(icon_style: IconStyle) -> Self {
        match icon_style {
            IconStyle::Custom(style) => style,
            IconStyle::Default(style) => style,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Icon {
    #[serde(rename = "icon")]
    pub icon_char: char,
    #[serde(default)]
    #[serde(deserialize_with = "icon_color_to_style", rename = "color")]
    pub style: Option<IconStyle>,
}

impl Icon {
    /// Loads a given style if the icon style is undefined or based on a default value
    pub fn with_default_style(&mut self, style: Style) {
        if self.style.is_none() || matches!(self.style, Some(IconStyle::Default(_))) {
            self.style = Some(IconStyle::Default(style));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Icons {
    pub name: String,
    pub mime_type: Option<HashMap<String, Icon>>,
    pub diagnostic: Diagnostic,
    pub symbol_kind: Option<HashMap<String, Icon>>,
}

impl Icons {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set theme defined styles to diagnostic icons
    pub fn set_diagnostic_icons_base_style(&mut self, theme: &Theme) {
        self.diagnostic.error.with_default_style(theme.get("error"));
        self.diagnostic.info.with_default_style(theme.get("info"));
        self.diagnostic.hint.with_default_style(theme.get("hint"));
        self.diagnostic
            .warning
            .with_default_style(theme.get("warning"));
    }

    /// Set the default style for all icons
    pub fn reset_styles(&mut self) {
        if let Some(mime_type_icons) = &mut self.mime_type {
            for (_, icon) in mime_type_icons.iter_mut() {
                icon.style = Some(IconStyle::Default(Style::default()));
            }
        }
        if let Some(symbol_kind_icons) = &mut self.symbol_kind {
            for (_, icon) in symbol_kind_icons.iter_mut() {
                icon.style = Some(IconStyle::Default(Style::default()));
            }
        }
        self.diagnostic.error.style = Some(IconStyle::Default(Style::default()));
        self.diagnostic.warning.style = Some(IconStyle::Default(Style::default()));
        self.diagnostic.hint.style = Some(IconStyle::Default(Style::default()));
        self.diagnostic.info.style = Some(IconStyle::Default(Style::default()));
    }

    pub fn icon_from_filetype<'a>(&'a self, filetype: &str) -> Option<&'a Icon> {
        if let Some(mime_type_icons) = &self.mime_type {
            mime_type_icons.get(filetype)
        } else {
            None
        }
    }

    /// Returns a reference to an appropriate icon for the specified file path, with a default "file" icon if none is found (if available, otherwise it returns `None`)
    pub fn icon_from_path<'a>(&'a self, filepath: &Path) -> Option<&'a Icon> {
        if let Some(extension_or_filename) = filepath
            .extension()
            .or_else(|| filepath.file_name())
            .and_then(|e| e.to_str())
        {
            if let Some(mime_type_icons) = &self.mime_type {
                match mime_type_icons.get(extension_or_filename) {
                    Some(i) => Some(i),
                    None => {
                        if let Some(symbol_kind_icons) = &self.symbol_kind {
                            symbol_kind_icons.get("file")
                        } else {
                            None
                        }
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
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

fn icon_color_to_style<'de, D>(deserializer: D) -> Result<Option<IconStyle>, D::Error>
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
        Ok(Some(IconStyle::Custom(style)))
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

pub static DEFAULT_ICONS: Lazy<Value> = Lazy::new(|| {
    toml::from_slice(include_bytes!("../../icons.toml")).expect("Failed to parse default icons")
});

pub static DEFAULT_ICONS_DATA: Lazy<Icons> = Lazy::new(|| Icons {
    name: "default".into(),
    ..Icons::from(DEFAULT_ICONS.clone())
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
    pub fn load(
        &self,
        name: &str,
        theme: &Theme,
        true_color: bool,
    ) -> Result<Icons, anyhow::Error> {
        if name == "default" {
            return Ok(self.default(theme));
        }
        let mut icons: Icons = self.load_flavor(name, name, false).map(Icons::from)?;

        // Remove all styles when there is no truecolor support.
        // Not classy, but less cumbersome than trying to pass a parameter to a deserializer.
        if !true_color {
            icons.reset_styles();
        } else {
            icons.set_diagnostic_icons_base_style(theme);
        }

        Ok(Icons {
            name: name.into(),
            ..icons
        })
    }

    /// Lists all icons flavors names available in default and user directory
    pub fn names(&self) -> Vec<String> {
        let mut names = read_loadable_toml_names(&self.user_dir);
        names.extend(read_loadable_toml_names(&self.default_dir));
        names
    }

    /// Returns the default icon flavor.
    /// The `theme` is needed in order to load default styles for diagnostic icons.
    pub fn default(&self, theme: &Theme) -> Icons {
        let mut icons = DEFAULT_ICONS_DATA.clone();
        icons.set_diagnostic_icons_base_style(theme);
        icons
    }
}

impl From<Value> for Icons {
    fn from(value: Value) -> Self {
        // Delete the `inherits` value to prevent cyclic loading
        let toml_str = value
            .to_string()
            .lines()
            .filter(|line| !line.contains("inherits"))
            .collect::<Vec<&str>>()
            .join("\n");
        match toml::from_str(&toml_str) {
            Ok(icons) => icons,
            Err(e) => {
                log::error!("Failed to load icons, falling back to default: {}\n", e);
                DEFAULT_ICONS_DATA.clone()
            }
        }
    }
}

impl FlavorLoader<Icons> for Loader {
    fn user_dir(&self) -> &Path {
        &self.user_dir
    }

    fn default_dir(&self) -> &Path {
        &self.default_dir
    }

    fn log_type_display(&self) -> String {
        "Icons".into()
    }

    fn merge_flavors(
        &self,
        parent_flavor_toml: toml::Value,
        flavor_toml: toml::Value,
    ) -> toml::Value {
        merge_toml_values(parent_flavor_toml, flavor_toml, 3)
    }

    fn default_data(&self, name: &str) -> Option<Value> {
        if name == "default" {
            Some(DEFAULT_ICONS.clone())
        } else {
            None
        }
    }
}
