//! Image engine: applies a film "look" plus a handful of user adjustments.
//! Works on 8-bit sRGB (as produced by the RAW developer) and is parallelised with rayon.

use rayon::prelude::*;

#[derive(Clone)]
pub struct Image8 {
    pub w: usize,
    pub h: usize,
    pub data: Vec<u8>, // RGB
}

/// Describes how a film stock renders an image.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Look {
    pub contrast: f32,   // S-curve strength (-0.5..1)
    pub saturation: f32, // multiplier
    pub warmth: f32,     // -1 cool .. +1 warm
    pub tint: f32,       // -1 green .. +1 magenta
    pub fade: f32,       // black-point lift
    pub grain: f32,      // 0..1
    pub rolloff: f32,    // highlight shoulder softness 0..1
    pub halation: f32,   // red glow around highlights 0..1
    pub vignette: f32,   // built-in vignette (instant films)
    pub shadow_tone: [f32; 3],
    pub highlight_tone: [f32; 3],
    pub bw: Option<[f32; 3]>,         // black & white channel mixer
    pub matrix: Option<[[f32; 3]; 3]>, // colour matrix (false colour / redscale / purple…)
}

/// User sliders.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Adjust {
    pub exposure: f32,    // stops
    pub contrast: f32,    // -1..1
    pub highlights: f32,  // -1..1
    pub shadows: f32,     // -1..1
    pub temperature: f32, // -1..1
    pub tint: f32,        // -1..1
    pub saturation: f32,  // -1..1
    pub strength: f32,    // 0..1 film strength
    pub grain: f32,       // 0..2 multiplier
    pub glow: f32,        // 0..2 halation multiplier
    pub vignette: f32,    // -1..1
}

impl Default for Adjust {
    fn default() -> Self {
        Adjust {
            exposure: 0.0,
            contrast: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            temperature: 0.0,
            tint: 0.0,
            saturation: 0.0,
            strength: 1.0,
            grain: 1.0,
            glow: 1.0,
            vignette: 0.0,
        }
    }
}

// ---------------------------------------------------------------- helpers

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

#[inline]
fn srgb_enc(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

fn srgb_lut() -> [f32; 256] {
    let mut l = [0.0; 256];
    for (i, v) in l.iter_mut().enumerate() {
        let x = i as f32 / 255.0;
        *v = if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) };
    }
    l
}

#[inline]
fn scurve(x: f32, k: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let s = x * x * (3.0 - 2.0 * x);
    lerp(x, s, k.clamp(-0.8, 1.2))
}

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d)
        ^ (y as u32).wrapping_mul(0x1656_67b1)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    (h as f32 / 4_294_967_295.0) * 2.0 - 1.0
}

fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    let xf = x.floor();
    let yf = y.floor();
    let fx = x - xf;
    let fy = y - yf;
    let (xi, yi) = (xf as i32, yf as i32);
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let a = hash(xi, yi, seed);
    let b = hash(xi + 1, yi, seed);
    let c = hash(xi, yi + 1, seed);
    let d = hash(xi + 1, yi + 1, seed);
    lerp(lerp(a, b, sx), lerp(c, d, sx), sy)
}

// ---------------------------------------------------------------- halation

struct Halo {
    w: usize,
    h: usize,
    data: Vec<f32>,
}

fn box_blur(buf: &mut [f32], w: usize, h: usize, r: usize) {
    let mut tmp = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let (x0, x1) = (x.saturating_sub(r), (x + r).min(w - 1));
            let mut s = 0.0;
            for xx in x0..=x1 {
                s += buf[y * w + xx];
            }
            tmp[y * w + x] = s / (x1 - x0 + 1) as f32;
        }
    }
    for y in 0..h {
        let (y0, y1) = (y.saturating_sub(r), (y + r).min(h - 1));
        for x in 0..w {
            let mut s = 0.0;
            for yy in y0..=y1 {
                s += tmp[yy * w + x];
            }
            buf[y * w + x] = s / (y1 - y0 + 1) as f32;
        }
    }
}

