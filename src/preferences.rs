use std::{fs, path::Path};

use gpui::{App, Global};
use gpui_component::ThemeMode;
use log::error;
use rusqlite::{Connection, OptionalExtension as _, params};

use crate::app_dirs::PROJECT_DIRS;
use crate::settings::{ResizeFilter, ScreenshotSettings};

const SELECTED_THEME_KEY: &str = "selected_theme";
const JPEG_QUALITY_KEY: &str = "jpegQuality";
const FILTER_TYPE_KEY: &str = "filterType";
const CHECK_UPDATES_ON_STARTUP_KEY: &str = "checkUpdatesOnStartup";

pub struct Preferences {
    connection: Option<Connection>,
}

impl Global for Preferences {}

impl Preferences {
    fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let connection = Connection::open(path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );",
        )?;

        Ok(Self {
            connection: Some(connection),
        })
    }

    fn unavailable() -> Self {
        Self { connection: None }
    }

    fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let Some(connection) = &self.connection else {
            return Ok(None);
        };

        Ok(connection
            .query_row(
                "SELECT value FROM preferences WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()?)
    }

    fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let Some(connection) = &self.connection else {
            return Ok(());
        };

        connection.execute(
            "INSERT INTO preferences (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    fn screenshot_settings(&self) -> ScreenshotSettings {
        let defaults = ScreenshotSettings::default();
        ScreenshotSettings {
            jpeg_quality: self.read_validated(JPEG_QUALITY_KEY, defaults.jpeg_quality, |value| {
                value.parse().ok().filter(|value| (1..=100).contains(value))
            }),
            resize_filter: self.read_validated(
                FILTER_TYPE_KEY,
                defaults.resize_filter,
                ResizeFilter::from_name,
            ),
            check_updates_on_startup: self.read_validated(
                CHECK_UPDATES_ON_STARTUP_KEY,
                defaults.check_updates_on_startup,
                |value| match value {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                },
            ),
        }
    }

    fn read_validated<T>(
        &self,
        key: &str,
        default: T,
        validate: impl FnOnce(&str) -> Option<T>,
    ) -> T {
        match self.get(key) {
            Ok(Some(value)) => validate(&value).unwrap_or_else(|| {
                error!("Ignoring invalid {key} preference: {value}");
                default
            }),
            Ok(None) => default,
            Err(read_error) => {
                error!("Failed to read {key} preference: {read_error:#}");
                default
            }
        }
    }
}

pub fn init(cx: &mut App) {
    let path = PROJECT_DIRS.config_dir().join("preferences.db");
    let preferences = Preferences::open(&path).unwrap_or_else(|open_error| {
        error!(
            "Failed to open preferences database at {}: {open_error:#}",
            path.display()
        );
        Preferences::unavailable()
    });
    cx.set_global(preferences);
}

pub fn selected_theme(cx: &App) -> Option<ThemeMode> {
    let value = match cx.global::<Preferences>().get(SELECTED_THEME_KEY) {
        Ok(value) => value,
        Err(read_error) => {
            error!("Failed to read selected theme preference: {read_error:#}");
            return None;
        }
    };

    match value.as_deref() {
        Some("light") => Some(ThemeMode::Light),
        Some("dark") => Some(ThemeMode::Dark),
        Some(value) => {
            error!("Ignoring invalid selected theme preference: {value}");
            None
        }
        None => None,
    }
}

pub fn set_selected_theme(cx: &App, mode: ThemeMode) {
    let value = match mode {
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    };
    if let Err(write_error) = cx.global::<Preferences>().set(SELECTED_THEME_KEY, value) {
        error!("Failed to save selected theme preference: {write_error:#}");
    }
}

pub fn screenshot_settings(cx: &App) -> ScreenshotSettings {
    cx.global::<Preferences>().screenshot_settings()
}

pub fn set_jpeg_quality(cx: &App, quality: u8) {
    set(cx, JPEG_QUALITY_KEY, &quality.clamp(1, 100).to_string());
}

pub fn set_resize_filter(cx: &App, filter: ResizeFilter) {
    set(cx, FILTER_TYPE_KEY, filter.name());
}

pub fn set_check_updates_on_startup(cx: &App, enabled: bool) {
    set(
        cx,
        CHECK_UPDATES_ON_STARTUP_KEY,
        if enabled { "true" } else { "false" },
    );
}

fn set(cx: &App, key: &str, value: &str) {
    if let Err(write_error) = cx.global::<Preferences>().set(key, value) {
        error!("Failed to save {key} preference: {write_error:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_are_persisted_between_connections() {
        let temp = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp.path().join("preferences.db");

        Preferences::open(&path)
            .expect("preferences should open")
            .set(SELECTED_THEME_KEY, "dark")
            .expect("preference should be written");

        assert_eq!(
            Preferences::open(&path)
                .expect("preferences should reopen")
                .get(SELECTED_THEME_KEY)
                .expect("preference should be read")
                .as_deref(),
            Some("dark")
        );
    }

    #[test]
    fn screenshot_settings_are_validated_and_persisted() {
        let temp = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp.path().join("preferences.db");
        let preferences = Preferences::open(&path).expect("preferences should open");
        preferences
            .set(JPEG_QUALITY_KEY, "72")
            .expect("quality should be written");
        preferences
            .set(FILTER_TYPE_KEY, "Gaussian")
            .expect("filter should be written");
        preferences
            .set(CHECK_UPDATES_ON_STARTUP_KEY, "false")
            .expect("update preference should be written");
        drop(preferences);

        assert_eq!(
            Preferences::open(&path)
                .expect("preferences should reopen")
                .screenshot_settings(),
            ScreenshotSettings {
                jpeg_quality: 72,
                resize_filter: ResizeFilter::Gaussian,
                check_updates_on_startup: false,
            }
        );
    }

    #[test]
    fn invalid_screenshot_settings_fall_back_to_defaults() {
        let temp = tempfile::tempdir().expect("temporary directory should be created");
        let preferences = Preferences::open(&temp.path().join("preferences.db"))
            .expect("preferences should open");
        preferences
            .set(JPEG_QUALITY_KEY, "101")
            .expect("quality should be written");
        preferences
            .set(FILTER_TYPE_KEY, "Pointy")
            .expect("filter should be written");
        preferences
            .set(CHECK_UPDATES_ON_STARTUP_KEY, "sometimes")
            .expect("update preference should be written");

        assert_eq!(
            preferences.screenshot_settings(),
            ScreenshotSettings::default()
        );
    }
}
