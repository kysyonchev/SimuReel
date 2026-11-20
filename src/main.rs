mod canister;
mod engine;
mod films;
mod native_menu;

use eframe::egui::{self, pos2, vec2, Align, Color32, FontId, Key, KeyboardShortcut, Layout, Modifiers, Rect};
use engine::{Adjust, Image8};
use films::{Film, Kind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

const ACCENT: Color32 = Color32::from_rgb(255, 160, 50);
const PREVIEW_MAX: usize = 1800;
const RAW_EXT: &[&str] = &[
    "cr3", "crw", "dng", "cr2", "nef", "nrw", "arw", "srf", "sr2", "orf", "raf", "rw2", "pef", "srw", "erf", "kdc",
    "dcr", "mef", "mos", "raw", "3fr", "iiq", "rwl", "fff", "gpr",
];

enum Msg {
    Loaded(Image8, PathBuf),
    Done(String),
    Err(String),
}

// ------------------------------------------------------------------ decoding

fn ext_of(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

fn is_raw(p: &Path) -> bool {
    RAW_EXT.contains(&ext_of(p).as_str())
}

fn decode_native_raw(path: &Path) -> Result<Image8, String> {
    let p = path.to_str().ok_or("Invalid path")?;
    let mut pipe = imagepipe::Pipeline::new_from_file(p).map_err(|e| format!("RAW decode failed: {e}"))?;
    let img = pipe.output_8bit(None).map_err(|e| format!("RAW develop failed: {e}"))?;
    Ok(Image8 { w: img.width, h: img.height, data: img.data })
}

/// macOS ships a RAW decoder (used by Photos / Preview) that understands CR3 and many newer cameras.
/// We reach it through the built-in `sips` tool.
#[cfg(target_os = "macos")]
fn decode_with_system(path: &Path) -> Result<Image8, String> {
    use std::process::Command;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!("filmlab_{}_{}.png", std::process::id(), nanos));
    let profile = "/System/Library/ColorSync/Profiles/sRGB Profile.icc";

    let mut last_err = String::new();
    for with_profile in [true, false] {
        let mut cmd = Command::new("sips");
        cmd.args(["-s", "format", "png"]);
        if with_profile {
            cmd.args(["-m", profile]);
        }
        cmd.arg(path).arg("--out").arg(&tmp);
        match cmd.output() {
            Ok(o) if o.status.success() && tmp.exists() => {
                let img = image::open(&tmp).map_err(|e| e.to_string());
                let _ = std::fs::remove_file(&tmp);
                let img = img?.to_rgb8();
                let (w, h) = img.dimensions();
                return Ok(Image8 { w: w as usize, h: h as usize, data: img.into_raw() });
            }
            Ok(o) => last_err = String::from_utf8_lossy(&o.stderr).trim().to_string(),
            Err(e) => last_err = e.to_string(),
        }
    }
    Err(format!(
        "macOS could not develop this RAW file ({last_err}). If the camera is very new, update macOS or convert to DNG."
    ))
}

#[cfg(not(target_os = "macos"))]
fn decode_with_system(_path: &Path) -> Result<Image8, String> {
    Err("This RAW format (e.g. CR3) is only supported through macOS's built-in decoder. Convert it to DNG first.".into())
}

fn decode(path: &Path) -> Result<Image8, String> {
    if is_raw(path) {
        let ext = ext_of(path);
        if ext != "cr3" && ext != "crw" {
            if let Ok(img) = decode_native_raw(path) {
                return Ok(img);
            }
        }
        decode_with_system(path)
    } else {
        let img = image::open(path).map_err(|e| e.to_string())?.to_rgb8();
        let (w, h) = img.dimensions();
        Ok(Image8 { w: w as usize, h: h as usize, data: img.into_raw() })
    }
}

fn save(dest: &Path, w: usize, h: usize, data: &[u8]) -> Result<(), String> {
    let ext = ext_of(dest);
    if ext == "jpg" || ext == "jpeg" {
        let f = std::fs::File::create(dest).map_err(|e| e.to_string())?;
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(f), 95);
        enc.encode(data, w as u32, h as u32, image::ExtendedColorType::Rgb8)
            .map_err(|e| e.to_string())
    } else {
        image::save_buffer(dest, data, w as u32, h as u32, image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())
    }
}

