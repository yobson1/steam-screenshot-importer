#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app_dirs;
mod components;
mod fallback_artwork;
mod image_fetch;
mod offscreen;
mod preferences;
mod steam_locate;

use std::{
    cell::Cell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use components::game_tile::{
    GameTileMotion, GameTileProps, Pointer, game_tile, offscreen_vertices,
    projected_vertices_changed,
};
use components::menu::{Menu, MenuEvent, NavItem};
use components::theme_toggle::theme_toggle;
use gpui::{
    App, Bounds, Context, Pixels, Render, RenderImage, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme as _, Root, Theme, ThemeRegistry,
    scroll::{ScrollableElement as _, ScrollbarMode},
    spinner::Spinner,
};
use image::{Frame, RgbaImage, imageops::FilterType};
use log::{error, info};
use offscreen::{OffscreenRenderer, ProjectedVertex, RenderTag};
use rayon::prelude::*;
use steam_locate::GameArtwork;

const ARTWORK_WIDTH: u32 = 300;
const ARTWORK_HEIGHT: u32 = 450;

struct LoadedGame {
    app_id: u32,
    app_name: String,
    pixels: RgbaImage,
    missing_artwork: bool,
}

struct LoadedLibrary {
    games: Vec<LoadedGame>,
    steam_user: Option<String>,
}

struct CardState {
    app_id: u32,
    artwork_handle: usize,
    artwork: Arc<RenderImage>,
    artwork_is_projected: bool,
    desired_vertices: [ProjectedVertex; 4],
    desired_generation: u64,
    applied_generation: u64,
    in_flight_generation: Option<u64>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    motion: GameTileMotion,
}

enum LibraryState {
    Loading,
    Ready,
    Failed(String),
}

struct SteamScreenshotImporter {
    cards: Vec<CardState>,
    offscreen: OffscreenRenderer,
    retired_images: Vec<Arc<RenderImage>>,
    steam_user: Option<String>,
    library_state: LibraryState,
    menu: gpui::Entity<Menu>,
    last_frame: Instant,
    _appearance_subscription: gpui::Subscription,
    _menu_subscription: gpui::Subscription,
}

impl SteamScreenshotImporter {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let load_task = cx.background_spawn(async { load_steam_library() });
        cx.spawn(async move |this, cx| {
            let result = load_task.await;
            if let Err(update_error) = this.update(cx, |this, cx| {
                match result {
                    Ok(library) => this.install_library(library),
                    Err(load_error) => {
                        error!("Failed to load Steam library: {load_error}");
                        this.library_state = LibraryState::Failed(load_error);
                    }
                }
                cx.notify();
            }) {
                error!("Failed to update Steam library view: {update_error}");
            }
        })
        .detach();

        let now = Instant::now();
        let menu = cx.new(|cx| Menu::new(window, cx));
        let menu_subscription = cx.subscribe(&menu, |_, _, event, _| match event {
            MenuEvent::Navigate(NavItem::Home) => {}
            MenuEvent::Navigate(NavItem::About) => info!("About navigation selected"),
            MenuEvent::Navigate(NavItem::Options) => info!("Options navigation selected"),
            MenuEvent::Navigate(NavItem::AppId) => unreachable!("App ID uses a dialog"),
            MenuEvent::CustomAppId(app_id) => info!("Selected custom Steam app {app_id}"),
        });
        let appearance_subscription = cx.observe_window_appearance(window, |_, window, cx| {
            if preferences::selected_theme(cx).is_none() {
                Theme::sync_system_appearance(Some(window), cx);
                Theme::set_scrollbar_mode(ScrollbarMode::Always, cx);
                cx.notify();
            }
        });
        Self {
            cards: Vec::new(),
            offscreen: OffscreenRenderer::new()
                .expect("failed to initialize offscreen WGPU card renderer"),
            retired_images: Vec::new(),
            steam_user: None,
            library_state: LibraryState::Loading,
            menu,
            last_frame: now,
            _appearance_subscription: appearance_subscription,
            _menu_subscription: menu_subscription,
        }
    }

    fn install_library(&mut self, library: LoadedLibrary) {
        self.steam_user = library.steam_user;
        self.cards = library
            .games
            .into_iter()
            .map(|game| {
                let artwork_handle = self.offscreen.upload_artwork(&game.pixels);
                CardState {
                    app_id: game.app_id,
                    artwork_handle,
                    artwork: render_image_from_bgra(game.pixels),
                    artwork_is_projected: false,
                    desired_vertices: offscreen_vertices(Pointer::default(), 0.0),
                    desired_generation: 0,
                    applied_generation: 0,
                    in_flight_generation: None,
                    bounds: Rc::new(Cell::new(Bounds::default())),
                    motion: GameTileMotion::default(),
                }
            })
            .collect();
        self.library_state = LibraryState::Ready;
        info!("Loaded {} games from Steam", self.cards.len());
    }

    fn tick_cards(&mut self, window: &mut Window) {
        for image in self.retired_images.drain(..) {
            let _ = window.drop_image(image);
        }

        match self.offscreen.poll_completed() {
            Ok(completed_renders) => {
                for completed in completed_renders {
                    let Some(card) = self.cards.get_mut(completed.tag.card_index) else {
                        continue;
                    };
                    if card.in_flight_generation == Some(completed.tag.generation) {
                        card.in_flight_generation = None;
                    }
                    if completed.tag.generation >= card.applied_generation {
                        card.applied_generation = completed.tag.generation;
                        card.artwork_is_projected = true;
                        let previous = std::mem::replace(
                            &mut card.artwork,
                            render_image_from_bgra(completed.pixels),
                        );
                        self.retired_images.push(previous);
                    }
                }
            }
            Err(render_error) => error!("Failed to poll projected card renders: {render_error:#}"),
        }

        let now = Instant::now();
        let dt = now
            .saturating_duration_since(self.last_frame)
            .min(Duration::from_millis(34));
        self.last_frame = now;

        let mut animating = false;
        for card in &mut self.cards {
            animating |= card.motion.tick(dt);
            let vertices = offscreen_vertices(card.motion.pointer, card.motion.hover);
            if projected_vertices_changed(card.desired_vertices, vertices) {
                card.desired_vertices = vertices;
                card.desired_generation += 1;
            }
        }

        for (card_index, card) in self.cards.iter_mut().enumerate() {
            if card.in_flight_generation.is_some()
                || card.applied_generation >= card.desired_generation
            {
                continue;
            }

            let tag = RenderTag {
                card_index,
                generation: card.desired_generation,
            };
            if self
                .offscreen
                .submit(card.artwork_handle, card.desired_vertices, tag)
            {
                card.in_flight_generation = Some(tag.generation);
            }
        }

        if animating || self.offscreen.has_pending_work() {
            window.request_animation_frame();
        }
    }

    fn render_card(&self, index: usize, cx: &Context<Self>) -> impl IntoElement {
        let card = &self.cards[index];
        let props = GameTileProps {
            index,
            artwork: card.artwork.clone(),
            artwork_is_projected: card.artwork_is_projected,
            bounds: card.bounds.clone(),
            pointer: card.motion.pointer,
            hover: card.motion.hover,
            glare: card.motion.glare,
        };
        let app_id = card.app_id;

        game_tile(
            props,
            cx.listener(move |this, hovering, _, cx| {
                this.cards[index].motion.set_hovered(*hovering);
                cx.notify();
            }),
            cx.listener(move |this, pointer, _, cx| {
                this.cards[index].motion.set_pointer(*pointer);
                cx.notify();
            }),
            cx.listener(move |_, _, _, _| {
                info!("Selected Steam app {app_id}");
            }),
        )
    }

    fn welcome_text(&self) -> String {
        self.steam_user.as_ref().map_or_else(
            || "WELCOME USER!".to_owned(),
            |user| format!("WELCOME {}!", user.to_uppercase()),
        )
    }
}

