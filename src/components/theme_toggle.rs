use gpui::{App, IntoElement, ParentElement as _, Styled as _, div};
use gpui_component::{
    ActiveTheme as _, IconName, Theme, ThemeMode,
    button::{Button, ButtonVariants as _},
};

use crate::preferences;

pub fn theme_toggle(cx: &App) -> impl IntoElement {
    let icon = if cx.theme().is_dark() {
        IconName::Sun
    } else {
        IconName::Moon
    };

    div().absolute().top_3().right_4().child(
        Button::new("theme-toggle")
            .ghost()
            .icon(icon)
            .accessibility_id("theme-toggle")
            .on_click(|_, window, cx| {
                let mode = if cx.theme().is_dark() {
                    ThemeMode::Light
                } else {
                    ThemeMode::Dark
                };
                Theme::change(mode, Some(window), cx);
                preferences::set_selected_theme(cx, mode);
            }),
    )
}
