use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

use gpui::{
    App, Bounds, BoxShadow, ClickEvent, Corners, MouseMoveEvent, PathBuilder, Pixels, Point,
    RenderImage, Window, canvas, div, hsla, linear_color_stop, linear_gradient, point, prelude::*,
    px, size,
};

use crate::offscreen::{OUTPUT_HEIGHT_F32, OUTPUT_WIDTH_F32, ProjectedVertex};

pub const CARD_WIDTH: f32 = 212.0;
pub const CARD_HEIGHT: f32 = 318.0;
pub const SLOT_WIDTH: f32 = 250.0;
pub const SLOT_HEIGHT: f32 = 366.0;

const PERSPECTIVE: f32 = 900.0;
const MAX_TILT_DEGREES: f32 = 12.0;
const HOVER_SCALE: f32 = 1.1;
const PROJECTED_VERTEX_EPSILON: f32 = 0.1;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pointer {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub struct GameTileMotion {
    pub hover_target: bool,
    pub hover: f32,
    pub pointer_target: Pointer,
    pub pointer: Pointer,
    pub glare: f32,
}

impl Default for GameTileMotion {
    fn default() -> Self {
        Self {
            hover_target: false,
            hover: 0.0,
            pointer_target: Pointer::default(),
            pointer: Pointer::default(),
            glare: 0.0,
        }
    }
}

impl GameTileMotion {
    pub fn tick(&mut self, dt: Duration) -> bool {
        let hover_target = f32::from(self.hover_target);
        let pointer_target = if self.hover_target {
            self.pointer_target
        } else {
            Pointer::default()
        };
        let glare_target = if self.hover_target {
            ((self.pointer_target.y + 1.0) * 0.25).clamp(0.0, 0.5)
        } else {
            0.0
        };

        self.hover = approach(self.hover, hover_target, dt, 13.0);
        self.pointer.x = approach(self.pointer.x, pointer_target.x, dt, 26.0);
        self.pointer.y = approach(self.pointer.y, pointer_target.y, dt, 26.0);
        self.glare = approach(self.glare, glare_target, dt, 22.0);

        (self.hover - hover_target).abs() > 0.001
            || (self.pointer.x - pointer_target.x).abs() > 0.001
            || (self.pointer.y - pointer_target.y).abs() > 0.001
            || (self.glare - glare_target).abs() > 0.001
    }
}

pub struct GameTileProps {
    pub index: usize,
    pub artwork: Arc<RenderImage>,
    pub artwork_is_projected: bool,
    pub bounds: Rc<Cell<Bounds<Pixels>>>,
    pub pointer: Pointer,
    pub hover: f32,
    pub glare: f32,
}

pub fn game_tile(
    props: GameTileProps,
    on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static,
    on_mouse_move: impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let left = (SLOT_WIDTH - CARD_WIDTH) * 0.5;
    let top = (SLOT_HEIGHT - CARD_HEIGHT) * 0.5;

    div()
        .relative()
        .flex_none()
        .w(px(SLOT_WIDTH))
        .h(px(SLOT_HEIGHT))
        .child(
            div()
                .id(("game-card", props.index))
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(CARD_WIDTH))
                .h(px(CARD_HEIGHT))
                .cursor_pointer()
                .child(projected_card_canvas(
                    props.bounds,
                    props.artwork,
                    props.artwork_is_projected,
                    props.pointer,
                    props.hover,
                    props.glare,
                ))
                .on_hover(on_hover)
                .on_mouse_move(on_mouse_move)
                .on_click(on_click),
        )
}

#[derive(Clone, Copy, Debug)]
struct ProjectedQuad {
    top_left: Point<Pixels>,
    top_right: Point<Pixels>,
    bottom_right: Point<Pixels>,
    bottom_left: Point<Pixels>,
}

impl ProjectedQuad {
    fn bounds(self) -> Bounds<Pixels> {
        let points = [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ];
        let left = points
            .iter()
            .map(|point| f32::from(point.x))
            .fold(f32::INFINITY, f32::min);
        let right = points
            .iter()
            .map(|point| f32::from(point.x))
            .fold(f32::NEG_INFINITY, f32::max);
        let top = points
            .iter()
            .map(|point| f32::from(point.y))
            .fold(f32::INFINITY, f32::min);
        let bottom = points
            .iter()
            .map(|point| f32::from(point.y))
            .fold(f32::NEG_INFINITY, f32::max);

        Bounds::new(
            point(px(left), px(top)),
            size(px(right - left), px(bottom - top)),
        )
    }