impl Render for SteamScreenshotImporter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.tick_cards(window);

        let cards = (0..self.cards.len())
            .map(|index| self.render_card(index, cx).into_any_element())
            .collect::<Vec<_>>();
        let is_loading = matches!(self.library_state, LibraryState::Loading);
        let library_message = match &self.library_state {
            LibraryState::Ready if self.cards.is_empty() => {
                Some("No installed Steam games were found.".to_owned())
            }
            LibraryState::Failed(message) => Some(format!("Error: {message}")),
            LibraryState::Loading | LibraryState::Ready => None,
        };
        let background = cx.theme().background;
        let foreground = cx.theme().foreground;
        let primary = cx.theme().primary;
        let muted_foreground = cx.theme().muted_foreground;

        let page = div()
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
                            .child(self.welcome_text()),
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
                    .when(is_loading, |gallery| {
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
                    .when_some(library_message, |gallery, message| {
                        gallery.child(
                            div()
                                .w_full()
                                .text_center()
                                .text_sm()
                                .text_color(muted_foreground)
                                .child(message),
                        )
                    })
                    .children(cards),
            )
            .overflow_y_scrollbar();

        let content = div()
            .relative()
            .size_full()
            .bg(background)
            .child(page)
            .child(self.menu.clone())
            .child(theme_toggle(cx));

        #[cfg(feature = "fps")]
        let content =
            content.child(gpui_fps::fps_monitor(window, cx).anchor(gpui::Anchor::BottomRight));

        content
    }
}

