pub mod check_updates_on_startup;
pub mod jpeg_quality;
mod preference_store;
pub mod resize_filter;
pub mod theme;

use gpui::App;
use log::error;

use crate::app_dirs::PROJECT_DIRS;

pub use preference_store::{Preference, Preferences};
pub use resize_filter::ResizeFilter;
pub use theme::ThemeSelection;

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
