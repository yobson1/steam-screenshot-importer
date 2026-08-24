use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, AppContext as _, ClickEvent, Context, ElementId,
    EventEmitter, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Render,
    RenderOnce, Size, StatefulInteractiveElement as _, Styled as _, Transformation, Window, div,
    ease_in_out, percentage, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, StyledExt as _, WindowExt as _,
    button::{Button, ButtonRounded, ButtonVariants as _},
    dialog::{Dialog, DialogButtonProps},
    input::{Input, InputState},
};

pub const MENU_WIDTH: f32 = 136.0;
const MENU_ANIMATION_DURATION: Duration = Duration::from_millis(525);

fn parse_app_id(value: &str) -> Option<u32> {
    value.trim().parse().ok()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavItem {
    Home,
    AppId,
    About,
    Options,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuEvent {
    Navigate(NavItem),
    CustomAppId(u32),
}

impl NavItem {
    #[cfg(test)]
    pub const ALL: [Self; 4] = [Self::Home, Self::AppId, Self::About, Self::Options];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::AppId => "App ID",
            Self::About => "About",
            Self::Options => "Options",
        }
    }

    pub const fn accessibility_id(self) -> &'static str {
        match self {
            Self::Home => "nav-home",
            Self::AppId => "nav-app-id",
            Self::About => "nav-about",
            Self::Options => "nav-options",
        }
    }

    pub const fn icon(self) -> IconName {
        match self {
            Self::Home => IconName::LayoutDashboard,
            Self::AppId => IconName::Plus,
            Self::About => IconName::Info,
            Self::Options => IconName::Settings,
        }
    }

    pub const fn spins(self) -> bool {
        !matches!(self, Self::Home)
    }
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type HoverHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct NavButton {
    item: NavItem,
    hovered: bool,
    on_click: Option<ClickHandler>,
    on_hover: HoverHandler,
}

impl NavButton {
    pub fn new(
        item: NavItem,
        hovered: bool,
        on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            item,
            hovered,
            on_click: None,
            on_hover: Rc::new(on_hover),
        }
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    fn render_icon(&self) -> AnyElement {
        let icon = Icon::new(self.item.icon()).size(px(25.0));
        if self.hovered && self.item.spins() {
            icon.with_animation(
                ElementId::Name(format!("nav-spin-{}", self.item.accessibility_id()).into()),
                Animation::new(Duration::from_millis(750)).with_easing(ease_in_out),
                |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
            )
            .into_any_element()
        } else {
            icon.into_any_element()
        }
    }
}

impl RenderOnce for NavButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let icon = self.render_icon();
        let mut button = Button::new(self.item.accessibility_id())
            .w(px(64.0))
            .h(px(64.0))
            .p_0()
            .rounded(ButtonRounded::Large)
            .accessibility_id(self.item.accessibility_id())
            .on_hover(move |hovered, window, cx| (self.on_hover)(hovered, window, cx))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .child(icon)
                    .child(div().text_xs().font_semibold().child(self.item.label())),
            );

        if let Some(on_click) = self.on_click {
            button = button.on_click(move |event, window, cx| on_click(event, window, cx));
        }

        button
    }
}

pub struct Menu {
    open: bool,
    closing: bool,
    transition_epoch: u64,
    hovered_item: Option<NavItem>,
    app_id_input: gpui::Entity<InputState>,
}

