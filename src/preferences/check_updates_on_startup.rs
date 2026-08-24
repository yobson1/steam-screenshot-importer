use std::{cell::RefCell, rc::Rc};

use super::preference_store::{Preference, PreferenceStore, get_cached, set_cached};

pub struct CheckUpdatesOnStartupPreference {
    key: &'static str,
    value: RefCell<Option<bool>>,
    store: Rc<PreferenceStore>,
}

impl CheckUpdatesOnStartupPreference {
    pub(super) fn new(store: Rc<PreferenceStore>) -> Self {
        Self {
            key: "checkUpdatesOnStartup",
            value: RefCell::new(None),
            store,
        }
    }
}

impl Preference for CheckUpdatesOnStartupPreference {
    type Value = bool;

    fn get(&self) -> Self::Value {
        get_cached(
            &self.store,
            self.key,
            &self.value,
            self.default(),
            |raw| match raw {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            },
        )
    }

    fn set(&self, value: Self::Value) {
        set_cached(&self.store, self.key, &self.value, value, bool::to_string);
    }

    fn default(&self) -> Self::Value {
        true
    }
}
