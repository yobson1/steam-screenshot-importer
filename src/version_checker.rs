use anyhow::Context as _;
use gpui::{App, InteractiveElement as _, ParentElement as _, Styled as _, Window, div, px};
use gpui_component::{
    WindowExt as _, dialog::DialogButtonProps, notification::NotificationType,
    scroll::ScrollableElement as _, text::TextView,
};
use log::error;
use semver::Version;
use serde::Deserialize;
use std::time::Duration;

const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/yobson1/steam-screenshot-importer/releases/latest";

#[derive(Debug)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub url: String,
}

#[derive(Debug)]
pub enum UpdateStatus {
    Current,
    Available(Release),
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
}

pub fn check() -> anyhow::Result<UpdateStatus> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .context("application version is not valid semantic versioning")?;
    let response = reqwest::blocking::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .timeout(Duration::from_secs(15))
        .build()
        .context("failed to create update client")?
        .get(LATEST_RELEASE_URL)
        .send()
        .context("failed to fetch latest release")?
        .error_for_status()
        .context("GitHub returned an error while checking for updates")?
        .json::<GitHubRelease>()
        .context("latest release response was invalid")?;

    let version_text = response.tag_name.trim_start_matches('v');
    let latest = Version::parse(version_text).context("latest release version is invalid")?;
    if latest <= current {
        return Ok(UpdateStatus::Current);
    }

    Ok(UpdateStatus::Available(Release {
        version: version_text.to_owned(),
        notes: response.body.unwrap_or_default(),
        url: response.html_url,
    }))
}

pub fn present(
    result: anyhow::Result<UpdateStatus>,
    manual: bool,
    window: &mut Window,
    cx: &mut App,
) {
    match result {
        Ok(UpdateStatus::Current) if manual => window.push_notification(
            (NotificationType::Success, "You're on the latest version."),
            cx,
        ),
        Ok(UpdateStatus::Current) => {}
        Ok(UpdateStatus::Available(release)) => show_update(release, window, cx),
        Err(check_error) => {
            error!("Failed to check for updates: {check_error:#}");
            if manual {
                window.push_notification(
                    (
                        NotificationType::Error,
                        "Could not check for updates. Please try again later.",
                    ),
                    cx,
                );
            }
        }
    }
}

fn show_update(release: Release, window: &mut Window, cx: &mut App) {
    let title = format!("Update available — v{}", release.version);
    let notes = if release.notes.trim().is_empty() {
        "No release notes were provided.".to_owned()
    } else {
        release.notes
    };
    let url = release.url;

    window.open_dialog(cx, move |dialog, _, _| {
        let url = url.clone();
        dialog
            .title(title.clone())
            .width(px(640.0))
            .button_props(
                DialogButtonProps::default()
                    .ok_text("Open release")
                    .cancel_text("Dismiss")
                    .show_cancel(true),
            )
            .on_ok(move |_, _, cx| {
                cx.open_url(&url);
                true
            })
            .child(
                div()
                    .id("release-notes-scroll")
                    .max_h(px(420.0))
                    .pr_2()
                    .child(TextView::markdown("release-notes", notes.clone()).selectable(true))
                    .overflow_y_scrollbar(),
            )
    });
}