impl Menu {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let app_id_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter custom app ID")
                .clean_on_escape()
        });
        Self {
            open: false,
            closing: false,
            transition_epoch: 0,
            hovered_item: None,
            app_id_input,
        }
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.open {
            self.close(cx);
        } else {
            self.transition_epoch = self.transition_epoch.wrapping_add(1);
            self.open = true;
            self.closing = false;
            cx.notify();
        }
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        if !self.open || self.closing {
            return;
        }

        self.open = false;
        self.closing = true;
        self.transition_epoch = self.transition_epoch.wrapping_add(1);
        let transition_epoch = self.transition_epoch;
        self.hovered_item = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(MENU_ANIMATION_DURATION)
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.closing && this.transition_epoch == transition_epoch {
                    this.closing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn navigate(&mut self, item: NavItem, cx: &mut Context<Self>) {
        self.close(cx);
        cx.emit(MenuEvent::Navigate(item));
    }

    fn render_nav_button(&self, item: NavItem, cx: &Context<Self>) -> NavButton {
        NavButton::new(
            item,
            self.hovered_item == Some(item),
            cx.listener(move |this, hovered: &bool, _, cx| {
                this.hovered_item = hovered.then_some(item);
                cx.notify();
            }),
        )
    }

    fn render_drawer(&self, viewport: Size<Pixels>, cx: &mut Context<Self>) -> impl IntoElement {
        let closing = self.closing;
        let home = self
            .render_nav_button(NavItem::Home, cx)
            .on_click(cx.listener(|this, _, _, cx| this.navigate(NavItem::Home, cx)));

        let view = cx.entity();
        let app_id_input = self.app_id_input.clone();
        let dialog_input = app_id_input.clone();
        let app_id = Dialog::new(cx)
            .trigger(self.render_nav_button(NavItem::AppId, cx))
            .title("Custom App ID")
            .button_props(
                DialogButtonProps::default()
                    .ok_text("Import")
                    .cancel_text("Cancel")
                    .show_cancel(true),
            )
            .on_ok(move |_, window, cx| {
                let value = app_id_input.read(cx).value();
                if let Some(app_id) = parse_app_id(&value) {
                    view.update(cx, |this, cx| {
                        this.close(cx);
                        cx.emit(MenuEvent::CustomAppId(app_id));
                    });
                    true
                } else {
                    window.push_notification("Please enter a valid App ID.", cx);
                    false
                }
            })
            .child(Input::new(&dialog_input).w_full());

        let about = self
            .render_nav_button(NavItem::About, cx)
            .on_click(cx.listener(|this, _, _, cx| this.navigate(NavItem::About, cx)));
        let options = self
            .render_nav_button(NavItem::Options, cx)
            .on_click(cx.listener(|this, _, _, cx| this.navigate(NavItem::Options, cx)));

        div()
            .child(
                div()
                    .id("menu-backdrop")
                    .absolute()
                    .w(viewport.width)
                    .h(viewport.height)
                    .top_0()
                    .left_0()
                    .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
            )
            .child(
                div()
                    .id("navigation-menu")
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(px(MENU_WIDTH))
                    .h(viewport.height)
                    .pt(px(88.0))
                    .px(px(36.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_5()
                    .overflow_y_scroll()
                    .bg(cx.theme().sidebar)
                    .shadow_2xl()
                    .occlude()
                    .children([
                        home.into_any_element(),
                        app_id.into_any_element(),
                        about.into_any_element(),
                        options.into_any_element(),
                    ])
                    .with_animation(
                        if closing {
                            "navigation-menu-slide-out"
                        } else {
                            "navigation-menu-slide-in"
                        },
                        Animation::new(MENU_ANIMATION_DURATION).with_easing(ease_in_out),
                        move |drawer, delta| {
                            let hidden_fraction = if closing { delta } else { 1.0 - delta };
                            drawer.left(px(-MENU_WIDTH * hidden_fraction))
                        },
                    ),
            )
    }

    fn render_toggle(&self, cx: &Context<Self>) -> impl IntoElement {
        Button::new("menu-toggle")
            .absolute()
            .top_3()
            .left_4()
            .ghost()
            .icon(if self.open {
                IconName::ArrowLeft
            } else {
                IconName::Menu
            })
            .toggled(self.open)
            .accessibility_id("menu-toggle")
            .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
    }
}

impl EventEmitter<MenuEvent> for Menu {}

impl Render for Menu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        div()
            .absolute()
            .top_0()
            .left_0()
            .when(self.open || self.closing, |menu| {
                menu.child(self.render_drawer(viewport, cx))
            })
            .child(self.render_toggle(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_keeps_the_legacy_order_and_labels() {
        assert_eq!(
            NavItem::ALL.map(NavItem::label),
            ["Home", "App ID", "About", "Options"]
        );
    }

    #[test]
    fn home_is_the_only_non_spinning_navigation_icon() {
        assert!(!NavItem::Home.spins());
        assert!(NavItem::AppId.spins());
        assert!(NavItem::About.spins());
        assert!(NavItem::Options.spins());
    }

    #[test]
    fn custom_app_id_must_be_a_non_negative_integer() {
        assert_eq!(parse_app_id(" 108600 "), Some(108_600));
        assert_eq!(parse_app_id("not an id"), None);
        assert_eq!(parse_app_id("-1"), None);
        assert_eq!(parse_app_id("12games"), None);
    }
}
