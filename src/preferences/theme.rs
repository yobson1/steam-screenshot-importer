use std::{cell::RefCell, rc::Rc};

use gpui_component::ThemeMode;

use super::preference_store::{Preference, PreferenceStore, get_cached, set_cached};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeSelection {
    System,
    Light,
    Dark,
}

impl ThemeSelection {
    pub const fn mode(self) -> Option<ThemeMode> {
        match self {
            Self::System => None,
            Self::Light => Some(ThemeMode::Light),
            Self::Dark => Some(ThemeMode::Dark),
        }
    }
}

impl From<ThemeMode> for ThemeSelection {
    fn from(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::Light,
            ThemeMode::Dark => Self::Dark,
        }
    }
}

pub struct ThemePreference {
    key: &'static str,
    value: RefCell<Option<ThemeSelection>>,
    store: Rc<PreferenceStore>,
}

impl ThemePreference {
    pub(super) fn new(store: Rc<PreferenceStore>) -> Self {
        Self {
            key: "selected_theme",
            value: RefCell::new(None),
            store,
        }
    }
}

impl Preference for ThemePreference {
    type Value = ThemeSelection;

    fn get(&self) -> Self::Value {
        get_cached(
            &self.store,
            self.key,
            &self.value,
            self.default(),
            |raw| match raw {
                "light" => Some(ThemeSelection::Light),
                "dark" => Some(ThemeSelection::Dark),
                "system" => Some(ThemeSelection::System),
                _ => None,
            },
        )
    }

    fn set(&self, value: Self::Value) {
        set_cached(&self.store, self.key, &self.value, value, |value| {
            match value {
                ThemeSelection::Light => "light",
                ThemeSelection::Dark => "dark",
                ThemeSelection::System => "system",
            }
            .to_owned()
        });
    }

    fn default(&self) -> Self::Value {
        ThemeSelection::System
    }
}
