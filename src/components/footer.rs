use gpui::{App, IntoElement, ParentElement as _, Styled as _, div, img, px};
use gpui_component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
};

const BUY_ME_A_COFFEE_URL: &str = "https://buymeacoffee.com/yobson";
const GITHUB_URL: &str = "https://github.com/yobson1";

pub fn footer(cx: &App) -> impl IntoElement {
    div()
        .absolute()
        .bottom_2()
        .left_4()
        .right_4()
        .flex()
        .items_end()
        .justify_between()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
        )
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    Button::new("buy-me-a-coffee")
                        .w(px(48.0))
                        .h(px(48.0))
                        .p_0()
                        .ghost()
                        .tooltip("Buy me a coffee")
                        .accessibility_id("buy-me-a-coffee")
                        .child(img("assets/bmc-logo.svg").size(px(32.0)))
                        .on_click(|_, _, cx| cx.open_url(BUY_ME_A_COFFEE_URL)),
                )
                .child(
                    Button::new("github-profile")
                        .w(px(48.0))
                        .h(px(48.0))
                        .p_0()
                        .ghost()
                        .tooltip("GitHub profile")
                        .accessibility_id("github-profile")
                        .child(Icon::new(IconName::Github).size(px(32.0)))
                        .on_click(|_, _, cx| cx.open_url(GITHUB_URL)),
                ),
        )
}