fn load_steam_library() -> Result<LoadedLibrary, String> {
    let steam_user = steam_locate::get_recent_steam_user()
        .ok()
        .filter(|user| !user.is_empty());
    let games = steam_locate::get_games()?;
    let mut games = games
        .into_par_iter()
        .map(|game| {
            let artwork_bytes = match game.artwork {
                GameArtwork::Bytes(bytes) => Some(bytes),
                GameArtwork::Url(url) => image_fetch::download_image(&url),
                GameArtwork::Missing => None,
            };
            let (pixels, missing_artwork) = artwork_bytes
                .as_deref()
                .and_then(decode_artwork)
                .map_or_else(|| (placeholder_artwork(), true), |pixels| (pixels, false));

            LoadedGame {
                app_id: game.app_id,
                app_name: game.app_name,
                pixels,
                missing_artwork,
            }
        })
        .collect::<Vec<_>>();

    if games.iter().any(|game| game.missing_artwork) {
        let mut title_renderer = fallback_artwork::TitleRenderer::new();
        for game in &mut games {
            if game.missing_artwork {
                title_renderer.composite_title(&mut game.pixels, &game.app_name);
            }
        }
    }

    Ok(LoadedLibrary { games, steam_user })
}

fn decode_artwork(bytes: &[u8]) -> Option<RgbaImage> {
    let decoded = image::load_from_memory(bytes).ok()?.into_rgba8();
    let mut pixels = image::imageops::resize(
        &decoded,
        ARTWORK_WIDTH,
        ARTWORK_HEIGHT,
        FilterType::Lanczos3,
    );
    rgba_to_bgra(&mut pixels);
    Some(pixels)
}

fn placeholder_artwork() -> RgbaImage {
    decode_artwork(include_bytes!("../assets/defaultappimage.png"))
        .expect("bundled default game artwork should decode")
}

fn rgba_to_bgra(pixels: &mut RgbaImage) {
    for pixel in pixels.pixels_mut() {
        pixel.0.swap(0, 2);
    }
}

fn render_image_from_bgra(pixels: RgbaImage) -> Arc<RenderImage> {
    Arc::new(RenderImage::new(vec![Frame::new(pixels)]))
}

fn configure_themes(cx: &mut App) {
    ThemeRegistry::global_mut(cx)
        .load_themes_from_str(include_str!("../assets/themes/macos-classic.json"))
        .expect("bundled macOS Classic themes should parse");

    let registry = ThemeRegistry::global(cx);
    let light = registry
        .themes()
        .get("macOS Classic Light")
        .cloned()
        .expect("bundled macOS Classic Light theme should be registered");
    let dark = registry
        .themes()
        .get("macOS Classic Dark")
        .cloned()
        .expect("bundled macOS Classic Dark theme should be registered");

    let theme = Theme::global_mut(cx);
    theme.light_theme = light;
    theme.dark_theme = dark;
    if let Some(mode) = preferences::selected_theme(cx) {
        Theme::change(mode, None, cx);
    } else {
        Theme::sync_system_appearance(None, cx);
    }
    Theme::set_scrollbar_mode(ScrollbarMode::Always, cx);
}

fn main() {
    let _ = simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init();

    gpui_platform::application()
        .with_assets(gpui_component_assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);
            preferences::init(cx);
            configure_themes(cx);
            let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Steam Screenshot Importer".into()),
                        ..Default::default()
                    }),
                    focus: true,
                    ..Default::default()
                },
                |window, cx| {
                    if let Some(mode) = preferences::selected_theme(cx) {
                        Theme::change(mode, Some(window), cx);
                    } else {
                        Theme::sync_system_appearance(Some(window), cx);
                    }
                    Theme::set_scrollbar_mode(ScrollbarMode::Always, cx);
                    let view = cx.new(|cx| SteamScreenshotImporter::new(window, cx));
                    cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
                },
            )
            .expect("failed to open Steam Screenshot Importer window");
            cx.activate(true);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_has_expected_card_dimensions() {
        let placeholder = placeholder_artwork();
        assert_eq!(placeholder.dimensions(), (ARTWORK_WIDTH, ARTWORK_HEIGHT));
    }

    #[test]
    fn rgba_channels_are_converted_for_gpui_and_wgpu() {
        let mut pixels = RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 4]));
        rgba_to_bgra(&mut pixels);
        assert_eq!(pixels.get_pixel(0, 0).0, [3, 2, 1, 4]);
    }

    #[test]
    fn fixture_artwork_uses_the_production_decode_path() {
        let fixture = include_bytes!("../screenshots/fixtures/108600.jpg");
        let pixels = decode_artwork(fixture).expect("fixture should decode");
        assert_eq!(pixels.dimensions(), (ARTWORK_WIDTH, ARTWORK_HEIGHT));
    }
}
