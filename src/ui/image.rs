//! Half-block image preview: decode an image and render it as colored upper-half
//! block (▀) cells — fg = upper pixel, bg = lower pixel, two pixels per cell.
//! Uses only normal text cells, so it works in any truecolor terminal, through
//! tmux, and over SSH (no terminal graphics protocol needed).
use image::imageops::FilterType;
use image::RgbaImage;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// True for filenames with a previewable raster image extension.
pub fn is_image(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "tga"
    )
}

/// Render `img` into at most `cols`×`rows` character cells, preserving aspect.
pub fn halfblocks(img: &RgbaImage, cols: u16, rows: u16) -> Vec<Line<'static>> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let (iw, ih) = (img.width().max(1), img.height().max(1));
    let tw = cols as u32;
    let th = rows as u32 * 2; // two stacked pixels per cell
    let scale = (tw as f32 / iw as f32).min(th as f32 / ih as f32);
    let nw = ((iw as f32 * scale).round() as u32).clamp(1, tw);
    let nh = ((ih as f32 * scale).round() as u32).clamp(1, th);
    let r = image::imageops::resize(img, nw, nh, FilterType::Lanczos3);

    let mut lines = Vec::with_capacity((nh as usize).div_ceil(2));
    let mut y = 0u32;
    while y < nh {
        let mut spans = Vec::with_capacity(nw as usize);
        for x in 0..nw {
            let top = r.get_pixel(x, y).0;
            let bot = if y + 1 < nh { r.get_pixel(x, y + 1).0 } else { top };
            spans.push(Span::styled(
                "\u{2580}",
                Style::default()
                    .fg(Color::Rgb(top[0], top[1], top[2]))
                    .bg(Color::Rgb(bot[0], bot[1], bot[2])),
            ));
        }
        lines.push(Line::from(spans));
        y += 2;
    }
    lines
}
