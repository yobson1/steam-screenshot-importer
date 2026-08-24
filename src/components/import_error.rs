use std::sync::Arc;

use gpui::{App, InteractiveElement as _, ParentElement as _, Styled as _, Window, div, px};
use gpui_component::{
    ActiveTheme as _, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{Cancel, DialogClose, DialogFooter, DialogTitle},
    notification::NotificationType,
    scroll::ScrollableElement as _,
    v_flex,
};

use crate::image_import::{ImportError, ImportFailure};

pub fn present(import_error: ImportError, window: &mut Window, cx: &mut App) {
    if import_error.errors.is_empty() {
        window.push_notification((NotificationType::Error, import_error.summary), cx);
        return;
    }

    let title = format!("Import errors ({})", import_error.errors.len());
    let summary = import_error.summary;
    let errors = Arc::new(import_error.errors);

    window.open_dialog(cx, move |dialog, _, cx| {
        let error_items = errors
            .iter()
            .enumerate()
            .map(|(index, failure)| error_item(index, failure, cx));

        dialog.width(px(720.0)).overlay_closable(false).child(
            v_flex()
                .h(px(520.0))
                .gap_4()
                .on_mouse_down_out(|_, window, cx| {
                    window.dispatch_action(Box::new(Cancel), cx);
                })
                .child(DialogTitle::new().child(title.clone()))
                .child(div().text_sm().child(summary.clone()))
                .child(
                    div()
                        .id("import-error-list")
                        .flex_1()
                        .min_h_0()
                        .pr_2()
                        .overflow_y_scrollbar()
                        .child(v_flex().gap_3().children(error_items)),
                )
                .child(
                    DialogFooter::new().child(
                        DialogClose::new()
                            .child(Button::new("close-import-errors").label("Close").primary()),
                    ),
                ),
        )
    });
}

fn error_item(index: usize, failure: &ImportFailure, cx: &App) -> impl gpui::IntoElement {
    let file_name = failure.file_path.file_name().map_or_else(
        || failure.file_path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );

    v_flex()
        .id(("import-error", index))
        .gap_1()
        .p_3()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
        .child(div().font_semibold().child(file_name))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(failure.file_path.display().to_string()),
        )
        .child(div().text_sm().child(failure.message.clone()))
}
