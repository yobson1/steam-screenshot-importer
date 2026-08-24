#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app_dirs;
mod assets;
mod components;
mod fallback_artwork;
mod file_picker;
mod image_fetch;
mod image_import;
mod offscreen;
mod pages;
mod preferences;
mod steam;
mod steam_locate;
mod version_checker;

use std::{
    cell::Cell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use components::footer::footer;
use components::game_tile::{
    GameTileMotion, GameTileProps, Pointer, game_tile, offscreen_vertices,
    projected_vertices_changed,
};
use components::import_progress::ImportProgress;
use components::menu::{Menu, MenuEvent, NavItem};
use components::theme_toggle::theme_toggle;
use gpui::{
    App, Bounds, Context, Pixels, Render, RenderImage, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme as _, Root, Theme, ThemeRegistry, WindowExt as _, notification::NotificationType,
    scroll::ScrollbarMode,
};
use image::{Frame, RgbaImage, imageops::FilterType};
use log::{error, info};
use offscreen::{OffscreenRenderer, ProjectedVertex, RenderTag};
use pages::{
    Route, Router,
    routes::{AboutPage, GameSearch, GameSearchEvent, HomePage, OptionsPage},
};
use preferences::Preferences;
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
    app_name: String,
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
    window_handle: gpui::AnyWindowHandle,
    cards: Vec<CardState>,
    offscreen: OffscreenRenderer,
    retired_images: Vec<Arc<RenderImage>>,
    steam_user: Option<String>,
    library_state: LibraryState,
    importing: bool,
    game_search: gpui::Entity<GameSearch>,
    router: Router,
    menu: gpui::Entity<Menu>,
    options_page: gpui::Entity<OptionsPage>,
    last_frame: Instant,
    _appearance_subscription: gpui::Subscription,
    _menu_subscription: gpui::Subscription,
    _search_subscription: gpui::Subscription,
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
        let options_page = cx.new(|cx| OptionsPage::new(window, cx));
        let game_search = cx.new(|cx| GameSearch::new(window, cx));
        let search_subscription =
            cx.subscribe(&game_search, |_, _, _: &GameSearchEvent, cx| cx.notify());
        let menu_subscription = cx.subscribe(&menu, |this, _, event, cx| match event {
            MenuEvent::Navigate(NavItem::Home) => {
                if this.router.navigate(Route::Home) {
                    cx.notify();
                }
            }
            MenuEvent::Navigate(NavItem::About) => {
                if this.router.navigate(Route::About) {
                    cx.notify();
                }
            }
            MenuEvent::Navigate(NavItem::Options) => {
                if this.router.navigate(Route::Options) {
                    cx.notify();
                }
            }
            MenuEvent::Navigate(NavItem::AppId) => unreachable!("App ID uses a dialog"),
            MenuEvent::CustomAppId(app_id) => this.start_import(*app_id, cx),
        });
        let appearance_subscription = cx.observe_window_appearance(window, |_, window, cx| {
            if cx.global::<Preferences>().theme.get().mode().is_none() {
                Theme::sync_system_appearance(Some(window), cx);
                Theme::set_scrollbar_mode(ScrollbarMode::Always, cx);
                cx.notify();
            }
        });

        if cx.global::<Preferences>().check_updates_on_startup.get() {
            let check = cx.background_spawn(async { version_checker::check() });
            cx.spawn_in(window, async move |this, cx| {
                let result = check.await;
                if let Err(update_error) = this.update_in(cx, |_, window, cx| {
                    version_checker::present(result, false, window, cx);
                }) {
                    error!("Failed to present startup update check: {update_error}");
                }
            })
            .detach();
        }

        Self {
            window_handle: window.window_handle(),
            cards: Vec::new(),
            offscreen: OffscreenRenderer::new()
                .expect("failed to initialize offscreen WGPU card renderer"),
            retired_images: Vec::new(),
            steam_user: None,
            library_state: LibraryState::Loading,
            importing: false,
            game_search,
            router: Router::default(),
            menu,
            options_page,
            last_frame: now,
            _appearance_subscription: appearance_subscription,
            _menu_subscription: menu_subscription,
            _search_subscription: search_subscription,
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
                    app_name: game.app_name,
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

    fn start_import(&mut self, app_id: u32, cx: &mut Context<Self>) {
        let window_handle = self.window_handle;
        if self.importing {
            let _ = window_handle.update(cx, |_, window, cx| {
                window.push_notification("A screenshot import is already in progress.", cx);
            });
            return;
        }

        self.importing = true;
        let (jpeg_quality, resize_filter) = {
            let preferences = cx.global::<Preferences>();
            (
                preferences.jpeg_quality.get(),
                preferences.resize_filter.get(),
            )
        };
        cx.spawn(async move |this, cx| {
            let file_paths = file_picker::pick_screenshot_files().await;
            if file_paths.is_empty() {
                if let Err(update_error) = this.update(cx, |this, cx| {
                    this.importing = false;
                    cx.notify();
                }) {
                    error!("Failed to reset import state: {update_error}");
                }
                return;
            }

            let progress = cx.new(|_| ImportProgress::new());
            let dialog_progress = progress.clone();
            if let Err(dialog_error) = window_handle.update(cx, move |_, window, cx| {
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .title("Importing Screenshots")
                        .close_button(false)
                        .keyboard(false)
                        .overlay_closable(false)
                        .on_ok(|_, _, _| false)
                        .on_cancel(|_, _, _| false)
                        .child(dialog_progress.clone())
                });
            }) {
                error!("Failed to show screenshot import progress: {dialog_error}");
            }

            let (progress_sender, progress_receiver) = async_channel::unbounded();
            let progress_updates = cx.spawn(async move |cx| {
                while let Ok(value) = progress_receiver.recv().await {
                    progress.update(cx, |progress, cx| progress.set_value(value, cx));
                }
            });
            let import = cx.background_spawn(async move {
                image_import::import_screenshots(
                    &file_paths,
                    app_id,
                    jpeg_quality,
                    resize_filter,
                    move |progress| {
                        let _ = progress_sender.send_blocking(progress);
                    },
                )
            });
            let result = import.await;
            progress_updates.await;

            if let Err(dialog_error) = window_handle.update(cx, |_, window, cx| {
                window.close_dialog(cx);
            }) {
                error!("Failed to close screenshot import progress: {dialog_error}");
            }

            if let Err(update_error) = this.update(cx, |this, cx| {
                this.importing = false;
                cx.notify();
            }) {
                error!("Failed to reset import state: {update_error}");
            }

            let _ = window_handle.update(cx, |_, window, cx| match result {
                Ok(()) => window
                    .push_notification((NotificationType::Success, "Screenshots imported."), cx),
                Err(import_error) => {
                    error!("Screenshot import failed: {}", import_error.summary);
                    for failure in &import_error.errors {
                        error!("{}: {}", failure.file_path.display(), failure.message);
                    }
                    window.push_notification((NotificationType::Error, import_error.summary), cx);
                }
            });
        })
        .detach();
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
            cx.listener(move |this, _, _, cx| {
                this.start_import(app_id, cx);
            }),
        )
    }

    fn welcome_text(&self) -> String {
        self.steam_user.as_ref().map_or_else(
            || "WELCOME USER!".to_owned(),
            |user| format!("WELCOME {}!", user.to_uppercase()),
        )
    }

    fn render_home_page(&self, cx: &Context<Self>) -> HomePage {
        let search_query = self.game_search.read(cx).query(cx);
        let query = search_query.trim();
        let matching_indices = self
            .cards
            .iter()
            .enumerate()
            .filter_map(|(index, card)| game_matches_search(&card.app_name, query).then_some(index))
            .collect::<Vec<_>>();
        let cards = matching_indices
            .iter()
            .copied()
            .map(|index| self.render_card(index, cx).into_any_element())
            .collect();
        let is_loading = matches!(self.library_state, LibraryState::Loading);
        let library_message = match &self.library_state {
            LibraryState::Ready if self.cards.is_empty() => {
                Some("No installed Steam games were found.".to_owned())
            }
            LibraryState::Ready if matching_indices.is_empty() => {
                Some(format!("No games found matching \"{query}\"."))
            }
            LibraryState::Failed(message) => Some(format!("Error: {message}")),
            LibraryState::Loading | LibraryState::Ready => None,
        };

        HomePage::new(
            self.welcome_text(),
            self.game_search.clone(),
            matches!(self.library_state, LibraryState::Ready) && !self.cards.is_empty(),
            cards,
            is_loading,
            library_message,
        )
    }
}

