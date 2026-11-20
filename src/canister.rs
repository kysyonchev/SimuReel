//! Draws a little picture of the film (a 35mm canister, or an instant-film pack) with egui's painter.

use crate::films::{Film, Kind};
use eframe::egui::{pos2, vec2, Align2, Color32, FontId, Painter, Rect, Stroke};

fn rgb(c: u32) -> Color32 {
    Color32::from_rgb(((c >> 16) & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, (c & 0xFF) as u8)
}

fn lum(c: Color32) -> f32 {
    0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32
}

/// Greedy word wrap on a character budget.
fn wrap(s: &str, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in s.split_whitespace() {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + word.chars().count() <= max => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

pub fn draw(p: &Painter, r: Rect, film: &Film, selected: bool, hovered: bool) {
    let bg = if hovered { Color32::from_gray(50) } else { Color32::from_gray(34) };
    p.rect_filled(r, 8.0, bg);
    if selected {
        p.rect_stroke(r, 8.0, Stroke::new(2.5, Color32::from_rgb(255, 170, 40)));
    }
    let body = rgb(film.body);
    let accent = rgb(film.accent);
    let txt = if lum(body) > 140.0 { Color32::from_gray(20) } else { Color32::WHITE };
    match film.kind {
        Kind::Instant => instant(p, r, film, body, accent, txt),
        _ => canister(p, r, film, body, accent, txt),
    }
}

fn canister(p: &Painter, r: Rect, film: &Film, body: Color32, accent: Color32, txt: Color32) {
    let (w, h) = (r.width(), r.height());
    let cw = w * 0.66;
    let cx = r.center().x;
    let (x0, x1) = (cx - cw / 2.0, cx + cw / 2.0);
    let top = r.top() + h * 0.05;
    let dark = Color32::from_rgb(38, 38, 42);

    // spool stem
    p.rect_filled(
        Rect::from_min_max(pos2(cx - cw * 0.12, top), pos2(cx + cw * 0.12, top + h * 0.07)),
        2.0,
        Color32::from_gray(150),
    );
    // top cap
    let cap_t = top + h * 0.05;
    let cap_b = cap_t + h * 0.07;
    p.rect_filled(Rect::from_min_max(pos2(x0 - 3.0, cap_t), pos2(x1 + 3.0, cap_b)), 3.0, dark);
    // body
    let body_b = r.bottom() - h * 0.09;
    let br = Rect::from_min_max(pos2(x0, cap_b), pos2(x1, body_b));
    p.rect_filled(br, 1.0, body);
    // bottom cap
    p.rect_filled(Rect::from_min_max(pos2(x0 - 3.0, body_b), pos2(x1 + 3.0, body_b + h * 0.065)), 3.0, dark);

    // accent stripes
    let bh = br.height();
    p.rect_filled(Rect::from_min_max(pos2(x0, br.top() + bh * 0.05), pos2(x1, br.top() + bh * 0.14)), 0.0, accent);
    p.rect_filled(Rect::from_min_max(pos2(x0, br.bottom() - bh * 0.15), pos2(x1, br.bottom() - bh * 0.06)), 0.0, accent);

    // label text
    p.text(pos2(cx, br.top() + bh * 0.20), Align2::CENTER_TOP, film.brand.to_uppercase(), FontId::proportional(9.0), txt);
    let mut y = br.top() + bh * 0.32;
    for l in wrap(film.name, 9).iter().take(3) {
        p.text(pos2(cx, y), Align2::CENTER_TOP, l, FontId::proportional(11.5), txt);
        y += 13.0;
    }
    p.text(pos2(cx, br.bottom() - bh * 0.18), Align2::CENTER_BOTTOM, film.iso.to_string(), FontId::proportional(15.0), txt);

    // cylinder shading
    p.rect_filled(Rect::from_min_max(pos2(x0 + cw * 0.06, br.top()), pos2(x0 + cw * 0.16, body_b)), 0.0, Color32::from_white_alpha(45));
    p.rect_filled(Rect::from_min_max(pos2(x1 - cw * 0.22, br.top()), pos2(x1, body_b)), 0.0, Color32::from_black_alpha(55));
}

fn instant(p: &Painter, r: Rect, film: &Film, body: Color32, accent: Color32, txt: Color32) {
    let w = r.width() * 0.74;
    let h = r.height() * 0.84;
    let rect = Rect::from_center_size(r.center(), vec2(w, h));
    p.rect_filled(rect.translate(vec2(2.0, 3.0)), 4.0, Color32::from_black_alpha(80));
    p.rect_filled(rect, 4.0, body);
    // photo window
    let win = Rect::from_min_max(
        pos2(rect.left() + w * 0.08, rect.top() + h * 0.06),
        pos2(rect.right() - w * 0.08, rect.top() + h * 0.50),
    );
    p.rect_filled(win, 2.0, Color32::from_gray(28));
    // accent stripe
    p.rect_filled(
        Rect::from_min_max(pos2(win.left(), win.bottom() + h * 0.02), pos2(win.right(), win.bottom() + h * 0.06)),
        0.0,
        accent,
    );
    let cx = rect.center().x;
    p.text(pos2(cx, rect.top() + h * 0.59), Align2::CENTER_TOP, film.brand.to_uppercase(), FontId::proportional(8.5), txt);
    let mut y = rect.top() + h * 0.69;
    for l in wrap(film.name, 10).iter().take(2) {
        p.text(pos2(cx, y), Align2::CENTER_TOP, l, FontId::proportional(11.0), txt);
        y += 12.5;
    }
    p.text(pos2(cx, rect.bottom() - 3.0), Align2::CENTER_BOTTOM, format!("ISO {}", film.iso), FontId::proportional(9.0), txt);
}
