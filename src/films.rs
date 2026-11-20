//! The film library. Every stock is an *approximation* of the character of the real film
//! (not an official profile). Tweak the numbers freely – this table is the whole "look" system.
//!
//! c(brand, name, iso, kind, canister_body_rgb, canister_accent_rgb,
//!   [contrast, saturation, warmth, tint, fade, grain], shadow_tone_rgb, highlight_tone_rgb)
//! bw(brand, name, iso, body, accent, channel_mixer, contrast, grain, fade)

use crate::engine::Look;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    ColorNeg,
    Slide,
    BlackWhite,
    Instant,
    Cine,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::ColorNeg, Kind::Slide, Kind::BlackWhite, Kind::Instant, Kind::Cine];
    pub fn short(self) -> &'static str {
        match self {
            Kind::ColorNeg => "Colour",
            Kind::Slide => "Slide",
            Kind::BlackWhite => "B&W",
            Kind::Instant => "Instant",
            Kind::Cine => "Cine",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Kind::ColorNeg => "Colour negative",
            Kind::Slide => "Slide / reversal",
            Kind::BlackWhite => "Black & white",
            Kind::Instant => "Instant",
            Kind::Cine => "Cine",
        }
    }
}

pub struct Film {
    pub brand: &'static str,
    pub name: &'static str,
    pub iso: u32,
    pub kind: Kind,
    pub body: u32,
    pub accent: u32,
    pub look: Look,
}

impl Film {
    fn hal(mut self, v: f32) -> Self {
        self.look.halation = v;
        self
    }
    fn mat(mut self, m: [[f32; 3]; 3]) -> Self {
        self.look.matrix = Some(m);
        self
    }
    fn vig(mut self, v: f32) -> Self {
        self.look.vignette = v;
        self
    }
    fn roll(mut self, v: f32) -> Self {
        self.look.rolloff = v;
        self
    }
    fn tone(mut self, sh: [f32; 3], hi: [f32; 3]) -> Self {
        self.look.shadow_tone = sh;
        self.look.highlight_tone = hi;
        self
    }
}

const Z: [f32; 3] = [0.0, 0.0, 0.0];
// black & white channel mixers (spectral sensitivity)
const PAN: [f32; 3] = [0.30, 0.59, 0.11];
const ORTHO: [f32; 3] = [0.02, 0.40, 0.58];
const EXTRED: [f32; 3] = [0.45, 0.45, 0.10];
const IR: [f32; 3] = [0.15, 1.10, -0.30];
const SOFT: [f32; 3] = [0.33, 0.50, 0.17];
// toning
const WARM_SH: [f32; 3] = [0.004, 0.0, -0.004];
const WARM_HI: [f32; 3] = [0.020, 0.010, -0.015];
const COOL_SH: [f32; 3] = [-0.004, 0.0, 0.006];
const COOL_HI: [f32; 3] = [-0.004, 0.0, 0.010];

use Kind::*;

