use gpui::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    Styled as _, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{ActiveTheme as _, scroll::ScrollableElement as _, spinner::Spinner};

#[derive(IntoElement)]
pub struct HomePage {
    welcome_text: String,
    cards: Vec<AnyElement>,
    is_loading: bool,
    library_message: Option<String>,
}

impl HomePage {
    pub fn new(
        welcome_text: String,
        cards: Vec<AnyElement>,
        is_loading: bool,
        library_message: Option<String>,
    ) -> Self {
        Self {
            welcome_text,
            cards,
            is_loading,
            library_message,
        }
    }
}

impl RenderOnce for HomePage {
    fn render(self, _window: &mut gpui::Window, cx: &mut App) -> impl IntoElement {
        let background = cx.theme().background;
        let foreground = cx.theme().foreground;
        let primary = cx.theme().primary;
        let muted_foreground = cx.theme().muted_foreground;

        div()
            .id("main-page-scroll")
            .size_full()
            .flex()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .child(
                div()
                    .flex_none()
                    .w_full()
                    .pt_6()
                    .pb_2()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(40.0))
                            .font_weight(gpui::FontWeight::THIN)
                            .text_color(primary)
                            .child(self.welcome_text),
                    ),
            )
            .child(
                div()
                    .id("game-library")
                    .w_full()
                    .px_5()
                    .pb_8()
                    .flex()
                    .flex_wrap()
                    .justify_center()
                    .items_start()
                    .when(self.is_loading, |gallery| {
                        gallery.child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .gap_2()
                                .text_sm()
                                .text_color(muted_foreground)
                                .child(Spinner::new().color(primary))
                                .child("Fetching games."),
                        )
                    })
                    .when_some(self.library_message, |gallery, message| {
                        gallery.child(
                            div()
                                .w_full()
                                .text_center()
                                .text_sm()
                                .text_color(muted_foreground)
                                .child(message),
                        )
                    })
                    .children(self.cards),
            )
            .overflow_y_scrollbar()
    }
}