    fn path(self) -> Option<gpui::Path<Pixels>> {
        quad_path(
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct CardProjection {
    center: Point<Pixels>,
    half_width: f32,
    half_height: f32,
    scale: f32,
    tilt_radians: f32,
}

impl CardProjection {
    fn new(bounds: Bounds<Pixels>, pointer: Pointer, hover: f32) -> Self {
        Self {
            center: point(
                bounds.origin.x + bounds.size.width * 0.5,
                bounds.origin.y + bounds.size.height * 0.5,
            ),
            half_width: f32::from(bounds.size.width) * 0.5,
            half_height: f32::from(bounds.size.height) * 0.5,
            scale: 1.0 + (HOVER_SCALE - 1.0) * hover,
            tilt_radians: (-MAX_TILT_DEGREES * pointer.y).to_radians(),
        }
    }

    fn clip_w(self) -> Corners<f32> {
        let top = self.homogeneous_w(-self.half_height);
        let bottom = self.homogeneous_w(self.half_height);
        Corners {
            top_left: top,
            top_right: top,
            bottom_right: bottom,
            bottom_left: bottom,
        }
    }

    fn homogeneous_w(self, local_y: f32) -> f32 {
        let depth = local_y * self.scale * self.tilt_radians.sin();
        1.0 - depth / PERSPECTIVE
    }

    fn screen_point(self, local_x: f32, local_y: f32) -> Point<Pixels> {
        let scaled_x = local_x * self.scale;
        let scaled_y = local_y * self.scale;
        let depth = scaled_y * self.tilt_radians.sin();
        let perspective_scale = PERSPECTIVE / (PERSPECTIVE - depth);

        point(
            self.center.x + px(scaled_x * perspective_scale),
            self.center.y + px(scaled_y * self.tilt_radians.cos() * perspective_scale),
        )
    }

    fn quad(self) -> ProjectedQuad {
        ProjectedQuad {
            top_left: self.screen_point(-self.half_width, -self.half_height),
            top_right: self.screen_point(self.half_width, -self.half_height),
            bottom_right: self.screen_point(self.half_width, self.half_height),
            bottom_left: self.screen_point(-self.half_width, self.half_height),
        }
    }

    fn flat_bounds(self) -> Bounds<Pixels> {
        Bounds::new(
            point(
                self.center.x - px(self.half_width * self.scale),
                self.center.y - px(self.half_height * self.scale),
            ),
            size(
                px(self.half_width * self.scale * 2.0),
                px(self.half_height * self.scale * 2.0),
            ),
        )
    }
}

fn projected_card_canvas(
    bounds_probe: Rc<Cell<Bounds<Pixels>>>,
    artwork: Arc<RenderImage>,
    artwork_is_projected: bool,
    pointer: Pointer,
    hover: f32,
    glare: f32,
) -> impl IntoElement {
    canvas(
        move |bounds, _, _| bounds_probe.set(bounds),
        move |bounds, (), window, _| {
            let projection = CardProjection::new(bounds, pointer, hover);
            paint_card_shadow(projection, window);
            paint_card_artwork(projection, &artwork, artwork_is_projected, window);

            if glare > 0.001 {
                paint_glare(projection, pointer, glare, window);
            }
        },
    )
    .absolute()
    .left_0()
    .top_0()
    .size_full()
}

fn paint_card_shadow(projection: CardProjection, window: &mut Window) {
    window.paint_drop_shadows(
        projection.quad().bounds(),
        Corners::default(),
        &[BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.78),
            offset: point(px(0.0), px(5.0)),
            blur_radius: px(10.0),
            spread_radius: px(0.0),
            inset: false,
        }],
    );
}

fn paint_card_artwork(
    projection: CardProjection,
    artwork: &Arc<RenderImage>,
    artwork_is_projected: bool,
    window: &mut Window,
) {
    let image_bounds = if artwork_is_projected {
        let output_size = size(px(OUTPUT_WIDTH_F32), px(OUTPUT_HEIGHT_F32));
        Bounds::new(
            point(
                projection.center.x - output_size.width * 0.5,
                projection.center.y - output_size.height * 0.5,
            ),
            output_size,
        )
    } else {
        projection.flat_bounds()
    };
    let _ = window.paint_image(
        image_bounds,
        image_bounds,
        Corners::default(),
        artwork.clone(),
        0,
        false,
    );
}

#[allow(
    clippy::similar_names,
    reason = "paired x/y direction components intentionally share coordinate terminology"
)]
fn paint_glare(
    projection: CardProjection,
    pointer: Pointer,
    glare_opacity: f32,
    window: &mut Window,
) {
    let Some(path) = projection.quad().path() else {
        return;
    };

    let local_dx = pointer.x * projection.half_width;
    let local_dy = pointer.y * projection.half_height;
    let angle = if local_dx.abs() + local_dy.abs() < f32::EPSILON {
        0.0
    } else {
        local_dx.atan2(-local_dy).to_degrees().rem_euclid(360.0)
    };
    let radians = angle.to_radians();
    let direction_x = radians.sin();
    let direction_y = -radians.cos();
    let glare_size = 4.0 * projection.half_width.max(projection.half_height);
    let half_gradient_span =
        direction_x.abs() * projection.half_width + direction_y.abs() * projection.half_height;
    let low = (0.5 - half_gradient_span / glare_size).clamp(0.0, 1.0);
    let high = (0.5 + half_gradient_span / glare_size).clamp(0.0, 1.0);

    let direction_extent = projection.half_width.min(projection.half_height) * 0.5;
    let direction_start = projection.screen_point(
        -direction_x * direction_extent,
        -direction_y * direction_extent,
    );
    let direction_end = projection.screen_point(
        direction_x * direction_extent,
        direction_y * direction_extent,
    );
    let screen_dx = f32::from(direction_end.x - direction_start.x);
    let screen_dy = f32::from(direction_end.y - direction_start.y);
    let screen_angle = screen_dx.atan2(-screen_dy).to_degrees().rem_euclid(360.0);

    window.paint_path(
        path,
        linear_gradient(
            screen_angle,
            linear_color_stop(hsla(0.0, 0.0, 1.0, low * glare_opacity), 0.0),
            linear_color_stop(hsla(0.0, 0.0, 1.0, high * glare_opacity), 1.0),
        ),
    );
}

