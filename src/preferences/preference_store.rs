use std::{cell::RefCell, fmt::Display, fs, path::Path, rc::Rc, str::FromStr};

use gpui::Global;
use log::error;
use rusqlite::{Connection, OptionalExtension as _, params};

use super::{ResizeFilter, ThemeSelection};

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

pub struct Preference<T> {
    key: &'static str,
    value: RefCell<Option<T>>,
    default: T,
    validate: fn(&T) -> bool,
    normalize: fn(T) -> T,
    store: Rc<PreferenceStore>,
}

impl<T: Clone + Display + FromStr> Preference<T> {
    fn new(store: Rc<PreferenceStore>, key: &'static str, default: T) -> Self {
        Self {
            key,
            value: RefCell::new(None),
            default,
            validate: always_valid,
            normalize: identity,
            store,
        }
    }

    fn validated_by(mut self, validate: fn(&T) -> bool) -> Self {
        assert!(
            validate(&self.default),
            "default for {} preference must be valid",
            self.key
        );
        self.validate = validate;
        self
    }

    fn normalized_by(mut self, normalize: fn(T) -> T) -> Self {
        self.normalize = normalize;
        self
    }

    pub fn get(&self) -> T {
        if let Some(value) = self.value.borrow().as_ref() {
            return value.clone();
        }

        let loaded = match self.store.read(self.key) {
            Ok(Some(raw)) => raw
                .parse()
                .ok()
                .filter(|value| (self.validate)(value))
                .unwrap_or_else(|| {
                    error!("Ignoring invalid {} preference: {raw}", self.key);
                    self.default()
                }),
            Ok(None) => self.default(),
            Err(read_error) => {
                error!("Failed to read {} preference: {read_error:#}", self.key);
                self.default()
            }
        };
        *self.value.borrow_mut() = Some(loaded.clone());
        loaded
    }

    pub fn set(&self, value: T) {
        let value = (self.normalize)(value);
        if !(self.validate)(&value) {
            error!("Ignoring invalid {} preference", self.key);
            return;
        }

        let serialized = value.to_string();
        *self.value.borrow_mut() = Some(value);

        if let Err(write_error) = self.store.write(self.key, &serialized) {
            error!("Failed to save {} preference: {write_error:#}", self.key);
        }
    }

    pub fn default(&self) -> T {
        self.default.clone()
    }
}

fn always_valid<T>(_: &T) -> bool {
    true
}

fn identity<T>(value: T) -> T {
    value
}

pub struct Preferences {
    pub theme: Preference<ThemeSelection>,
    pub jpeg_quality: Preference<u8>,
    pub resize_filter: Preference<ResizeFilter>,
    pub check_updates_on_startup: Preference<bool>,
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
            theme: Preference::new(Rc::clone(&store), "selected_theme", ThemeSelection::System),
            jpeg_quality: Preference::new(Rc::clone(&store), "jpegQuality", 95)
                .validated_by(|value| (1..=100).contains(value))
                .normalized_by(|value| value.clamp(1, 100)),
            resize_filter: Preference::new(Rc::clone(&store), "filterType", ResizeFilter::Lanczos3),
            check_updates_on_startup: Preference::new(store, "checkUpdatesOnStartup", true),
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

    #[test]
    fn values_are_normalized_before_they_are_cached_and_persisted() {
        let temp = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp.path().join("preferences.db");
        let preferences = Preferences::open(&path).expect("preferences should open");

        preferences.jpeg_quality.set(0);
        assert_eq!(preferences.jpeg_quality.get(), 1);

        preferences.jpeg_quality.set(101);
        assert_eq!(preferences.jpeg_quality.get(), 100);
        drop(preferences);

        let reopened = Preferences::open(&path).expect("preferences should reopen");
        assert_eq!(reopened.jpeg_quality.get(), 100);
    }
}
