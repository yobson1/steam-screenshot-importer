use gpui::{App, IntoElement, ParentElement as _, Styled as _, div, px};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Theme, ThemeMode,
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
            .w(px(48.0))
            .h(px(48.0))
            .p_0()
            .ghost()
            .child(Icon::new(icon).size(px(30.0)))
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
