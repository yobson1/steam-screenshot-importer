use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, AppContext as _, BoxShadow, ClickEvent, Context,
    EventEmitter, Focusable as _, InteractiveElement as _, IntoElement, ParentElement as _, Pixels,
    Render, RenderOnce, Size, StatefulInteractiveElement as _, Styled as _, Transformation, Window,
    div, ease_in_out, hsla, percentage, point, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, StyledExt as _, WindowExt as _,
    button::{Button, ButtonRounded, ButtonVariants as _},
    dialog::{Cancel, DialogAction, DialogClose, DialogFooter, DialogTitle},
    input::{Input, InputState},
};

pub const MENU_WIDTH: f32 = 136.0;
const MENU_ANIMATION_DURATION: Duration = Duration::from_millis(525);
const NAV_SPIN_DURATION: Duration = Duration::from_millis(750);

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

    const fn index(self) -> usize {
        match self {
            Self::Home => 0,
            Self::AppId => 1,
            Self::About => 2,
            Self::Options => 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SpinTransition {
    value: f32,
    from: f32,
    target: f32,
    started_at: Option<Instant>,
}

impl Default for SpinTransition {
    fn default() -> Self {
        Self {
            value: 0.0,
            from: 0.0,
            target: 0.0,
            started_at: None,
        }
    }
}

impl SpinTransition {
    fn set_hovered(&mut self, hovered: bool, now: Instant) {
        self.tick(now);
        let target = f32::from(hovered);
        if (self.target - target).abs() < f32::EPSILON {
            return;
        }
        self.from = self.value;
        self.target = target;
        self.started_at = Some(now);
    }

    fn tick(&mut self, now: Instant) -> bool {
        let Some(started_at) = self.started_at else {
            return false;
        };
        let elapsed = now.saturating_duration_since(started_at);
        let delta = (elapsed.as_secs_f32() / NAV_SPIN_DURATION.as_secs_f32()).min(1.0);
        self.value = self.from + (self.target - self.from) * ease_in_out(delta);
        if delta >= 1.0 {
            self.value = self.target;
            self.started_at = None;
            false
        } else {
            true
        }
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type HoverHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

#[derive(IntoElement)]
struct NavButton {
    item: NavItem,
    rotation: f32,
    on_click: Option<ClickHandler>,
    on_hover: HoverHandler,
}

impl NavButton {
    fn new(
        item: NavItem,
        rotation: f32,
        on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            item,
            rotation,
            on_click: None,
            on_hover: Rc::new(on_hover),
        }
    }

    fn on_click(mut self, on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    fn render_icon(&self) -> AnyElement {
        let icon = Icon::new(self.item.icon()).size(px(25.0));
        if self.item.spins() && self.rotation > 0.0 {
            icon.transform(Transformation::rotate(percentage(self.rotation)))
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
    spin_transitions: [SpinTransition; 4],
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
            spin_transitions: [SpinTransition::default(); 4],
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
        for transition in &mut self.spin_transitions {
            transition.reset();
        }
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
        cx.emit(MenuEvent::Navigate(item));
    }

    fn open_app_id_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close(cx);
        self.app_id_input
            .update(cx, |input, cx| input.set_value("", window, cx));

        let view = cx.entity();
        let app_id_input = self.app_id_input.clone();
        let dialog_input = app_id_input.clone();
        window.open_dialog(cx, move |dialog, _, _| {
            let app_id_input = app_id_input.clone();
            let view = view.clone();
            dialog
                .width(px(448.0))
                .overlay_closable(false)
                .on_ok(move |_, window, cx| {
                    let value = app_id_input.read(cx).value();
                    if let Some(app_id) = parse_app_id(&value) {
                        view.update(cx, |_, cx| cx.emit(MenuEvent::CustomAppId(app_id)));
                        true
                    } else {
                        window.push_notification("Please enter a valid App ID.", cx);
                        false
                    }
                })
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .on_mouse_down_out(|_, window, cx| {
                            window.dispatch_action(Box::new(Cancel), cx);
                        })
                        .child(DialogTitle::new().child("Custom App ID"))
                        .child(Input::new(&dialog_input).w_full())
                        .child(
                            DialogFooter::new()
                                .gap_2()
                                .child(
                                    DialogClose::new().child(
                                        Button::new("cancel-custom-app-id")
                                            .label("Cancel")
                                            .outline(),
                                    ),
                                )
                                .child(
                                    DialogAction::new().child(
                                        Button::new("import-custom-app-id")
                                            .label("Import")
                                            .primary(),
                                    ),
                                ),
                        ),
                )
        });
        self.app_id_input
            .read(cx)
            .focus_handle(cx)
            .focus(window, cx);
    }

    fn render_nav_button(&self, item: NavItem, cx: &Context<Self>) -> NavButton {
        NavButton::new(
            item,
            self.spin_transitions[item.index()].value,
            cx.listener(move |this, hovered: &bool, _, cx| {
                this.set_item_hovered(item, *hovered);
                cx.notify();
            }),
        )
    }

    fn set_item_hovered(&mut self, item: NavItem, hovered: bool) {
        if item.spins() {
            self.spin_transitions[item.index()].set_hovered(hovered, Instant::now());
        }
    }

    fn tick_spin_transitions(&mut self, now: Instant) -> bool {
        let mut animating = false;
        for transition in &mut self.spin_transitions {
            animating |= transition.tick(now);
        }
        animating
    }

    fn render_drawer(&self, viewport: Size<Pixels>, cx: &mut Context<Self>) -> impl IntoElement {
        let closing = self.closing;
        let home = self
            .render_nav_button(NavItem::Home, cx)
            .on_click(cx.listener(|this, _, _, cx| this.navigate(NavItem::Home, cx)));

        let app_id = self
            .render_nav_button(NavItem::AppId, cx)
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_app_id_dialog(window, cx);
            }));

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
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .shadow(vec![BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.4),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(4.0),
                        spread_radius: px(5.0),
                        inset: false,
                    }])
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
        let icon = if self.open {
            IconName::ArrowLeft
        } else {
            IconName::Menu
        };

        Button::new("menu-toggle")
            .absolute()
            .top_3()
            .left_4()
            .w(px(48.0))
            .h(px(48.0))
            .p_0()
            .ghost()
            .child(Icon::new(icon).size(px(30.0)))
            .toggled(self.open)
            .accessibility_id("menu-toggle")
            .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
    }
}

impl EventEmitter<MenuEvent> for Menu {}

impl Render for Menu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.tick_spin_transitions(Instant::now()) {
            window.request_animation_frame();
        }
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

    #[test]
    fn interrupted_spin_reverses_from_its_current_rotation() {
        let started_at = Instant::now();
        let mut transition = SpinTransition::default();
        transition.set_hovered(true, started_at);
        transition.tick(started_at + NAV_SPIN_DURATION / 2);
        let interrupted_rotation = transition.value;

        transition.set_hovered(false, started_at + NAV_SPIN_DURATION / 2);
        assert!((transition.value - interrupted_rotation).abs() < f32::EPSILON);

        transition.tick(started_at + NAV_SPIN_DURATION);
        assert!(transition.value < interrupted_rotation);
    }
}
