use gpui::{Context, IntoElement, Render, Window, div, prelude::*};
use gpui_component::{Sizable as _, progress::Progress, v_flex};

pub struct ImportProgress {
    value: f32,
}

impl ImportProgress {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }

    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        let value = value.clamp(0.0, 100.0);
        if value > self.value {
            self.value = value;
            cx.notify();
        }
    }
}

impl Render for ImportProgress {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(div().text_center().child(format!("{:.0}%", self.value)))
            .child(
                Progress::new("screenshot-import-progress")
                    .value(self.value)
                    .large(),
            )
    }
}