fn game_matches_search(game_name: &str, query: &str) -> bool {
    let query = query.trim();
    query.is_empty() || game_name.to_lowercase().contains(&query.to_lowercase())
}

impl Render for SteamScreenshotImporter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);
        let background = cx.theme().background;
        let page = match self.router.current() {
            Route::Home => {
                self.tick_cards(window);
                self.render_home_page(cx).into_any_element()
            }
            Route::About => AboutPage.into_any_element(),
            Route::Options => self.options_page.clone().into_any_element(),
        };

        let content = div()
            .relative()
            .size_full()
            .bg(background)
            .child(page)
            .child(footer(cx))
            .child(self.menu.clone())
            .child(theme_toggle(cx))
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer);

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
    let image = assets::load("assets/defaultappimage.png")
        .expect("default app image should be included in application assets");
    decode_artwork(&image).expect("bundled default game artwork should decode")
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
    if let Some(mode) = cx.global::<Preferences>().theme.get().mode() {
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
        .with_assets(assets::Assets::new())
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
                    if let Some(mode) = cx.global::<Preferences>().theme.get().mode() {
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

    #[test]
    fn game_search_is_case_insensitive_and_ignores_surrounding_whitespace() {
        assert!(game_matches_search("Half-Life 2", " half-LIFE "));
        assert!(game_matches_search("Portal", ""));
        assert!(!game_matches_search("Portal", "Dota"));
    }
}