fn build_halo(src: &Image8, lut: &[f32; 256]) -> Halo {
    let f = (src.w.max(src.h) / 400).max(1);
    let lw = (src.w + f - 1) / f;
    let lh = (src.h + f - 1) / f;
    let mut data = vec![0f32; lw * lh];
    data.par_chunks_mut(lw).enumerate().for_each(|(ly, row)| {
        for (lx, v) in row.iter_mut().enumerate() {
            let x = (lx * f + f / 2).min(src.w - 1);
            let y = (ly * f + f / 2).min(src.h - 1);
            let i = (y * src.w + x) * 3;
            let l = luma([
                lut[src.data[i] as usize],
                lut[src.data[i + 1] as usize],
                lut[src.data[i + 2] as usize],
            ]);
            let m = ((srgb_enc(l) - 0.62) / 0.38).clamp(0.0, 1.0);
            *v = m * m;
        }
    });
    for _ in 0..3 {
        box_blur(&mut data, lw, lh, (lw / 40).max(2));
    }
    Halo { w: lw, h: lh, data }
}

fn sample_halo(hm: &Halo, u: f32, v: f32) -> f32 {
    let fx = (u * hm.w as f32 - 0.5).clamp(0.0, (hm.w - 1) as f32);
    let fy = (v * hm.h as f32 - 0.5).clamp(0.0, (hm.h - 1) as f32);
    let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(hm.w - 1), (y0 + 1).min(hm.h - 1));
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    let g = |x: usize, y: usize| hm.data[y * hm.w + x];
    lerp(lerp(g(x0, y0), g(x1, y0), tx), lerp(g(x0, y1), g(x1, y1), tx), ty)
}

// ---------------------------------------------------------------- render