pub fn offscreen_vertices(pointer: Pointer, hover: f32) -> [ProjectedVertex; 4] {
    let card_bounds = Bounds::new(
        point(
            px((OUTPUT_WIDTH_F32 - CARD_WIDTH) * 0.5),
            px((OUTPUT_HEIGHT_F32 - CARD_HEIGHT) * 0.5),
        ),
        size(px(CARD_WIDTH), px(CARD_HEIGHT)),
    );
    let projection = CardProjection::new(card_bounds, pointer, hover);
    let quad = projection.quad();
    let clip_w = projection.clip_w();

    [
        ProjectedVertex {
            x: f32::from(quad.top_left.x),
            y: f32::from(quad.top_left.y),
            w: clip_w.top_left,
        },
        ProjectedVertex {
            x: f32::from(quad.top_right.x),
            y: f32::from(quad.top_right.y),
            w: clip_w.top_right,
        },
        ProjectedVertex {
            x: f32::from(quad.bottom_left.x),
            y: f32::from(quad.bottom_left.y),
            w: clip_w.bottom_left,
        },
        ProjectedVertex {
            x: f32::from(quad.bottom_right.x),
            y: f32::from(quad.bottom_right.y),
            w: clip_w.bottom_right,
        },
    ]
}

pub fn projected_vertices_changed(
    previous: [ProjectedVertex; 4],
    next: [ProjectedVertex; 4],
) -> bool {
    previous.into_iter().zip(next).any(|(previous, next)| {
        (previous.x - next.x).abs() >= PROJECTED_VERTEX_EPSILON
            || (previous.y - next.y).abs() >= PROJECTED_VERTEX_EPSILON
    })
}

