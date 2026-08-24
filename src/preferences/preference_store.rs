use std::{cell::RefCell, fs, path::Path, rc::Rc};

use gpui::Global;
use log::error;
use rusqlite::{Connection, OptionalExtension as _, params};

use super::{
    check_updates_on_startup::CheckUpdatesOnStartupPreference, jpeg_quality::JpegQualityPreference,
    resize_filter::ResizeFilterPreference, theme::ThemePreference,
};

pub trait Preference {
    type Value: Clone;

    fn get(&self) -> Self::Value;
    fn set(&self, value: Self::Value);
    fn default(&self) -> Self::Value;
}

pub(super) struct PreferenceStore {
    connection: Option<Connection>,
}

impl PreferenceStore {
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

    fn read(&self, key: &str) -> anyhow::Result<Option<String>> {
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

    fn write(&self, key: &str, value: &str) -> anyhow::Result<()> {
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

pub(super) fn get_cached<T: Clone>(
    store: &PreferenceStore,
    key: &str,
    value: &RefCell<Option<T>>,
    default: T,
    parse: impl FnOnce(&str) -> Option<T>,
) -> T {
    if let Some(value) = value.borrow().as_ref() {
        return value.clone();
    }

    let loaded = match store.read(key) {
        Ok(Some(raw)) => parse(&raw).unwrap_or_else(|| {
            error!("Ignoring invalid {key} preference: {raw}");
            default
        }),
        Ok(None) => default,
        Err(read_error) => {
            error!("Failed to read {key} preference: {read_error:#}");
            default
        }
    };
    *value.borrow_mut() = Some(loaded.clone());
    loaded
}

pub(super) fn set_cached<T>(
    store: &PreferenceStore,
    key: &str,
    value: &RefCell<Option<T>>,
    new_value: T,
    serialize: impl FnOnce(&T) -> String,
) {
    *value.borrow_mut() = Some(new_value);
    let serialized = {
        let cached = value.borrow();
        serialize(
            cached
                .as_ref()
                .expect("preference cache was just populated"),
        )
    };

    if let Err(write_error) = store.write(key, &serialized) {
        error!("Failed to save {key} preference: {write_error:#}");
    }
}

pub struct Preferences {
    pub theme: ThemePreference,
    pub jpeg_quality: JpegQualityPreference,
    pub resize_filter: ResizeFilterPreference,
    pub check_updates_on_startup: CheckUpdatesOnStartupPreference,
}

impl Global for Preferences {}

impl Preferences {
    pub(super) fn open(path: &Path) -> anyhow::Result<Self> {
        Ok(Self::new(Rc::new(PreferenceStore::open(path)?)))
    }

    pub(super) fn unavailable() -> Self {
        Self::new(Rc::new(PreferenceStore::unavailable()))
    }

    fn new(store: Rc<PreferenceStore>) -> Self {
        Self {
            theme: ThemePreference::new(store.clone()),
            jpeg_quality: JpegQualityPreference::new(store.clone()),
            resize_filter: ResizeFilterPreference::new(store.clone()),
            check_updates_on_startup: CheckUpdatesOnStartupPreference::new(store),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::ResizeFilter;
    use crate::preferences::ThemeSelection;

    #[test]
    fn preferences_are_cached_and_persisted_between_connections() {
        let temp = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp.path().join("preferences.db");
        let preferences = Preferences::open(&path).expect("preferences should open");

        assert_eq!(preferences.jpeg_quality.get(), 95);
        Connection::open(&path)
            .expect("database should open separately")
            .execute(
                "INSERT INTO preferences (key, value) VALUES ('jpegQuality', '20')
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [],
            )
            .expect("database value should be changed");
        assert_eq!(preferences.jpeg_quality.get(), 95);

        preferences.jpeg_quality.set(72);
        preferences.resize_filter.set(ResizeFilter::Gaussian);
        preferences.check_updates_on_startup.set(false);
        preferences.theme.set(ThemeSelection::Dark);
        drop(preferences);

        let reopened = Preferences::open(&path).expect("preferences should reopen");
        assert_eq!(reopened.jpeg_quality.get(), 72);
        assert_eq!(reopened.resize_filter.get(), ResizeFilter::Gaussian);
        assert!(!reopened.check_updates_on_startup.get());
        assert_eq!(reopened.theme.get(), ThemeSelection::Dark);
    }

    #[test]
    fn invalid_preferences_fall_back_to_their_defaults() {
        let temp = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp.path().join("preferences.db");
        let preferences = Preferences::open(&path).expect("preferences should open");
        let connection = Connection::open(&path).expect("database should open separately");
        for (key, value) in [
            ("jpegQuality", "101"),
            ("filterType", "Pointy"),
            ("checkUpdatesOnStartup", "sometimes"),
            ("selected_theme", "purple"),
        ] {
            connection
                .execute(
                    "INSERT INTO preferences (key, value) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )
                .expect("invalid value should be written");
        }

        assert_eq!(
            preferences.jpeg_quality.get(),
            preferences.jpeg_quality.default()
        );
        assert_eq!(
            preferences.resize_filter.get(),
            preferences.resize_filter.default()
        );
        assert_eq!(
            preferences.check_updates_on_startup.get(),
            preferences.check_updates_on_startup.default()
        );
        assert_eq!(preferences.theme.get(), preferences.theme.default());
    }
}
