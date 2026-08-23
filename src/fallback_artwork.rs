use cosmic_text::{Align, Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache, Wrap};
use image::RgbaImage;

const DISPLAY_CARD_WIDTH: f32 = 212.0;
const DISPLAY_TITLE_WIDTH: f32 = 180.0;
const DISPLAY_FONT_SIZE: f32 = 24.0;
const LINE_HEIGHT_SCALE: f32 = 1.2;

pub struct TitleRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl TitleRenderer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "artwork dimensions and centered text offsets are small pixel values"
    )]
    pub fn composite_title(&mut self, artwork: &mut RgbaImage, title: &str) {
        let raster_scale = artwork.width() as f32 / DISPLAY_CARD_WIDTH;
        let content_width = DISPLAY_TITLE_WIDTH * raster_scale;
        let font_size = DISPLAY_FONT_SIZE * raster_scale;
        let metrics = Metrics::relative(font_size, LINE_HEIGHT_SCALE);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        let mut buffer = buffer.borrow_with(&mut self.font_system);
        buffer.set_size(Some(content_width), Some(artwork.height() as f32));
        buffer.set_wrap(Wrap::Word);
        buffer.set_text(title, &Attrs::new(), Shaping::Advanced, Some(Align::Center));

        let (text_top, text_bottom) =
            buffer
                .layout_runs()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |bounds, run| {
                    (
                        bounds.0.min(run.line_top),
                        bounds.1.max(run.line_top + run.line_height),
                    )
                });
        if !text_top.is_finite() || !text_bottom.is_finite() {
            return;
        }

        let x_offset = ((artwork.width() as f32 - content_width) * 0.5).round() as i32;
        let text_height = text_bottom - text_top;
        let y_offset = ((artwork.height() as f32 - text_height) * 0.5 - text_top).round() as i32;
        let shadow_offset = raster_scale.round().max(1.0) as i32;

        buffer.draw(
            &mut self.swash_cache,
            Color::rgba(0, 0, 0, 77),
            |x, y, width, height, color| {
                paint_bgra_rect(
                    artwork,
                    x + x_offset + shadow_offset,
                    y + y_offset + shadow_offset,
                    width,
                    height,
                    color,
                );
            },
        );
        buffer.draw(
            &mut self.swash_cache,
            Color::rgb(0xb9, 0xc2, 0xcc),
            |x, y, width, height, color| {
                paint_bgra_rect(artwork, x + x_offset, y + y_offset, width, height, color);
            },
        );
    }
}

fn paint_bgra_rect(artwork: &mut RgbaImage, x: i32, y: i32, width: u32, height: u32, color: Color) {
    if color.a() == 0 {
        return;
    }

    for row in 0..height {
        let Ok(row) = i32::try_from(row) else {
            continue;
        };
        let Ok(pixel_y) = u32::try_from(y + row) else {
            continue;
        };
        if pixel_y >= artwork.height() {
            continue;
        }

        for column in 0..width {
            let Ok(column) = i32::try_from(column) else {
                continue;
            };
            let Ok(pixel_x) = u32::try_from(x + column) else {
                continue;
            };
            if pixel_x >= artwork.width() {
                continue;
            }

            let pixel = artwork.get_pixel_mut(pixel_x, pixel_y);
            pixel[0] = blend_channel(color.b(), pixel[0], color.a());
            pixel[1] = blend_channel(color.g(), pixel[1], color.a());
            pixel[2] = blend_channel(color.r(), pixel[2], color.a());
        }
    }
}

fn blend_channel(source: u8, destination: u8, alpha: u8) -> u8 {
    let alpha = u32::from(alpha);
    let inverse_alpha = 255 - alpha;
    let blended = (u32::from(source) * alpha + u32::from(destination) * inverse_alpha + 127) / 255;
    u8::try_from(blended).expect("alpha blending produces one byte")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_blending_handles_transparent_and_opaque_sources() {
        assert_eq!(blend_channel(200, 40, 0), 40);
        assert_eq!(blend_channel(200, 40, 255), 200);
        assert_eq!(blend_channel(200, 40, 128), 120);
    }
}
