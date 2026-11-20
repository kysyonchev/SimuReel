//! Native menu bar. On macOS this is the real system menu bar at the top of the screen
//! (Film Lab | File | Edit | View | Film | Window | Help). On other platforms it is a no-op
//! and the app falls back to an in-window egui menu bar.

pub enum Cmd {
    Open,
    Export,
    RotateRight,
    RotateLeft,
    Reset,
    ToggleOriginal,
    ToggleBrowser,
    ToggleAdjust,
    Next,
    Prev,
    Random,
    Pick(usize),
    Shortcuts,
}

#[cfg(target_os = "macos")]
pub use mac::NativeMenu;
#[cfg(not(target_os = "macos"))]
pub use stub::NativeMenu;

#[cfg(not(target_os = "macos"))]
mod stub {
    use super::Cmd;
    use crate::films::Film;
    use eframe::egui;

    pub struct NativeMenu;

    impl NativeMenu {
        pub fn new(_ctx: &egui::Context, _films: &[Film]) -> Option<Self> {
            None
        }
        pub fn poll(&self) -> Vec<Cmd> {
            vec![]
        }
        pub fn sync(&mut self, _has: bool, _busy: bool, _orig: bool, _browser: bool, _adjust: bool) {}
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use super::Cmd;
    use crate::films::Film;
    use eframe::egui;
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use std::sync::mpsc::{channel, Receiver};

    pub struct NativeMenu {
        _menu: Menu,
        rx: Receiver<String>,
        export: MenuItem,
        rot_l: MenuItem,
        rot_r: MenuItem,
        v_orig: CheckMenuItem,
        v_browser: CheckMenuItem,
        v_adjust: CheckMenuItem,
        last: Option<[bool; 5]>,
    }

    fn cmd(key: Code) -> Option<Accelerator> {
        Some(Accelerator::new(Some(Modifiers::SUPER), key))
    }

    impl NativeMenu {
        pub fn new(ctx: &egui::Context, films: &[Film]) -> Option<Self> {
            let (tx, rx) = channel::<String>();
            let c = ctx.clone();
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                let _ = tx.send(e.id.0.clone());
                c.request_repaint();
            }));

            let menu = Menu::new();

            // ---- app menu ("Film Lab")
            let app = Submenu::new("Film Lab", true);
            app.append_items(&[
                &PredefinedMenuItem::about(None, None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ])
            .ok()?;

            // ---- File
            let file = Submenu::new("File", true);
            let open = MenuItem::with_id("open", "Open…", true, cmd(Code::KeyO));
            let export = MenuItem::with_id("export", "Export…", false, cmd(Code::KeyE));
            file.append_items(&[&open, &export, &PredefinedMenuItem::separator(), &PredefinedMenuItem::close_window(None)])
                .ok()?;

            // ---- Edit
            let edit = Submenu::new("Edit", true);
            let rot_r = MenuItem::with_id("rot_r", "Rotate Right", false, cmd(Code::KeyR));
            let rot_l = MenuItem::with_id(
                "rot_l",
                "Rotate Left",
                false,
                Some(Accelerator::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyR)),
            );
            let reset = MenuItem::with_id("reset", "Reset Adjustments", true, cmd(Code::Digit0));
            edit.append_items(&[&rot_r, &rot_l, &PredefinedMenuItem::separator(), &reset]).ok()?;

            // ---- View
            let view = Submenu::new("View", true);
            let v_orig = CheckMenuItem::with_id("v_orig", "Show Original (hold Space)", true, false, None);
            let v_browser = CheckMenuItem::with_id("v_browser", "Film Browser", true, true, cmd(Code::Digit1));
            let v_adjust = CheckMenuItem::with_id("v_adjust", "Adjustments Panel", true, true, cmd(Code::Digit2));
            view.append_items(&[
                &v_orig,
                &PredefinedMenuItem::separator(),
                &v_browser,
                &v_adjust,
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::fullscreen(None),
            ])
            .ok()?;

            // ---- Film
            let film = Submenu::new("Film", true);
            let next = MenuItem::with_id("next", "Next Film  (→)", true, None);
            let prev = MenuItem::with_id("prev", "Previous Film  (←)", true, None);
            let random = MenuItem::with_id("random", "Random Film", true, cmd(Code::KeyG));
            let by_brand = Submenu::new("By Brand", true);
            let mut brands: Vec<&str> = films.iter().map(|f| f.brand).collect();
            brands.sort();
            brands.dedup();
            for b in brands {
                let sub = Submenu::new(b, true);
                for (i, f) in films.iter().enumerate().filter(|(_, f)| f.brand == b) {
                    let item = MenuItem::with_id(format!("film:{i}"), format!("{}  ·  ISO {}", f.name, f.iso), true, None);
                    sub.append(&item).ok()?;
                }
                by_brand.append(&sub).ok()?;
            }
            film.append_items(&[&next, &prev, &random, &PredefinedMenuItem::separator(), &by_brand]).ok()?;

            // ---- Window
            let window = Submenu::new("Window", true);
            window
                .append_items(&[&PredefinedMenuItem::minimize(None), &PredefinedMenuItem::maximize(None)])
                .ok()?;

            // ---- Help
            let help = Submenu::new("Help", true);
            help.append(&MenuItem::with_id("shortcuts", "Keyboard Shortcuts", true, None)).ok()?;

            menu.append_items(&[&app, &file, &edit, &view, &film, &window, &help]).ok()?;
            menu.init_for_nsapp();
            window.set_as_windows_menu_for_nsapp();
            help.set_as_help_menu_for_nsapp();

            Some(NativeMenu { _menu: menu, rx, export, rot_l, rot_r, v_orig, v_browser, v_adjust, last: None })
        }

        pub fn poll(&self) -> Vec<Cmd> {
            let mut out = vec![];
            while let Ok(id) = self.rx.try_recv() {
                match id.as_str() {
                    "open" => out.push(Cmd::Open),
                    "export" => out.push(Cmd::Export),
                    "rot_r" => out.push(Cmd::RotateRight),
                    "rot_l" => out.push(Cmd::RotateLeft),
                    "reset" => out.push(Cmd::Reset),
                    "v_orig" => out.push(Cmd::ToggleOriginal),
                    "v_browser" => out.push(Cmd::ToggleBrowser),
                    "v_adjust" => out.push(Cmd::ToggleAdjust),
                    "next" => out.push(Cmd::Next),
                    "prev" => out.push(Cmd::Prev),
                    "random" => out.push(Cmd::Random),
                    "shortcuts" => out.push(Cmd::Shortcuts),
                    s if s.starts_with("film:") => {
                        if let Ok(i) = s[5..].parse::<usize>() {
                            out.push(Cmd::Pick(i));
                        }
                    }
                    _ => {}
                }
            }
            out
        }

        /// Keep enabled / checked states in step with the app (only touches the menu when something changed).
        pub fn sync(&mut self, has: bool, busy: bool, orig: bool, browser: bool, adjust: bool) {
            let st = [has, busy, orig, browser, adjust];
            if self.last == Some(st) {
                return;
            }
            self.last = Some(st);
            self.export.set_enabled(has && !busy);
            self.rot_l.set_enabled(has);
            self.rot_r.set_enabled(has);
            self.v_orig.set_checked(orig);
            self.v_browser.set_checked(browser);
            self.v_adjust.set_checked(adjust);
        }
    }
}
