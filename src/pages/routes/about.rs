use gpui::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, ScrollHandle,
    StatefulInteractiveElement as _, Styled as _, div, img, px,
};
use gpui_component::{
    ActiveTheme as _, InteractiveElementExt as _, link::Link, scroll::ScrollableElement as _,
};

const ISSUES_URL: &str = "https://github.com/yobson1/steam-screenshot-importer/issues";

#[derive(IntoElement)]
pub struct AboutPage {
    scroll_handle: ScrollHandle,
}

impl AboutPage {
    pub fn new(scroll_handle: ScrollHandle) -> Self {
        Self { scroll_handle }
    }
}

impl RenderOnce for AboutPage {
    fn render(self, _window: &mut gpui::Window, cx: &mut App) -> impl IntoElement {
        let content = div()
            .w_full()
            .min_h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .px_6()
            .pt_6()
            .pb_4()
            .child(
                div()
                    .text_size(px(40.0))
                    .font_weight(gpui::FontWeight::THIN)
                    .text_color(cx.theme().primary)
                    .child("ABOUT"),
            )
            .child(
                div()
                    .flex_1()
                    .max_w(px(720.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .text_center()
                    .child(
                        "A native Rust application built with GPUI and the Steamworks API to import any image as a screenshot for any game you own.",
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_center()
                            .gap_x_1()
                            .child("Please report bugs and submit feature requests on the")
                            .child(
                                Link::new("about-repository-link")
                                    .href(ISSUES_URL)
                                    .child("GitHub repository."),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("and maybe leave a star? 😳👉💖"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("by yobson with")
                    .child(img("assets/rainbow-heart.svg").size(px(32.0))),
            );

        div()
            .id("about-page")
            .size_full()
            .relative()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .id("about-page-scroll-area")
                    .size_full()
                    .track_scroll(&self.scroll_handle)
                    .overflow_y_scroll()
                    .lock_scroll_axis()
                    .child(content),
            )
            .vertical_scrollbar(&self.scroll_handle)
    }
}