// ------------------------------------------------------------------ app

#[derive(Default)]
struct Actions {
    open: bool,
    export: bool,
    rotate: i8,
    reset: bool,
    step: i32,
    random: bool,
    pick: Option<usize>,
    scroll: bool,
    quit: bool,
}

struct App {
    films: Vec<Film>,
    brands: Vec<String>,
    visible: Vec<usize>,
    selected: usize,
    adj: Adjust,
    search: String,
    brand: String,
    kind: Option<Kind>,
    base: Option<Image8>,
    src: Option<Image8>,
    path: Option<PathBuf>,
    rot: u8,
    tex_before: Option<egui::TextureHandle>,
    tex_after: Option<egui::TextureHandle>,
    dirty: bool,
    show_original: bool,
    show_browser: bool,
    show_adjust: bool,
    show_help: bool,
    scroll_to_selected: bool,
    native: Option<native_menu::NativeMenu>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    busy: bool,
    status: String,
}

fn setup_style(ctx: &egui::Context) {
    ctx.style_mut(|s| {
        let v = &mut s.visuals;
        *v = egui::Visuals::dark();
        v.panel_fill = Color32::from_rgb(26, 26, 29);
        v.window_fill = Color32::from_rgb(30, 30, 34);
        v.extreme_bg_color = Color32::from_rgb(18, 18, 20);
        v.selection.bg_fill = ACCENT.linear_multiply(0.85);
        v.selection.stroke = egui::Stroke::new(1.0, Color32::BLACK);
        v.hyperlink_color = ACCENT;
        v.widgets.inactive.rounding = 6.0.into();
        v.widgets.hovered.rounding = 6.0.into();
        v.widgets.active.rounding = 6.0.into();
        v.widgets.noninteractive.rounding = 6.0.into();
        v.widgets.inactive.bg_fill = Color32::from_rgb(48, 48, 54);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(48, 48, 54);
        v.widgets.hovered.bg_fill = Color32::from_rgb(66, 66, 74);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(66, 66, 74);
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.button_padding = vec2(10.0, 4.0);
        s.spacing.menu_margin = egui::Margin::same(6.0);
    });
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_style(&cc.egui_ctx);
        let films = films::all();
        let mut brands: Vec<String> = films.iter().map(|f| f.brand.to_string()).collect();
        brands.sort();
        brands.dedup();
        brands.insert(0, "All brands".to_string());
        let native = native_menu::NativeMenu::new(&cc.egui_ctx, &films);
        let (tx, rx) = channel();
        let selected = films.iter().position(|f| f.name == "Portra 400").unwrap_or(0);
        App {
            films,
            brands,
            visible: vec![],
            selected,
            adj: Adjust::default(),
            search: String::new(),
            brand: "All brands".into(),
            kind: None,
            base: None,
            src: None,
            path: None,
            rot: 0,
            tex_before: None,
            tex_after: None,
            dirty: false,
            show_original: false,
            show_browser: true,
            show_adjust: true,
            show_help: false,
            scroll_to_selected: false,
            native,
            tx,
            rx,
            busy: false,
            status: "Open or drop a RAW / JPEG / PNG / TIFF file to begin.".into(),
        }
    }

    fn filtered(&self) -> Vec<usize> {
        let q = self.search.to_lowercase();
        self.films
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                (self.brand == "All brands" || f.brand == self.brand)
                    && self.kind.map_or(true, |k| k == f.kind)
                    && (q.is_empty()
                        || f.name.to_lowercase().contains(&q)
                        || f.brand.to_lowercase().contains(&q)
                        || f.iso.to_string().contains(&q))
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn open_dialog(&mut self, ctx: &egui::Context) {
        let mut exts: Vec<&str> = RAW_EXT.to_vec();
        exts.extend(["jpg", "jpeg", "png", "tif", "tiff"]);
        if let Some(p) = rfd::FileDialog::new().add_filter("Images (RAW, JPEG, PNG, TIFF)", &exts).pick_file() {
            self.load(ctx, p);
        }
    }

    fn load(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.busy = true;
        self.status = format!("Developing {} …", path.file_name().and_then(|n| n.to_str()).unwrap_or("image"));
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let msg = match decode(&path) {
                Ok(img) => Msg::Loaded(engine::downscale(img, PREVIEW_MAX), path),
                Err(e) => Msg::Err(e),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    fn rebuild_src(&mut self, ctx: &egui::Context) {
        if let Some(b) = &self.base {
            let s = engine::rotated(b, self.rot);
            let img = egui::ColorImage::from_rgb([s.w, s.h], &s.data);
            self.tex_before = Some(ctx.load_texture("before", img, egui::TextureOptions::LINEAR));
            self.src = Some(s);
            self.dirty = true;
        }
    }

    fn render(&mut self, ctx: &egui::Context) {
        let Some(src) = &self.src else { return };
        let film = &self.films[self.selected];
        let out = engine::render(src, &film.look, &self.adj);
        let img = egui::ColorImage::from_rgb([src.w, src.h], &out);
        match &mut self.tex_after {
            Some(t) => t.set(img, egui::TextureOptions::LINEAR),
            None => self.tex_after = Some(ctx.load_texture("after", img, egui::TextureOptions::LINEAR)),
        }
    }

    fn export(&mut self, ctx: &egui::Context) {
        let Some(path) = self.path.clone() else { return };
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("photo");
        let film = &self.films[self.selected];
        let suggested = format!("{}_{}.jpg", stem, film.name.replace(' ', "_"));
        let Some(dest) = rfd::FileDialog::new()
            .set_file_name(suggested)
            .add_filter("JPEG", &["jpg", "jpeg"])
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        let look = film.look;
        let adj = self.adj;
        let rot = self.rot;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        self.busy = true;
        self.status = "Exporting full resolution …".into();
        thread::spawn(move || {
            let r = (|| -> Result<String, String> {
                let img = engine::rotated(&decode(&path)?, rot);
                let out = engine::render(&img, &look, &adj);
                save(&dest, img.w, img.h, &out)?;
                Ok(format!("Saved {}", dest.display()))
            })();
            let _ = tx.send(match r {
                Ok(s) => Msg::Done(s),
                Err(e) => Msg::Err(e),
            });
            ctx.request_repaint();
        });
    }

    fn apply(&mut self, ctx: &egui::Context, act: Actions) {
        if act.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if act.open {
            self.open_dialog(ctx);
        }
        if act.export && self.src.is_some() && !self.busy {
            self.export(ctx);
        }
        if act.rotate != 0 && self.src.is_some() {
            self.rot = (self.rot as i8 + act.rotate).rem_euclid(4) as u8;
            self.rebuild_src(ctx);
        }
        if act.reset {
            self.adj = Adjust::default();
            self.dirty = true;
        }
        if act.step != 0 && !self.visible.is_empty() {
            let n = self.visible.len() as i32;
            let np = match self.visible.iter().position(|&i| i == self.selected) {
                Some(p) => (p as i32 + act.step).rem_euclid(n),
                None => 0,
            };
            self.selected = self.visible[np as usize];
            self.dirty = true;
            self.scroll_to_selected = true;
        }
        if act.random {
            let n = self.films.len().max(1);
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0) as usize;
            self.selected = (t / 7) % n;
            self.dirty = true;
            self.scroll_to_selected = true;
        }
        if let Some(i) = act.pick {
            self.selected = i;
            self.dirty = true;
            self.scroll_to_selected = act.scroll;
        }
    }
}

fn menu_item(ui: &mut egui::Ui, ctx: &egui::Context, label: &str, sc: Option<&KeyboardShortcut>) -> bool {
    let mut b = egui::Button::new(label);
    if let Some(s) = sc {
        b = b.shortcut_text(ctx.format_shortcut(s));
    }
    if ui.add(b).clicked() {
        ui.close_menu();
        true
    } else {
        false
    }
}

fn slider(ui: &mut egui::Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, def: f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(vec2(96.0, 20.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(label);
        });
        ui.spacing_mut().slider_width = 110.0;
        let resp = ui.add(egui::Slider::new(&mut *v, range).fixed_decimals(2));
        if resp.changed() {
            changed = true;
        }
        if resp.double_clicked() {
            *v = def;
            changed = true;
        }
    });
    changed
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ---- background results
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Loaded(img, p) => {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("image").to_string();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("SimuReel — {name}")));
                    self.base = Some(img);
                    self.path = Some(p);
                    self.rot = 0;
                    self.busy = false;
                    self.status = format!("{name} loaded. Hold Space to compare, double-click a slider to reset it.");
                    self.rebuild_src(ctx);
                }
                Msg::Done(s) | Msg::Err(s) => {
                    self.busy = false;
                    self.status = s;
                }
            }
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(p) = dropped.into_iter().find_map(|f| f.path) {
            self.load(ctx, p);
        }

        self.visible = self.filtered();

        // ---- shortcuts
        let sc_open = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
        let sc_export = KeyboardShortcut::new(Modifiers::COMMAND, Key::E);
        let sc_rr = KeyboardShortcut::new(Modifiers::COMMAND, Key::R);
        let sc_rl = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::R);
        let sc_reset = KeyboardShortcut::new(Modifiers::COMMAND, Key::Num0);
        let sc_next = KeyboardShortcut::new(Modifiers::NONE, Key::ArrowRight);
        let sc_prev = KeyboardShortcut::new(Modifiers::NONE, Key::ArrowLeft);

        let mut act = Actions::default();
        let native = self.native.is_some();
        let cmds = self.native.as_ref().map(|n| n.poll()).unwrap_or_default();
        for c in cmds {
            use native_menu::Cmd;
            match c {
                Cmd::Open => act.open = true,
                Cmd::Export => act.export = true,
                Cmd::RotateRight => act.rotate = 1,
                Cmd::RotateLeft => act.rotate = -1,
                Cmd::Reset => act.reset = true,
                Cmd::ToggleOriginal => self.show_original = !self.show_original,
                Cmd::ToggleBrowser => self.show_browser = !self.show_browser,
                Cmd::ToggleAdjust => self.show_adjust = !self.show_adjust,
                Cmd::Next => act.step = 1,
                Cmd::Prev => act.step = -1,
                Cmd::Random => act.random = true,
                Cmd::Pick(i) => {
                    act.pick = Some(i);
                    act.scroll = true;
                }
                Cmd::Shortcuts => self.show_help = true,
            }
        }
        let typing = ctx.wants_keyboard_input();
        ctx.input_mut(|i| {
            if !native && i.consume_shortcut(&sc_open) {
                act.open = true;
            }
            if !native && i.consume_shortcut(&sc_export) {
                act.export = true;
            }
            if !native && i.consume_shortcut(&sc_rl) {
                act.rotate = -1;
            } else if !native && i.consume_shortcut(&sc_rr) {
                act.rotate = 1;
            }
            if !native && i.consume_shortcut(&sc_reset) {
                act.reset = true;
            }
            if !typing {
                if i.consume_shortcut(&sc_next) {
                    act.step = 1;
                }
                if i.consume_shortcut(&sc_prev) {
                    act.step = -1;
                }
            }
        });

        let has = self.src.is_some();
        if let Some(n) = &mut self.native {
            n.sync(has, self.busy, self.show_original, self.show_browser, self.show_adjust);
        }

        // ---- (menu bar +) toolbar. With the native macOS menu the toolbar sits in the title bar row.
        let use_egui_menu = !native;
        let mac_titlebar = cfg!(target_os = "macos");
        let tb_h = 44.0;
        let pad = if mac_titlebar { 74.0 } else { 0.0 }; // room for the traffic-light buttons
        egui::TopBottomPanel::top("top")
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(30, 30, 34))
                    .inner_margin(egui::Margin::symmetric(12.0, 0.0)),
            )
            .show(ctx, |ui| {
            if use_egui_menu {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if menu_item(ui, ctx, "Open…", Some(&sc_open)) {
                        act.open = true;
                    }
                    ui.add_enabled_ui(has && !self.busy, |ui| {
                        if menu_item(ui, ctx, "Export…", Some(&sc_export)) {
                            act.export = true;
                        }
                    });
                    ui.separator();
                    if menu_item(ui, ctx, "Quit", None) {
                        act.quit = true;
                    }
                });
                ui.menu_button("Edit", |ui| {
                    ui.add_enabled_ui(has, |ui| {
                        if menu_item(ui, ctx, "Rotate right", Some(&sc_rr)) {
                            act.rotate = 1;
                        }
                        if menu_item(ui, ctx, "Rotate left", Some(&sc_rl)) {
                            act.rotate = -1;
                        }
                    });
                    ui.separator();
                    if menu_item(ui, ctx, "Reset adjustments", Some(&sc_reset)) {
                        act.reset = true;
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_original, "Show original (or hold Space)");
                    ui.separator();
                    ui.checkbox(&mut self.show_browser, "Film browser");
                    ui.checkbox(&mut self.show_adjust, "Adjustments panel");
                });
                ui.menu_button("Film", |ui| {
                    if menu_item(ui, ctx, "Next film", Some(&sc_next)) {
                        act.step = 1;
                    }
                    if menu_item(ui, ctx, "Previous film", Some(&sc_prev)) {
                        act.step = -1;
                    }
                    if menu_item(ui, ctx, "Random film", None) {
                        act.random = true;
                    }
                    ui.separator();
                    ui.menu_button("By brand", |ui| {
                        egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
                            for b in self.brands.iter().skip(1) {
                                ui.menu_button(b.as_str(), |ui| {
                                    egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                                        for (i, f) in self.films.iter().enumerate().filter(|(_, f)| f.brand == b.as_str()) {
                                            if ui.button(format!("{}   ISO {}", f.name, f.iso)).clicked() {
                                                act.pick = Some(i);
                                                act.scroll = true;
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                });
                            }
                        });
                    });
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard shortcuts").clicked() {
                        self.show_help = true;
                        ui.close_menu();
                    }
                });
            });
            }
            let tb_rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), tb_h));
            let drag = ui.interact(tb_rect, ui.id().with("titlebar_drag"), egui::Sense::click_and_drag());
            ui.allocate_ui_with_layout(vec2(ui.available_width(), tb_h), Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(pad);
                if ui.button("Open").clicked() {
                    act.open = true;
                }
                if ui.add_enabled(has && !self.busy, egui::Button::new("Export")).clicked() {
                    act.export = true;
                }
                ui.separator();
                if ui.add_enabled(has, egui::Button::new("Rotate left")).clicked() {
                    act.rotate = -1;
                }
                if ui.add_enabled(has, egui::Button::new("Rotate right")).clicked() {
                    act.rotate = 1;
                }
                ui.separator();
                ui.toggle_value(&mut self.show_original, "Before / After");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.busy {
                        ui.spinner();
                    }
                    let f = &self.films[self.selected];
                    ui.label(egui::RichText::new(format!("{} {}", f.brand, f.name)).strong().color(ACCENT));
                });
            });
            if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            }
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&self.status).weak());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(s) = &self.src {
                        ui.label(egui::RichText::new(format!("preview {}×{}", s.w, s.h)).weak());
                    }
                });
            });
        });

        // ---- film browser
        if self.show_browser {
            egui::SidePanel::left("films").default_width(360.0).show(ctx, |ui| {
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search films, brands, ISO…")
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.kind, None, "All");
                    for k in Kind::ALL {
                        ui.selectable_value(&mut self.kind, Some(k), k.short());
                    }
                });
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("brand_combo")
                        .selected_text(self.brand.clone())
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for b in &self.brands {
                                ui.selectable_value(&mut self.brand, b.clone(), b.as_str());
                            }
                        });
                    if ui.small_button("Clear").clicked() {
                        self.search.clear();
                        self.brand = "All brands".into();
                        self.kind = None;
                    }
                    ui.label(egui::RichText::new(format!("{} films", self.visible.len())).weak());
                });
                ui.separator();

                let mut clicked = None;
                let scroll = self.scroll_to_selected;
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        for &i in &self.visible {
                            let f = &self.films[i];
                            let (rect, resp) = ui.allocate_exact_size(vec2(104.0, 150.0), egui::Sense::click());
                            canister::draw(&ui.painter_at(rect), rect, f, i == self.selected, resp.hovered());
                            if scroll && i == self.selected {
                                resp.scroll_to_me(Some(Align::Center));
                            }
                            if resp.clicked() {
                                clicked = Some(i);
                            }
                            resp.on_hover_text(format!("{} {} — ISO {} · {}", f.brand, f.name, f.iso, f.kind.label()));
                        }
                    });
                });
                self.scroll_to_selected = false;
                if let Some(i) = clicked {
                    self.selected = i;
                    self.dirty = true;
                }
            });
        }

        // ---- adjustments
        if self.show_adjust {
            egui::SidePanel::right("adjust").default_width(310.0).show(ctx, |ui| {
                {
                    let f = &self.films[self.selected];
                    ui.add_space(4.0);
                    let w = ui.available_width().min(190.0);
                    let (rect, _) = ui.allocate_exact_size(vec2(w, 190.0), egui::Sense::hover());
                    canister::draw(&ui.painter_at(rect), rect, f, false, false);
                    ui.label(egui::RichText::new(format!("{} {}", f.brand, f.name)).strong().size(16.0));
                    ui.label(egui::RichText::new(format!("ISO {} · {}", f.iso, f.kind.label())).weak());
                }
                ui.separator();
                let mut ch = false;
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    egui::CollapsingHeader::new("Film").default_open(true).show(ui, |ui| {
                        ch |= slider(ui, "Strength", &mut self.adj.strength, 0.0..=1.0, 1.0);
                        ch |= slider(ui, "Grain", &mut self.adj.grain, 0.0..=2.0, 1.0);
                        ch |= slider(ui, "Glow", &mut self.adj.glow, 0.0..=2.0, 1.0);
                        ch |= slider(ui, "Vignette", &mut self.adj.vignette, -1.0..=1.0, 0.0);
                    });
                    egui::CollapsingHeader::new("Light").default_open(true).show(ui, |ui| {
                        ch |= slider(ui, "Exposure", &mut self.adj.exposure, -3.0..=3.0, 0.0);
                        ch |= slider(ui, "Contrast", &mut self.adj.contrast, -1.0..=1.0, 0.0);
                        ch |= slider(ui, "Highlights", &mut self.adj.highlights, -1.0..=1.0, 0.0);
                        ch |= slider(ui, "Shadows", &mut self.adj.shadows, -1.0..=1.0, 0.0);
                    });
                    egui::CollapsingHeader::new("Colour").default_open(true).show(ui, |ui| {
                        ch |= slider(ui, "Temperature", &mut self.adj.temperature, -1.0..=1.0, 0.0);
                        ch |= slider(ui, "Tint", &mut self.adj.tint, -1.0..=1.0, 0.0);
                        ch |= slider(ui, "Saturation", &mut self.adj.saturation, -1.0..=1.0, 0.0);
                    });
                    ui.add_space(6.0);
                    if ui.button("Reset adjustments").clicked() {
                        act.reset = true;
                    }
                });
                if ch {
                    self.dirty = true;
                }
            });
        }

        // ---- help window
        if self.show_help {
            let mut open = true;
            let rows: Vec<(String, &str)> = vec![
                (ctx.format_shortcut(&sc_open), "Open image"),
                (ctx.format_shortcut(&sc_export), "Export full resolution"),
                (ctx.format_shortcut(&sc_rr), "Rotate right"),
                (ctx.format_shortcut(&sc_rl), "Rotate left"),
                (ctx.format_shortcut(&sc_reset), "Reset adjustments"),
                ("Left / Right".to_string(), "Previous / next film"),
                ("Space (hold)".to_string(), "Show original"),
                ("Double-click slider".to_string(), "Reset that slider"),
            ];
            egui::Window::new("Keyboard shortcuts").open(&mut open).resizable(false).show(ctx, |ui| {
                egui::Grid::new("shortcuts").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
                    for (k, d) in &rows {
                        ui.label(egui::RichText::new(k).strong());
                        ui.label(*d);
                        ui.end_row();
                    }
                });
            });
            self.show_help = open;
        }

        self.apply(ctx, act);
        if self.dirty {
            self.render(ctx);
            self.dirty = false;
        }

        // ---- canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(14, 14, 16)))
            .show(ctx, |ui| {
                let avail = ui.available_size();
                let (rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                let p = ui.painter_at(rect);
                let hold_space = !ctx.wants_keyboard_input() && ctx.input(|i| i.key_down(Key::Space));
                let before = self.show_original || hold_space;
                match &self.src {
                    Some(src) => {
                        let tex = if before { &self.tex_before } else { &self.tex_after };
                        if let Some(t) = tex {
                            let pad = 24.0;
                            let aw = (avail.x - pad * 2.0).max(10.0);
                            let ah = (avail.y - pad * 2.0).max(10.0);
                            let s = (aw / src.w as f32).min(ah / src.h as f32);
                            let img_rect = Rect::from_center_size(rect.center(), vec2(src.w as f32 * s, src.h as f32 * s));
                            p.rect_filled(img_rect.expand(1.0).translate(vec2(0.0, 4.0)), 2.0, Color32::from_black_alpha(100));
                            p.image(
                                t.id(),
                                img_rect,
                                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                            if before {
                                let chip = Rect::from_min_size(img_rect.left_top() + vec2(10.0, 10.0), vec2(84.0, 22.0));
                                p.rect_filled(chip, 11.0, Color32::from_black_alpha(180));
                                p.text(chip.center(), egui::Align2::CENTER_CENTER, "ORIGINAL", FontId::proportional(11.0), Color32::WHITE);
                            }
                        }
                    }
                    None => {
                        let cmd_o = ctx.format_shortcut(&sc_open);
                        p.text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("Drop a RAW / JPEG / PNG / TIFF here\nor press {cmd_o}"),
                            FontId::proportional(20.0),
                            Color32::from_gray(130),
                        );
                    }
                }
            });
    }
}

fn main() -> eframe::Result<()> {
    // Load and decode the PNG icon once at startup.
    let icon_image = image::load_from_memory(include_bytes!("../assets/icon-1024.png"))
        .expect("failed to load icon")
        .to_rgba8();
    let (icon_w, icon_h) = icon_image.dimensions();
    let icon_data = eframe::egui::IconData {
        rgba: icon_image.into_raw(),
        width: icon_w,
        height: icon_h,
    };

    let mut vb = egui::ViewportBuilder::default()
        .with_inner_size([1500.0, 900.0])
        .with_min_inner_size([980.0, 600.0])
        .with_title("SimuReel")
        .with_drag_and_drop(true)
        .with_icon(icon_data); // <-- this overrides eframe's default "e"

    if cfg!(target_os = "macos") {
        vb = vb
            .with_fullsize_content_view(true)
            .with_title_shown(false)
            .with_titlebar_shown(false);
    }
    let options = eframe::NativeOptions { viewport: vb, ..Default::default() };
    eframe::run_native("SimuReel", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}