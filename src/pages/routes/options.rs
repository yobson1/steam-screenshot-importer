use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, IndexPath, InteractiveElementExt as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    group_box::{GroupBox, GroupBoxVariants as _},
    scroll::ScrollableElement as _,
    select::{Select, SelectEvent, SelectItem, SelectState},
    slider::{Slider, SliderEvent, SliderState},
};

use crate::{
    preferences::{Preferences, ResizeFilter},
    version_checker,
};

#[derive(Clone)]
struct FilterOption(ResizeFilter);

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn slider_quality(value: f32) -> u8 {
    value.round().clamp(1.0, 100.0) as u8
}

impl SelectItem for FilterOption {
    type Value = ResizeFilter;

    fn title(&self) -> SharedString {
        self.0.label().into()
    }

    fn value(&self) -> &Self::Value {
        &self.0
    }
}

pub struct OptionsPage {
    jpeg_quality: u8,
    check_updates_on_startup: bool,
    checking_for_updates: bool,
    quality_slider: Entity<SliderState>,
    filter_select: Entity<SelectState<Vec<FilterOption>>>,
    scroll_handle: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl OptionsPage {
    pub fn new(window: &mut Window, scroll_handle: ScrollHandle, cx: &mut Context<Self>) -> Self {
        let (jpeg_quality, resize_filter, check_updates_on_startup) = {
            let preferences = cx.global::<Preferences>();
            (
                preferences.jpeg_quality.get(),
                preferences.resize_filter.get(),
                preferences.check_updates_on_startup.get(),
            )
        };
        let quality_slider = cx.new(|_| {
            SliderState::new()
                .min(1.0)
                .max(100.0)
                .step(1.0)
                .default_value(f32::from(jpeg_quality))
        });

        let filters = ResizeFilter::ALL
            .into_iter()
            .map(FilterOption)
            .collect::<Vec<_>>();
        let selected_filter = ResizeFilter::ALL
            .iter()
            .position(|filter| *filter == resize_filter)
            .map(|index| IndexPath::default().row(index));
        let filter_select = cx.new(|cx| SelectState::new(filters, selected_filter, window, cx));

        let subscriptions = vec![
            cx.subscribe(&quality_slider, |this, _, event: &SliderEvent, cx| {
                let value = match event {
                    SliderEvent::Change(value) | SliderEvent::Release(value) => value.start(),
                };
                this.jpeg_quality = slider_quality(value);
                if matches!(event, SliderEvent::Release(_)) {
                    cx.global::<Preferences>()
                        .jpeg_quality
                        .set(this.jpeg_quality);
                }
                cx.notify();
            }),
            cx.subscribe(
                &filter_select,
                |_, _, event: &SelectEvent<Vec<FilterOption>>, cx| {
                    let SelectEvent::Confirm(Some(filter)) = event else {
                        return;
                    };
                    cx.global::<Preferences>().resize_filter.set(*filter);
                },
            ),
        ];

        Self {
            jpeg_quality,
            check_updates_on_startup,
            checking_for_updates: false,
            quality_slider,
            filter_select,
            scroll_handle,
            _subscriptions: subscriptions,
        }
    }

    fn check_for_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.checking_for_updates {
            return;
        }

        self.checking_for_updates = true;
        cx.notify();
        let check = cx.background_spawn(async { version_checker::check() });
        cx.spawn_in(window, async move |this, cx| {
            let result = check.await;
            if let Err(update_error) = this.update_in(cx, |this, window, cx| {
                this.checking_for_updates = false;
                version_checker::present(result, true, window, cx);
                cx.notify();
            }) {
                log::error!("Failed to present manual update check: {update_error}");
            }
        })
        .detach();
    }

    fn hint(text: &'static str, cx: &App) -> impl IntoElement {
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(text)
    }

    fn scrollable_page(
        content: impl IntoElement,
        scroll_handle: &ScrollHandle,
        cx: &App,
    ) -> impl IntoElement {
        div()
            .id("options-page")
            .size_full()
            .relative()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .id("options-page-scroll-area")
                    .size_full()
                    .track_scroll(scroll_handle)
                    .overflow_y_scroll()
                    .lock_scroll_axis()
                    .child(content),
            )
            .vertical_scrollbar(scroll_handle)
    }
}

impl Render for OptionsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let quality = self.jpeg_quality;
        let checking = self.checking_for_updates;

        let content = div()
            .w_full()
            .min_h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .px_4()
            .pt_6()
            .pb_8()
            .gap_4()
            .child(
                div()
                    .flex_none()
                    .text_size(px(40.0))
                    .font_weight(gpui::FontWeight::THIN)
                    .text_color(cx.theme().primary)
                    .child("OPTIONS"),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(480.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        GroupBox::new()
                            .id("image-processing-options")
                            .outline()
                            .title(div().font_semibold().child("Image Processing"))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .font_semibold()
                                    .child("JPEG quality")
                                    .child(
                                        div()
                                            .rounded_md()
                                            .bg(cx.theme().primary)
                                            .text_color(cx.theme().primary_foreground)
                                            .px_2()
                                            .py_0p5()
                                            .text_sm()
                                            .font_bold()
                                            .child(quality.to_string()),
                                    ),
                            )
                            .child(Slider::new(&self.quality_slider).w_full())
                            .child(Self::hint(
                                "Used when converting non-JPEG images and when generating the Steam thumbnail. Higher is better quality but a larger file size.",
                                cx,
                            ))
                            .child(div().font_semibold().child("Downscale filter"))
                            .child(Select::new(&self.filter_select).w_full())
                            .child(Self::hint(
                                "Algorithm used when an image needs to be resized to fit within Steam's limits, and when generating the thumbnail.",
                                cx,
                            )),
                    )
                    .child(
                        GroupBox::new()
                            .id("update-options")
                            .outline()
                            .title(div().font_semibold().child("Updates"))
                            .child(
                                Checkbox::new("check-updates-on-startup")
                                    .checked(self.check_updates_on_startup)
                                    .label("Check for updates on startup")
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        this.check_updates_on_startup = *checked;
                                        cx.global::<Preferences>()
                                            .check_updates_on_startup
                                            .set(*checked);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("check-for-updates")
                                    .primary()
                                    .w_full()
                                    .loading(checking)
                                    .label(if checking {
                                        "Checking…"
                                    } else {
                                        "Check for updates now"
                                    })
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.check_for_updates(window, cx);
                                    })),
                            ),
                    ),
            );

        Self::scrollable_page(content, &self.scroll_handle, cx)
    }
}