pub fn render(src: &Image8, look: &Look, adj: &Adjust) -> Vec<u8> {
    let (w, h) = (src.w, src.h);
    let lut = srgb_lut();
    let st = adj.strength.clamp(0.0, 1.0);

    let hal = look.halation * adj.glow * st;
    let halo = if hal > 0.001 { Some(build_halo(src, &lut)) } else { None };

    let ex = 2f32.powf(adj.exposure);
    let gain = [
        ex * (1.0 + 0.25 * adj.temperature),
        ex * (1.0 - 0.15 * adj.tint),
        ex * (1.0 - 0.25 * adj.temperature),
    ];

    let gs = (w.max(h) as f32 / 1600.0).max(0.5) * 1.5; // grain cell size in pixels
    let grain_amt = look.grain * adj.grain * st;
    let vig = adj.vignette + look.vignette * st;
    let (cx, cy) = (w as f32 * 0.5, h as f32 * 0.5);
    let inv_r2 = 1.0 / (cx * cx + cy * cy);
    let is_bw = look.bw.is_some();

    let mut out = vec![0u8; w * h * 3];
    out.par_chunks_mut(w * 3).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let i = (y * w + x) * 3;
            let mut c = [
                lut[src.data[i] as usize] * gain[0],
                lut[src.data[i + 1] as usize] * gain[1],
                lut[src.data[i + 2] as usize] * gain[2],
            ];

            // red halation around bright areas (light bouncing off the film base)
            if let Some(hm) = &halo {
                let hv = sample_halo(hm, (x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32) * hal * 0.55;
                c[0] += hv;
                c[1] += hv * 0.28;
                c[2] += hv * 0.10;
            }

            let mut g = [srgb_enc(c[0]), srgb_enc(c[1]), srgb_enc(c[2])];
            let base = g;

            // ---------- film path
            let mut f = g;
            if let Some(m) = &look.matrix {
                f = [
                    dot(m[0], f).clamp(0.0, 1.0),
                    dot(m[1], f).clamp(0.0, 1.0),
                    dot(m[2], f).clamp(0.0, 1.0),
                ];
            }
            if let Some(mix) = look.bw {
                let l = dot(mix, f).clamp(0.0, 1.0);
                f = [l, l, l];
            } else {
                f[0] *= 1.0 + 0.10 * look.warmth + 0.04 * look.tint;
                f[1] *= 1.0 - 0.08 * look.tint;
                f[2] *= 1.0 - 0.10 * look.warmth + 0.04 * look.tint;
            }
            for v in f.iter_mut() {
                let mut t = scurve(*v, look.contrast);
                let sh = ((t - 0.6) / 0.4).clamp(0.0, 1.0);
                t -= look.rolloff * 0.12 * sh * sh;
                *v = look.fade + t * (1.0 - look.fade);
            }
            let l = luma(f);
            if !is_bw {
                for v in f.iter_mut() {
                    *v = l + (*v - l) * look.saturation;
                }
            }
            let (sw, hw) = ((1.0 - l) * (1.0 - l), l * l);
            for k in 0..3 {
                f[k] += look.shadow_tone[k] * sw + look.highlight_tone[k] * hw;
            }
            for k in 0..3 {
                g[k] = lerp(base[k], f[k], st);
            }

            // ---------- user adjustments
            if adj.contrast != 0.0 {
                for v in g.iter_mut() {
                    *v = scurve(*v, adj.contrast * 0.8);
                }
            }
            let l = luma(g);
            let sh = adj.shadows * 0.30 * (l * (1.0 - l) * (1.0 - l) * 3.0);
            let hl = adj.highlights * 0.30 * (l * l * (1.0 - l) * 3.0);
            let sat = 1.0 + adj.saturation;
            for v in g.iter_mut() {
                *v = l + (*v - l) * sat + sh + hl;
            }

            // ---------- vignette
            if vig != 0.0 {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                let r2 = (dx * dx + dy * dy) * inv_r2;
                let vf = 1.0 - vig * 0.8 * r2.powf(1.3);
                for v in g.iter_mut() {
                    *v *= vf;
                }
            }

            // ---------- grain
            if grain_amt > 0.001 {
                let l = luma(g).clamp(0.0, 1.0);
                let amp = grain_amt * 0.11 * (0.35 + 0.65 * 4.0 * l * (1.0 - l));
                let (u, v) = (x as f32 / gs, y as f32 / gs);
                let n = vnoise(u, v, 1) * 1.7;
                if is_bw {
                    for k in g.iter_mut() {
                        *k += n * amp;
                    }
                } else {
                    let n2 = vnoise(u, v, 2) * 1.7;
                    g[0] += n * amp + n2 * amp * 0.3;
                    g[1] += n * amp;
                    g[2] += n * amp - n2 * amp * 0.3;
                }
            }

            for k in 0..3 {
                row[x * 3 + k] = (g[k].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
    });
    out
}

// ---------------------------------------------------------------- geometry

pub fn rotated(img: &Image8, rot: u8) -> Image8 {
    if rot % 4 == 0 {
        return img.clone();
    }
    let buf = image::RgbImage::from_raw(img.w as u32, img.h as u32, img.data.clone()).expect("size");
    let r = match rot % 4 {
        1 => image::imageops::rotate90(&buf),
        2 => image::imageops::rotate180(&buf),
        _ => image::imageops::rotate270(&buf),
    };
    let (w, h) = r.dimensions();
    Image8 { w: w as usize, h: h as usize, data: r.into_raw() }
}

pub fn downscale(img: Image8, max_dim: usize) -> Image8 {
    let m = img.w.max(img.h);
    if m <= max_dim {
        return img;
    }
    let s = max_dim as f32 / m as f32;
    let (nw, nh) = (((img.w as f32 * s) as u32).max(1), ((img.h as f32 * s) as u32).max(1));
    let buf = image::RgbImage::from_raw(img.w as u32, img.h as u32, img.data).expect("size");
    let r = image::imageops::resize(&buf, nw, nh, image::imageops::FilterType::Triangle);
    Image8 { w: nw as usize, h: nh as usize, data: r.into_raw() }
}
