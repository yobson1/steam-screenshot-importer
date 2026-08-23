#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod components;
mod image_fetch;
mod offscreen;
mod steam_locate;

use std::{
    cell::Cell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use components::game_tile::{
    GameTileMotion, GameTileProps, Pointer, game_tile, offscreen_vertices, pointer_in_bounds,
    projected_vertices_changed,
};
#[cfg(debug_assertions)]
use gpui::hsla;
use gpui::{
    App, Bounds, Context, MouseMoveEvent, Pixels, Render, RenderImage, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, rgb, size,
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
    app_name: String,
    artwork_handle: usize,
    artwork: Arc<RenderImage>,
    artwork_is_projected: bool,
    missing_artwork: bool,
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

#[cfg(debug_assertions)]
struct FrameStats {
    last_frame: Instant,
    sample_started: Instant,
    sample_frames: u16,
    sample_time: Duration,
    sample_worst: Duration,
    fps: f32,
    average_ms: f32,
    worst_ms: f32,
}

#[cfg(debug_assertions)]
impl FrameStats {
    fn new(now: Instant) -> Self {
        Self {
            last_frame: now,
            sample_started: now,
            sample_frames: 0,
            sample_time: Duration::ZERO,
            sample_worst: Duration::ZERO,
            fps: 0.0,
            average_ms: 0.0,
            worst_ms: 0.0,
        }
    }

    fn record_frame(&mut self, now: Instant) {
        let frame_time = now.saturating_duration_since(self.last_frame);
        self.last_frame = now;

        // GPUI does not continuously repaint an idle window. Exclude idle gaps so the
        // diagnostic measures active animation rather than time spent doing no work.
        if frame_time > Duration::from_millis(100) {
            self.sample_started = now;
            self.sample_frames = 0;
            self.sample_time = Duration::ZERO;
            self.sample_worst = Duration::ZERO;
            return;
        }

        self.sample_frames += 1;
        self.sample_time += frame_time;
        self.sample_worst = self.sample_worst.max(frame_time);

        let sample_duration = now.saturating_duration_since(self.sample_started);
        if sample_duration >= Duration::from_millis(500) {
            self.fps = f32::from(self.sample_frames) / sample_duration.as_secs_f32();
            self.average_ms =
                self.sample_time.as_secs_f32() * 1_000.0 / f32::from(self.sample_frames.max(1));
            self.worst_ms = self.sample_worst.as_secs_f32() * 1_000.0;
            self.sample_started = now;
            self.sample_frames = 0;
            self.sample_time = Duration::ZERO;
            self.sample_worst = Duration::ZERO;
        }
    }
}

struct SteamScreenshotImporter {
    cards: Vec<CardState>,
    offscreen: OffscreenRenderer,
    retired_images: Vec<Arc<RenderImage>>,
    steam_user: Option<String>,
    library_state: LibraryState,
    last_frame: Instant,
    #[cfg(debug_assertions)]
    frame_stats: FrameStats,
}

impl SteamScreenshotImporter {
    fn new(cx: &mut Context<Self>) -> Self {
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
        Self {
            cards: Vec::new(),
            offscreen: OffscreenRenderer::new()
                .expect("failed to initialize offscreen WGPU card renderer"),
            retired_images: Vec::new(),
            steam_user: None,
            library_state: LibraryState::Loading,
            last_frame: now,
            #[cfg(debug_assertions)]
            frame_stats: FrameStats::new(now),
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
                    missing_artwork: game.missing_artwork,
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
            app_name: card.app_name.clone(),
            artwork: card.artwork.clone(),
            artwork_is_projected: card.artwork_is_projected,
            missing_artwork: card.missing_artwork,
            bounds: card.bounds.clone(),
            pointer: card.motion.pointer,
            hover: card.motion.hover,
            glare: card.motion.glare,
        };
        let app_id = card.app_id;

        game_tile(
            props,
            cx.listener(move |this, hovering, _, cx| {
                let motion = &mut this.cards[index].motion;
                motion.hover_target = *hovering;
                if !hovering {
                    motion.pointer_target = Pointer::default();
                }
                cx.notify();
            }),
            cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                let card = &mut this.cards[index];
                card.motion.hover_target = true;
                card.motion.pointer_target = pointer_in_bounds(event.position, card.bounds.get());
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
        #[cfg(debug_assertions)]
        self.frame_stats.record_frame(Instant::now());

        let cards = (0..self.cards.len())
            .map(|index| self.render_card(index, cx).into_any_element())
            .collect::<Vec<_>>();
        let library_message = match &self.library_state {
            LibraryState::Loading => Some("Fetching games.".to_owned()),
            LibraryState::Ready if self.cards.is_empty() => {
                Some("No installed Steam games were found.".to_owned())
            }
            LibraryState::Failed(message) => Some(format!("Error: {message}")),
            LibraryState::Ready => None,
        };

        let content = div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x001f_2022))
            .text_color(rgb(0x00bf_c2c7))
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
                            .text_color(rgb(0x00eb_6841))
                            .child(self.welcome_text()),
                    ),
            )
            .child(
                div()
                    .id("game-library-scroll")
                    .flex_1()
                    .w_full()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w_full()
                            .px_5()
                            .pb_8()
                            .flex()
                            .flex_wrap()
                            .justify_center()
                            .items_start()
                            .when_some(library_message, |gallery, message| {
                                gallery.child(
                                    div()
                                        .w_full()
                                        .text_center()
                                        .text_sm()
                                        .text_color(rgb(0x008d_9299))
                                        .child(message),
                                )
                            })
                            .children(cards),
                    ),
            );

        #[cfg(debug_assertions)]
        let content = content.child(
            div()
                .absolute()
                .top_4()
                .right_4()
                .px_3()
                .py_2()
                .rounded_md()
                .bg(hsla(0.0, 0.0, 0.05, 0.82))
                .border_1()
                .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                .text_xs()
                .text_color(rgb(0x00e3_e5e8))
                .flex()
                .flex_col()
                .items_end()
                .child(format!("{:.1} FPS", self.frame_stats.fps))
                .child(format!("{:.2} ms avg", self.frame_stats.average_ms))
                .child(format!("{:.2} ms worst", self.frame_stats.worst_ms)),
        );

        content
    }
}

fn load_steam_library() -> Result<LoadedLibrary, String> {
    let steam_user = steam_locate::get_recent_steam_user()
        .ok()
        .filter(|user| !user.is_empty());
    let games = steam_locate::get_games()?;
    let games = games
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
        .collect();
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
    RgbaImage::from_pixel(
        ARTWORK_WIDTH,
        ARTWORK_HEIGHT,
        image::Rgba([0x2a, 0x2a, 0x2a, 0xff]),
    )
}

fn rgba_to_bgra(pixels: &mut RgbaImage) {
    for pixel in pixels.pixels_mut() {
        pixel.0.swap(0, 2);
    }
}

fn render_image_from_bgra(pixels: RgbaImage) -> Arc<RenderImage> {
    Arc::new(RenderImage::new(vec![Frame::new(pixels)]))
}

fn main() {
    let _ = simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init();

    gpui_platform::application().run(|cx: &mut App| {
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
            |_, cx| cx.new(SteamScreenshotImporter::new),
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
