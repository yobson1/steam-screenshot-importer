use std::{fs, path::Path};

use gpui::{App, Global};
use gpui_component::ThemeMode;
use log::error;
use rusqlite::{Connection, OptionalExtension as _, params};

use crate::app_dirs::PROJECT_DIRS;

const SELECTED_THEME_KEY: &str = "selected_theme";

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
}
