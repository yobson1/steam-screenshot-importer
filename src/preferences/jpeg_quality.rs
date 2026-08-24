use std::{cell::RefCell, rc::Rc};

use super::preference_store::{Preference, PreferenceStore, get_cached, set_cached};

pub struct JpegQualityPreference {
    key: &'static str,
    value: RefCell<Option<u8>>,
    store: Rc<PreferenceStore>,
}

impl JpegQualityPreference {
    pub(super) fn new(store: Rc<PreferenceStore>) -> Self {
        Self {
            key: "jpegQuality",
            value: RefCell::new(None),
            store,
        }
    }
}

impl Preference for JpegQualityPreference {
    type Value = u8;

    fn get(&self) -> Self::Value {
        get_cached(&self.store, self.key, &self.value, self.default(), |raw| {
            raw.parse().ok().filter(|value| (1..=100).contains(value))
        })
    }

    fn set(&self, value: Self::Value) {
        set_cached(
            &self.store,
            self.key,
            &self.value,
            value.clamp(1, 100),
            u8::to_string,
        );
    }

    fn default(&self) -> Self::Value {
        95
    }
}
