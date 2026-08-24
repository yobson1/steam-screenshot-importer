use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, Focusable as _,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Render, RenderOnce,
    Styled as _, Subscription, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable as _,
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
    spinner::Spinner,
};

pub struct GameSearchEvent;

pub struct GameSearch {
    input: Entity<InputState>,
    _subscription: Subscription,
}

impl EventEmitter<GameSearchEvent> for GameSearch {}

impl GameSearch {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search your games…")
                .clean_on_escape()
        });
        let subscription = cx.subscribe(&input, |_, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                cx.emit(GameSearchEvent);
                cx.notify();
            }
        });

        Self {
            input,
            _subscription: subscription,
        }
    }

    pub fn query(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }
}

impl Render for GameSearch {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let blur_input = self.input.clone();
        let focus_input = self.input.clone();

        div()
            .w_full()
            .max_w(px(520.0))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                focus_input.update(cx, |input, cx| input.focus(window, cx));
            })
            .on_mouse_down_out(move |_, window, cx| {
                if blur_input.read(cx).focus_handle(cx).is_focused(window) {
                    window.blur();
                }
            })
            .child(
                Input::new(&self.input)
                    .w_full()
                    .cleanable(true)
                    .aria_label("Search games")
                    .prefix(Icon::new(IconName::Search).small()),
            )
    }
}

#[derive(IntoElement)]
pub struct HomePage {
    welcome_text: String,
    game_search: Entity<GameSearch>,
    show_search: bool,
    cards: Vec<AnyElement>,
    is_loading: bool,
    library_message: Option<String>,
}

impl HomePage {
    pub fn new(
        welcome_text: String,
        game_search: Entity<GameSearch>,
        show_search: bool,
        cards: Vec<AnyElement>,
        is_loading: bool,
        library_message: Option<String>,
    ) -> Self {
        Self {
            welcome_text,
            game_search,
            show_search,
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
            .when(self.show_search, |page| {
                page.child(
                    div()
                        .flex_none()
                        .w_full()
                        .px_5()
                        .pb_6()
                        .flex()
                        .justify_center()
                        .child(self.game_search),
                )
            })
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