pub fn pointer_in_bounds(position: Point<Pixels>, bounds: Bounds<Pixels>) -> Pointer {
    if bounds.size.width <= Pixels::ZERO || bounds.size.height <= Pixels::ZERO {
        return Pointer::default();
    }

    Pointer {
        x: (((position.x - bounds.origin.x) / bounds.size.width) * 2.0 - 1.0).clamp(-1.0, 1.0),
        y: (((position.y - bounds.origin.y) / bounds.size.height) * 2.0 - 1.0).clamp(-1.0, 1.0),
    }
}

fn quad_path(
    top_left: Point<Pixels>,
    top_right: Point<Pixels>,
    bottom_right: Point<Pixels>,
    bottom_left: Point<Pixels>,
) -> Option<gpui::Path<Pixels>> {
    let mut builder = PathBuilder::fill();
    builder.move_to(top_left);
    builder.line_to(top_right);
    builder.line_to(bottom_right);
    builder.line_to(bottom_left);
    builder.close();
    builder.build().ok()
}

fn approach(current: f32, target: f32, dt: Duration, responsiveness: f32) -> f32 {
    let blend = 1.0 - (-responsiveness * dt.as_secs_f32()).exp();
    current + (target - current) * blend
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_coordinates_are_normalized_and_clamped() {
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(200.0), px(300.0)));
        assert_eq!(
            pointer_in_bounds(point(px(110.0), px(170.0)), bounds),
            Pointer { x: 0.0, y: 0.0 }
        );
        assert_eq!(
            pointer_in_bounds(point(px(-50.0), px(500.0)), bounds),
            Pointer { x: -1.0, y: 1.0 }
        );
    }

    #[test]
    fn horizontal_motion_does_not_change_projected_artwork() {
        let left = offscreen_vertices(Pointer { x: -1.0, y: 0.4 }, 1.0);
        let right = offscreen_vertices(Pointer { x: 1.0, y: 0.4 }, 1.0);
        assert_eq!(left, right);
    }

    #[test]
    fn approach_is_frame_rate_independent_and_does_not_overshoot() {
        let after_one_frame = approach(0.0, 1.0, Duration::from_millis(16), 13.0);
        let after_two_frames = approach(after_one_frame, 1.0, Duration::from_millis(16), 13.0);
        let after_one_long_frame = approach(0.0, 1.0, Duration::from_millis(32), 13.0);

        assert!((after_two_frames - after_one_long_frame).abs() < 0.000_001);
        assert!((0.0..1.0).contains(&after_one_frame));
    }

    #[test]
    fn zero_angle_projection_is_an_ordinary_scaled_rectangle() {
        let projection = CardProjection::new(
            Bounds::new(
                point(Pixels::ZERO, Pixels::ZERO),
                size(px(CARD_WIDTH), px(CARD_HEIGHT)),
            ),
            Pointer::default(),
            1.0,
        );
        let quad = projection.quad();

        assert!((f32::from(quad.bounds().size.width) - CARD_WIDTH * HOVER_SCALE).abs() < 0.000_1);
        assert!((f32::from(quad.bounds().size.height) - CARD_HEIGHT * HOVER_SCALE).abs() < 0.000_1);
        assert_eq!(projection.clip_w(), Corners::all(1.0));
    }

    #[test]
    fn positive_pitch_makes_the_top_recede_and_bottom_advance() {
        let projection = CardProjection::new(
            Bounds::new(
                point(Pixels::ZERO, Pixels::ZERO),
                size(px(CARD_WIDTH), px(CARD_HEIGHT)),
            ),
            Pointer { x: 0.0, y: -1.0 },
            1.0,
        );
        let quad = projection.quad();
        let top_width = f32::from(quad.top_right.x - quad.top_left.x);
        let bottom_width = f32::from(quad.bottom_right.x - quad.bottom_left.x);
        let clip_w = projection.clip_w();

        assert!(top_width < bottom_width);
        assert!(clip_w.top_left > 1.0);
        assert!(clip_w.bottom_left < 1.0);
    }

    #[test]
    fn subpixel_projection_changes_are_coalesced() {
        let previous = offscreen_vertices(Pointer::default(), 0.0);
        let mut next = previous;
        next[0].x += PROJECTED_VERTEX_EPSILON * 0.5;

        assert!(!projected_vertices_changed(previous, next));
        next[0].x += PROJECTED_VERTEX_EPSILON;
        assert!(projected_vertices_changed(previous, next));
    }
}