fn base(kind: Kind) -> Look {
    Look {
        contrast: 0.0,
        saturation: 1.0,
        warmth: 0.0,
        tint: 0.0,
        fade: 0.0,
        grain: 0.3,
        rolloff: match kind {
            ColorNeg => 0.5,
            Slide => 0.1,
            BlackWhite => 0.4,
            Instant => 0.3,
            Cine => 0.6,
        },
        halation: 0.0,
        vignette: 0.0,
        shadow_tone: Z,
        highlight_tone: Z,
        bw: None,
        matrix: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn c(
    brand: &'static str,
    name: &'static str,
    iso: u32,
    kind: Kind,
    body: u32,
    accent: u32,
    p: [f32; 6],
    sh: [f32; 3],
    hi: [f32; 3],
) -> Film {
    let mut look = base(kind);
    look.contrast = p[0];
    look.saturation = p[1];
    look.warmth = p[2];
    look.tint = p[3];
    look.fade = p[4];
    look.grain = p[5];
    look.shadow_tone = sh;
    look.highlight_tone = hi;
    Film { brand, name, iso, kind, body, accent, look }
}

#[allow(clippy::too_many_arguments)]
fn bw(
    brand: &'static str,
    name: &'static str,
    iso: u32,
    body: u32,
    accent: u32,
    mix: [f32; 3],
    contrast: f32,
    grain: f32,
    fade: f32,
) -> Film {
    let mut look = base(BlackWhite);
    look.bw = Some(mix);
    look.contrast = contrast;
    look.grain = grain;
    look.fade = fade;
    Film { brand, name, iso, kind: BlackWhite, body, accent, look }
}

pub fn all() -> Vec<Film> {
    vec![
        // ============================================================ KODAK – colour negative
        c("Kodak", "Portra 160", 160, ColorNeg, 0xEBD9A0, 0xC8553D, [0.10, 0.88, 0.10, -0.03, 0.04, 0.18], [-0.004, 0.004, 0.010], [0.014, 0.008, 0.0]),
        c("Kodak", "Portra 400", 400, ColorNeg, 0xF0C75E, 0xB83227, [0.15, 0.93, 0.12, -0.04, 0.03, 0.30], [-0.004, 0.006, 0.010], [0.015, 0.008, 0.0]),
        c("Kodak", "Portra 800", 800, ColorNeg, 0xE9B04A, 0xA52A2A, [0.20, 0.98, 0.14, -0.02, 0.03, 0.48], [-0.004, 0.006, 0.012], [0.016, 0.008, -0.002]),
        c("Kodak", "Ektar 100", 100, ColorNeg, 0xD7263D, 0xFFD400, [0.40, 1.38, 0.06, 0.02, 0.0, 0.10], [-0.010, 0.0, 0.015], [0.010, 0.005, 0.0]),
        c("Kodak", "Gold 100", 100, ColorNeg, 0xF6C21A, 0x9E1B1B, [0.25, 1.10, 0.28, 0.04, 0.03, 0.25], [0.008, 0.0, -0.008], [0.020, 0.012, -0.010]),
        c("Kodak", "Gold 200", 200, ColorNeg, 0xF9B90F, 0xB71C1C, [0.28, 1.15, 0.30, 0.04, 0.03, 0.40], [0.010, 0.0, -0.010], [0.022, 0.012, -0.010]),
        c("Kodak", "UltraMax 400", 400, ColorNeg, 0xFFC107, 0x0B3D91, [0.35, 1.25, 0.22, 0.03, 0.02, 0.52], [0.005, 0.0, 0.005], [0.018, 0.010, -0.006]),
        c("Kodak", "ColorPlus 200", 200, ColorNeg, 0xFFD21F, 0xD32F2F, [0.30, 1.05, 0.28, 0.06, 0.04, 0.50], [0.010, 0.0, -0.006], [0.020, 0.010, -0.012]),
        c("Kodak", "Pro Image 100", 100, ColorNeg, 0xF4A300, 0x37474F, [0.30, 1.10, 0.18, 0.03, 0.02, 0.28], [0.0, 0.004, 0.004], [0.014, 0.008, -0.004]),
        c("Kodak", "Max 800", 800, ColorNeg, 0xFFB300, 0x1A237E, [0.30, 1.15, 0.25, 0.05, 0.03, 0.70], [0.008, 0.0, 0.0], [0.020, 0.010, -0.010]),
        c("Kodak", "Kodacolor VR 100", 100, ColorNeg, 0xE8A317, 0x7B1E1E, [0.20, 0.95, 0.35, 0.08, 0.08, 0.35], [0.012, 0.004, -0.010], [0.030, 0.018, -0.020]),
        c("Kodak", "Vericolor III 160", 160, ColorNeg, 0xD9C27A, 0x5D4037, [0.12, 0.85, 0.05, -0.02, 0.04, 0.20], [-0.004, 0.004, 0.008], [0.010, 0.006, 0.0]),
        // ============================================================ KODAK – slide
        c("Kodak", "Ektachrome E100", 100, Slide, 0x1565C0, 0xFFFFFF, [0.45, 1.22, -0.08, 0.0, 0.0, 0.10], [0.0, 0.004, 0.015], [0.0, 0.0, 0.004]),
        c("Kodak", "Ektachrome 100VS", 100, Slide, 0x0D47A1, 0xFFD600, [0.55, 1.45, -0.04, 0.02, 0.0, 0.12], [-0.004, 0.0, 0.016], [0.0, 0.0, 0.0]),
        c("Kodak", "Ektachrome 64", 64, Slide, 0x3F51B5, 0xFFFFFF, [0.50, 1.15, -0.02, 0.0, 0.0, 0.10], [0.0, 0.004, 0.012], [0.004, 0.0, 0.0]),
        c("Kodak", "Elitechrome 200", 200, Slide, 0x00838F, 0xFFEB3B, [0.45, 1.20, 0.05, 0.0, 0.0, 0.22], [0.0, 0.004, 0.010], [0.006, 0.004, 0.0]),
        c("Kodak", "Kodachrome 25", 25, Slide, 0xC8102E, 0xFFD100, [0.62, 1.30, 0.20, 0.03, 0.0, 0.06], [0.012, -0.004, -0.006], [0.016, 0.006, -0.012]),
        c("Kodak", "Kodachrome 64", 64, Slide, 0xC8102E, 0xFFD100, [0.60, 1.35, 0.18, 0.03, 0.0, 0.12], [0.010, -0.004, -0.004], [0.014, 0.006, -0.010]),
        c("Kodak", "Kodachrome 200", 200, Slide, 0xB0102A, 0xFFD100, [0.55, 1.28, 0.16, 0.03, 0.0, 0.30], [0.010, 0.0, -0.004], [0.014, 0.006, -0.008]),
        c("Kodak", "Aerochrome III (IR)", 400, Slide, 0x8E2B6B, 0xFF6F91, [0.40, 1.30, 0.0, 0.0, 0.0, 0.25], Z, Z)
            .mat([[0.15, 1.00, 0.00], [0.85, 0.00, 0.15], [0.00, 0.00, 0.90]]),
        // ============================================================ KODAK – cine
        c("Kodak", "Vision3 50D", 50, Cine, 0x2B2B2B, 0xFFEE58, [0.12, 0.95, 0.04, 0.0, 0.03, 0.12], [0.0, 0.004, 0.008], [0.008, 0.004, 0.0]),
        c("Kodak", "Vision3 250D", 250, Cine, 0x2B2B2B, 0xFFA726, [0.14, 0.95, 0.04, 0.0, 0.03, 0.25], [0.0, 0.004, 0.008], [0.010, 0.005, 0.0]),
        c("Kodak", "Vision3 200T", 200, Cine, 0x2B2B2B, 0xEF5350, [0.14, 0.95, -0.12, 0.0, 0.03, 0.22], [-0.004, 0.0, 0.012], [0.004, 0.0, 0.0]),
        c("Kodak", "Vision3 500T", 500, Cine, 0x2B2B2B, 0xE53935, [0.16, 0.98, -0.15, -0.02, 0.03, 0.40], [-0.006, 0.0, 0.014], [0.006, 0.002, 0.0]),
        // ============================================================ KODAK – B&W
        bw("Kodak", "Tri-X 400", 400, 0xE8B800, 0x111111, PAN, 0.45, 0.95, 0.02),
        bw("Kodak", "T-Max 100", 100, 0xD9A400, 0x0D47A1, PAN, 0.30, 0.18, 0.0),
        bw("Kodak", "T-Max 400", 400, 0xD9A400, 0x1565C0, PAN, 0.32, 0.45, 0.0),
        bw("Kodak", "T-Max P3200", 3200, 0xD9A400, 0xB71C1C, PAN, 0.50, 1.20, 0.03),
        bw("Kodak", "Plus-X 125", 125, 0xE0B000, 0x37474F, PAN, 0.35, 0.40, 0.02),
        bw("Kodak", "Double-X 5222", 250, 0x3A3A3A, 0xFFC107, PAN, 0.55, 0.85, 0.03),
        bw("Kodak", "Panatomic-X 32", 32, 0xCFA000, 0x424242, PAN, 0.38, 0.10, 0.0),
        bw("Kodak", "Verichrome Pan 125", 125, 0xC79A00, 0x6D4C41, PAN, 0.28, 0.40, 0.04).tone(WARM_SH, WARM_HI),
        bw("Kodak", "BW400CN", 400, 0x4E342E, 0xFFC107, PAN, 0.25, 0.45, 0.03).tone(WARM_SH, [0.012, 0.004, -0.008]),
        // ============================================================ FUJIFILM – colour negative
        c("Fujifilm", "Superia X-Tra 400", 400, ColorNeg, 0x2E7D32, 0xFFEB3B, [0.32, 1.22, -0.02, -0.06, 0.02, 0.50], [-0.008, 0.008, 0.004], [0.0, 0.006, 0.0]),
        c("Fujifilm", "Superia 200", 200, ColorNeg, 0x388E3C, 0xFFFFFF, [0.28, 1.15, -0.03, -0.05, 0.02, 0.38], [-0.006, 0.008, 0.004], [0.0, 0.006, 0.0]),
        c("Fujifilm", "Superia Premium 400", 400, ColorNeg, 0x1B5E20, 0xFFC107, [0.34, 1.20, 0.0, -0.05, 0.02, 0.44], [-0.006, 0.008, 0.006], [0.004, 0.006, 0.0]),
        c("Fujifilm", "Superia 1600", 1600, ColorNeg, 0x2E7D32, 0xFF7043, [0.30, 1.15, -0.02, -0.05, 0.03, 0.85], [-0.006, 0.008, 0.008], [0.0, 0.006, 0.0]),
        c("Fujifilm", "C200", 200, ColorNeg, 0x43A047, 0xFFFFFF, [0.26, 1.05, -0.04, -0.07, 0.03, 0.40], [-0.008, 0.010, 0.004], [0.0, 0.008, 0.0]),
        c("Fujifilm", "Industrial 100", 100, ColorNeg, 0x546E7A, 0x4CAF50, [0.30, 1.02, -0.02, -0.06, 0.03, 0.28], [-0.006, 0.008, 0.004], [0.0, 0.006, 0.0]),
        c("Fujifilm", "Pro 400H", 400, ColorNeg, 0xA7C7E7, 0x00695C, [0.12, 0.90, -0.06, -0.05, 0.05, 0.28], [-0.010, 0.010, 0.014], [0.0, 0.012, 0.008]),
        c("Fujifilm", "Pro 160NS", 160, ColorNeg, 0xE0C25A, 0x00695C, [0.12, 0.92, 0.02, -0.04, 0.04, 0.16], [-0.006, 0.006, 0.010], [0.006, 0.008, 0.0]),
        c("Fujifilm", "Natura 1600", 1600, ColorNeg, 0xE53935, 0xFFFFFF, [0.14, 0.98, -0.02, -0.04, 0.04, 0.80], [-0.004, 0.006, 0.012], [0.006, 0.006, 0.0]),
        c("Fujifilm", "Reala 100", 100, ColorNeg, 0x00A3A3, 0xFFFFFF, [0.22, 1.05, 0.0, -0.04, 0.02, 0.14], [-0.004, 0.006, 0.008], [0.004, 0.006, 0.0]),
        // ============================================================ FUJIFILM – slide
        c("Fujifilm", "Velvia 50", 50, Slide, 0x1B5E20, 0xFFD600, [0.58, 1.55, 0.0, 0.08, 0.0, 0.10], [-0.012, 0.0, 0.020], [0.0, 0.004, 0.0]),
        c("Fujifilm", "Velvia 100", 100, Slide, 0x2E7D32, 0xE91E63, [0.52, 1.42, 0.0, 0.06, 0.0, 0.12], [-0.010, 0.0, 0.016], [0.0, 0.004, 0.0]),
        c("Fujifilm", "Provia 100F", 100, Slide, 0x0D47A1, 0xFFFFFF, [0.40, 1.18, -0.02, 0.0, 0.0, 0.10], [0.0, 0.002, 0.008], [0.0, 0.0, 0.002]),
        c("Fujifilm", "Provia 400X", 400, Slide, 0x1976D2, 0xFFFFFF, [0.40, 1.20, -0.02, 0.0, 0.0, 0.28], [0.0, 0.002, 0.008], [0.0, 0.0, 0.002]),
        c("Fujifilm", "Astia 100F", 100, Slide, 0xEC407A, 0xFFFFFF, [0.28, 1.05, 0.02, 0.0, 0.01, 0.10], [0.0, 0.002, 0.006], [0.004, 0.002, 0.0]),
        c("Fujifilm", "Sensia 100", 100, Slide, 0x00ACC1, 0xFFFFFF, [0.40, 1.15, -0.02, 0.0, 0.0, 0.14], [0.0, 0.002, 0.008], [0.0, 0.0, 0.002]),
        c("Fujifilm", "Fortia SP", 50, Slide, 0xFF7043, 0x1B5E20, [0.60, 1.45, 0.14, 0.06, 0.0, 0.10], [0.004, 0.0, 0.010], [0.010, 0.004, -0.004]),
        // ============================================================ FUJIFILM – cine
        c("Fujifilm", "Eterna 500T", 500, Cine, 0x37474F, 0x00BFA5, [0.10, 0.85, -0.10, -0.03, 0.05, 0.35], [-0.006, 0.004, 0.012], [0.0, 0.006, 0.004]),
        c("Fujifilm", "Eterna Vivid 500", 500, Cine, 0x37474F, 0xFF5252, [0.26, 1.25, -0.06, -0.02, 0.03, 0.35], [-0.006, 0.0, 0.012], [0.004, 0.006, 0.0]),
        // ============================================================ FUJIFILM – B&W
        bw("Fujifilm", "Neopan Acros 100", 100, 0x424242, 0xFF6F00, PAN, 0.32, 0.12, 0.0),
        bw("Fujifilm", "Neopan Acros 100 II", 100, 0x303030, 0xFF8F00, PAN, 0.30, 0.10, 0.0).tone(COOL_SH, COOL_HI),
        bw("Fujifilm", "Neopan 400 Presto", 400, 0x455A64, 0xFFC107, PAN, 0.38, 0.55, 0.01),
        bw("Fujifilm", "Neopan SS 100", 100, 0x5D4037, 0xFFFFFF, PAN, 0.34, 0.20, 0.01),
        // ============================================================ FUJIFILM – instant
        c("Fujifilm", "Instax Mini", 800, Instant, 0xFAFAFA, 0x26C6DA, [0.20, 1.05, 0.02, -0.02, 0.06, 0.22], [0.0, 0.004, 0.008], [0.010, 0.006, 0.0]).vig(0.25),
        c("Fujifilm", "Instax Wide", 800, Instant, 0xF5F5F5, 0xAB47BC, [0.22, 1.08, 0.02, -0.02, 0.06, 0.22], [0.0, 0.004, 0.008], [0.010, 0.006, 0.0]).vig(0.28),
        c("Fujifilm", "FP-100C", 100, Instant, 0xE0E0E0, 0x00897B, [0.30, 1.10, 0.0, -0.02, 0.04, 0.12], [-0.004, 0.004, 0.010], [0.006, 0.006, 0.0]).vig(0.18),
        bw("Fujifilm", "Instax Mini Monochrome", 800, 0xF5F5F5, 0x212121, SOFT, 0.20, 0.25, 0.08).vig(0.25),
        // ============================================================ ILFORD / HARMAN / KENTMERE
        bw("Ilford", "HP5 Plus", 400, 0x2B2B2B, 0xFFFFFF, PAN, 0.38, 0.62, 0.015),
        bw("Ilford", "FP4 Plus", 125, 0x212121, 0x43A047, PAN, 0.34, 0.28, 0.01),
        bw("Ilford", "Delta 100", 100, 0x1A1A1A, 0x039BE5, PAN, 0.32, 0.12, 0.0),
        bw("Ilford", "Delta 400", 400, 0x1A1A1A, 0xFB8C00, PAN, 0.36, 0.30, 0.0),
        bw("Ilford", "Delta 3200", 3200, 0x1A1A1A, 0xE53935, PAN, 0.48, 1.10, 0.02),
        bw("Ilford", "Pan F Plus", 50, 0x212121, 0xFDD835, PAN, 0.40, 0.08, 0.0),
        bw("Ilford", "XP2 Super", 400, 0x263238, 0xFFFFFF, PAN, 0.26, 0.26, 0.02).tone(WARM_SH, [0.010, 0.004, -0.006]),
        bw("Ilford", "SFX 200", 200, 0x3E2723, 0xFF5252, EXTRED, 0.40, 0.45, 0.02).hal(0.6),
        bw("Ilford", "Ortho Plus", 80, 0x4A148C, 0xFFFFFF, ORTHO, 0.45, 0.20, 0.01),
        bw("Kentmere", "Pan 100", 100, 0x2E7D32, 0xFFFFFF, PAN, 0.30, 0.18, 0.02),
        bw("Kentmere", "Pan 400", 400, 0x1565C0, 0xFFFFFF, PAN, 0.32, 0.48, 0.02),
        c("Harman", "Phoenix 200", 200, ColorNeg, 0xE65100, 0xFFEB3B, [0.30, 1.25, 0.35, 0.18, 0.06, 0.90], [0.020, -0.006, 0.0], [0.040, 0.010, -0.020]).hal(0.8),
        // ============================================================ AGFA
        c("Agfa", "Vista 200", 200, ColorNeg, 0xD32F2F, 0xFFEB3B, [0.30, 1.20, 0.12, 0.10, 0.02, 0.45], [0.010, -0.004, 0.0], [0.016, 0.0, 0.0]),
        c("Agfa", "Vista Plus 400", 400, ColorNeg, 0xC62828, 0xFFC107, [0.32, 1.22, 0.10, 0.10, 0.02, 0.60], [0.010, -0.004, 0.0], [0.016, 0.0, 0.0]),
        c("Agfa", "Optima 100", 100, ColorNeg, 0xB71C1C, 0xFFFFFF, [0.22, 1.00, 0.08, 0.08, 0.03, 0.28], [0.006, -0.002, 0.004], [0.012, 0.0, 0.0]),
        c("Agfa", "Ultra 100", 100, ColorNeg, 0x6A1B9A, 0xFFD600, [0.45, 1.50, 0.05, 0.12, 0.0, 0.16], [0.006, -0.006, 0.010], [0.010, 0.0, 0.0]),
        c("Agfa", "Precisa CT 100", 100, Slide, 0x1E88E5, 0xFFFFFF, [0.40, 1.15, -0.08, 0.0, 0.0, 0.10], [0.0, 0.004, 0.012], [0.0, 0.0, 0.006]),
        bw("Agfa", "Scala 200x", 200, 0x212121, 0xFFFFFF, PAN, 0.70, 0.25, 0.0),
        bw("Agfa", "APX 100", 100, 0x1B5E20, 0xFFFFFF, PAN, 0.35, 0.22, 0.01),
        bw("Agfa", "APX 400", 400, 0x0D47A1, 0xFFFFFF, PAN, 0.38, 0.55, 0.01),
        // ============================================================ CINESTILL
        c("CineStill", "800T", 800, Cine, 0x212121, 0xF06292, [0.14, 1.00, -0.18, -0.04, 0.04, 0.55], [-0.008, 0.0, 0.016], [0.006, 0.0, 0.004]).hal(1.0),
        c("CineStill", "50D", 50, Cine, 0xF9A825, 0x212121, [0.16, 1.05, 0.04, 0.0, 0.03, 0.14], [0.0, 0.004, 0.008], [0.010, 0.004, 0.0]).hal(0.7),
        c("CineStill", "400D", 400, Cine, 0xEF6C00, 0x212121, [0.16, 1.05, 0.04, 0.0, 0.03, 0.36], [0.0, 0.004, 0.008], [0.010, 0.004, 0.0]).hal(0.7),
        bw("CineStill", "BwXX", 250, 0x212121, 0xFFC107, PAN, 0.40, 0.50, 0.02).hal(0.4),
        c("CineStill", "Redscale", 100, ColorNeg, 0xBF360C, 0xFFCC80, [0.30, 1.10, 0.0, 0.0, 0.03, 0.55], Z, Z)
            .mat([[1.00, 0.50, 0.35], [0.30, 0.55, 0.15], [0.00, 0.10, 0.15]]).hal(0.9),
        // ============================================================ LOMOGRAPHY
        c("Lomography", "Color Negative 100", 100, ColorNeg, 0xD32F2F, 0xFFFFFF, [0.40, 1.30, 0.10, 0.05, 0.0, 0.25], [0.0, 0.0, 0.010], [0.010, 0.0, 0.0]).vig(0.2),
        c("Lomography", "Color Negative 400", 400, ColorNeg, 0xE53935, 0xFFFFFF, [0.42, 1.32, 0.10, 0.05, 0.0, 0.50], [0.0, 0.0, 0.010], [0.010, 0.0, 0.0]).vig(0.2),
        c("Lomography", "Color Negative 800", 800, ColorNeg, 0xFB8C00, 0xFFFFFF, [0.42, 1.32, 0.12, 0.05, 0.0, 0.75], [0.0, 0.0, 0.010], [0.012, 0.0, 0.0]).vig(0.2),
        bw("Lomography", "Lady Grey 400", 400, 0x9E9E9E, 0x212121, PAN, 0.30, 0.50, 0.03),
        bw("Lomography", "Earl Grey 100", 100, 0x8D6E63, 0xFFFFFF, PAN, 0.30, 0.18, 0.03),
        bw("Lomography", "Berlin Kino 400", 400, 0x37474F, 0xFFFFFF, PAN, 0.40, 0.60, 0.04),
        c("Lomography", "LomoChrome Purple", 400, ColorNeg, 0x6A1B9A, 0xFF4081, [0.35, 1.15, 0.0, 0.0, 0.02, 0.45], Z, Z)
            .mat([[0.35, 0.65, 0.00], [0.10, -0.15, 0.95], [0.45, 0.55, 0.00]]),
        c("Lomography", "LomoChrome Turquoise", 400, ColorNeg, 0x00ACC1, 0x1DE9B6, [0.35, 1.10, 0.0, 0.0, 0.02, 0.45], Z, Z)
            .mat([[0.15, 0.55, 0.30], [0.55, 0.15, 0.30], [0.80, -0.10, 0.30]]),
        c("Lomography", "LomoChrome Metropolis", 400, ColorNeg, 0x546E7A, 0xFFC107, [0.50, 0.45, -0.05, 0.0, 0.08, 0.55], [0.0, 0.004, 0.0], [0.0, 0.0, 0.0]),
        c("Lomography", "Redscale XR", 200, ColorNeg, 0xC62828, 0xFFAB00, [0.32, 1.10, 0.0, 0.0, 0.03, 0.60], Z, Z)
            .mat([[1.00, 0.45, 0.30], [0.28, 0.55, 0.15], [0.00, 0.08, 0.15]]).vig(0.2),
        // ============================================================ ROLLEI
        bw("Rollei", "Retro 80S", 80, 0x212121, 0xFFD600, EXTRED, 0.40, 0.18, 0.0),
        bw("Rollei", "Retro 400S", 400, 0x212121, 0xFF6D00, EXTRED, 0.38, 0.48, 0.01),
        bw("Rollei", "RPX 100", 100, 0x37474F, 0xFFFFFF, PAN, 0.32, 0.16, 0.0),
        bw("Rollei", "RPX 400", 400, 0x263238, 0xFFFFFF, PAN, 0.34, 0.45, 0.01),
        bw("Rollei", "Superpan 200", 200, 0x1A1A1A, 0xFF1744, PAN, 0.42, 0.50, 0.01),
        bw("Rollei", "Infrared 400", 400, 0x7F0000, 0xFFFFFF, IR, 0.45, 0.55, 0.02).hal(0.9),
        bw("Rollei", "Ortho 25", 25, 0x4A148C, 0xFFFFFF, ORTHO, 0.50, 0.08, 0.0),
        c("Rollei", "CrossBird", 200, Slide, 0x00897B, 0xFFEB3B, [0.62, 1.40, -0.04, -0.14, 0.03, 0.30], [-0.012, 0.016, 0.012], [0.020, 0.012, -0.020]),
        c("Rollei", "Redbird", 200, ColorNeg, 0xD84315, 0xFFFFFF, [0.40, 1.30, 0.28, 0.12, 0.03, 0.40], [0.012, -0.004, 0.0], [0.024, 0.006, -0.012]),
        c("Rollei", "Digibase CN200", 200, ColorNeg, 0x1565C0, 0xFFC107, [0.26, 1.10, 0.10, 0.02, 0.03, 0.32], [0.0, 0.004, 0.006], [0.010, 0.006, 0.0]),
        // ============================================================ FOMA / ADOX / ORWO / OTHERS (B&W)
        bw("Foma", "Fomapan 100", 100, 0xE65100, 0x212121, PAN, 0.42, 0.28, 0.02),
        bw("Foma", "Fomapan 200", 200, 0xEF6C00, 0x212121, PAN, 0.42, 0.45, 0.02),
        bw("Foma", "Fomapan 400", 400, 0xF57C00, 0x212121, PAN, 0.44, 0.65, 0.02),
        bw("Foma", "Fomapan R100", 100, 0x8D6E63, 0xFFFFFF, PAN, 0.55, 0.20, 0.0),
        bw("Adox", "CHS 100 II", 100, 0xFBC02D, 0x212121, PAN, 0.34, 0.12, 0.0),
        bw("Adox", "Silvermax", 100, 0x9E9E9E, 0x212121, PAN, 0.50, 0.14, 0.0),
        bw("Adox", "HR-50", 50, 0x757575, 0xFFFFFF, PAN, 0.42, 0.06, 0.0),
        c("Adox", "Color Implosion", 100, ColorNeg, 0xAD1457, 0x00E5FF, [0.45, 1.25, -0.05, 0.16, 0.04, 0.55], [-0.010, 0.0, 0.020], [0.020, -0.006, 0.014]),
        bw("Orwo", "N75", 50, 0x546E7A, 0xFFEB3B, PAN, 0.38, 0.40, 0.02),
        bw("Orwo", "UN54", 100, 0x455A64, 0xFFFFFF, PAN, 0.40, 0.35, 0.02),
        bw("Shanghai", "GP3 100", 100, 0xC62828, 0xFFD600, PAN, 0.40, 0.30, 0.02),
        bw("Lucky", "SHD 100", 100, 0xE53935, 0xFFFFFF, PAN, 0.40, 0.32, 0.02),
        bw("Bergger", "Pancro 400", 400, 0x1B1B1B, 0xFFFFFF, PAN, 0.40, 0.50, 0.01),
        bw("Ferrania", "P30", 80, 0xFFFFFF, 0xD32F2F, PAN, 0.65, 0.35, 0.0),
        c("Ferrania", "Solaris FG 200", 200, ColorNeg, 0xFFF176, 0x388E3C, [0.30, 1.05, 0.12, 0.05, 0.04, 0.50], [0.0, 0.004, 0.004], [0.016, 0.008, -0.006]),
        // ============================================================ KONICA / SVEMA / OTHER COLOUR
        c("Konica", "Centuria 400", 400, ColorNeg, 0x1E88E5, 0xFFFFFF, [0.30, 1.10, 0.0, -0.02, 0.03, 0.42], [-0.004, 0.006, 0.010], [0.004, 0.006, 0.0]),
        c("Konica", "VX 100", 100, ColorNeg, 0x7CB342, 0xFFFFFF, [0.28, 1.10, 0.04, -0.04, 0.03, 0.20], [-0.004, 0.008, 0.006], [0.004, 0.006, 0.0]),
        c("Konica", "Impressa 50", 50, Slide, 0x3949AB, 0xFFFFFF, [0.45, 1.20, -0.04, 0.0, 0.0, 0.08], [0.0, 0.004, 0.012], [0.0, 0.0, 0.004]),
        c("Svema", "Color 125", 125, ColorNeg, 0xB71C1C, 0xFFFFFF, [0.30, 0.90, 0.20, 0.12, 0.08, 0.60], [0.012, 0.0, -0.004], [0.026, 0.010, -0.016]),
        c("Svema", "FN-64", 64, BlackWhite, 0x6D4C41, 0xFFFFFF, [0.40, 1.0, 0.0, 0.0, 0.03, 0.35], Z, Z)
            .tone(WARM_SH, WARM_HI)
            .mat([[0.30, 0.59, 0.11], [0.30, 0.59, 0.11], [0.30, 0.59, 0.11]]),
        // ============================================================ POLAROID – instant
        c("Polaroid", "Color 600", 640, Instant, 0xF8F8F8, 0xFF5252, [0.22, 1.02, 0.08, 0.02, 0.07, 0.25], [0.004, 0.006, 0.010], [0.018, 0.010, -0.004]).vig(0.30),
        c("Polaroid", "SX-70 Color", 160, Instant, 0xF2F2F2, 0xFFA000, [0.24, 1.05, 0.10, 0.03, 0.07, 0.20], [0.004, 0.006, 0.008], [0.020, 0.010, -0.006]).vig(0.28),
        c("Polaroid", "i-Type Color", 640, Instant, 0xFFFFFF, 0x29B6F6, [0.30, 1.10, 0.04, 0.0, 0.05, 0.18], [0.0, 0.004, 0.008], [0.010, 0.006, 0.0]).vig(0.22),
        c("Polaroid", "Spectra Color", 640, Instant, 0xEEEEEE, 0x7E57C2, [0.26, 1.06, 0.06, 0.02, 0.06, 0.20], [0.0, 0.006, 0.010], [0.014, 0.008, -0.002]).vig(0.22),
        c("Polaroid", "669 Peel-Apart", 80, Instant, 0xD7CCC8, 0x5D4037, [0.20, 0.95, 0.12, 0.04, 0.09, 0.15], [0.006, 0.008, 0.004], [0.020, 0.012, -0.010]).vig(0.22),
        bw("Polaroid", "B&W 600", 640, 0xF5F5F5, 0x212121, SOFT, 0.28, 0.25, 0.07).vig(0.28),
        bw("Polaroid", "667 Peel-Apart B&W", 3000, 0xBDBDBD, 0x212121, PAN, 0.45, 0.35, 0.05).vig(0.2),
    ]
}