use std::{cell::RefCell, rc::Rc};

use super::preference_store::{Preference, PreferenceStore, get_cached, set_cached};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl ResizeFilter {
    pub const ALL: [Self; 5] = [
        Self::Nearest,
        Self::Triangle,
        Self::CatmullRom,
        Self::Gaussian,
        Self::Lanczos3,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Nearest => "Nearest",
            Self::Triangle => "Triangle",
            Self::CatmullRom => "CatmullRom",
            Self::Gaussian => "Gaussian",
            Self::Lanczos3 => "Lanczos3",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Nearest => "Nearest Neighbor: fastest, blockiest",
            Self::Triangle => "Triangle: bilinear",
            Self::CatmullRom => "Catmull-Rom: bicubic",
            Self::Gaussian => "Gaussian",
            Self::Lanczos3 => "Lanczos3: best quality, slowest",
        }
    }

    fn from_name(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|filter| filter.name() == value)
    }
}

pub struct ResizeFilterPreference {
    key: &'static str,
    value: RefCell<Option<ResizeFilter>>,
    store: Rc<PreferenceStore>,
}

impl ResizeFilterPreference {
    pub(super) fn new(store: Rc<PreferenceStore>) -> Self {
        Self {
            key: "filterType",
            value: RefCell::new(None),
            store,
        }
    }
}

impl Preference for ResizeFilterPreference {
    type Value = ResizeFilter;

    fn get(&self) -> Self::Value {
        get_cached(
            &self.store,
            self.key,
            &self.value,
            self.default(),
            ResizeFilter::from_name,
        )
    }

    fn set(&self, value: Self::Value) {
        set_cached(&self.store, self.key, &self.value, value, |filter| {
            filter.name().to_owned()
        });
    }

    fn default(&self) -> Self::Value {
        ResizeFilter::Lanczos3
    }
}
