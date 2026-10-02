//! Menu screens and container GUIs (vanilla layouts).

use crate::block;
use crate::game::{Difficulty, Game, Menu};
use crate::gl::{self, Vertex};
use crate::inventory::{self, COOK_TOTAL};
use crate::item::{self, ItemStack};
use crate::math::{v3, Mat4};
use crate::models::{self, ModelKind, Pose, Shade};
use crate::platform::{key, Event, Window};
use crate::render::Renderer;
use crate::ui::{Ui, WHITE};
use crate::world::BlockEntity;

const GRAY_TEXT: [u8; 4] = [0x40, 0x40, 0x40, 255];

/// Per-frame UI input.
pub struct Input {
    pub click: Option<u8>,
    pub shift: bool,
    pub events: Vec<Event>,
    pub mouse_down: bool,
}

impl Input {
    pub fn from_window(win: &Window) -> Input {
        let mut click = None;
        for e in &win.events {
            if let Event::MouseDown(b) = e {
                click = Some(*b);
            }
        }
        Input { click, shift: win.key(key::SHIFT), events: win.events.clone(), mouse_down: win.buttons[0] }
    }
    pub fn empty() -> Input {
        Input { click: None, shift: false, events: Vec::new(), mouse_down: false }
    }
    pub fn key_pressed(&self, k: u16) -> bool {
        self.events.iter().any(|e| matches!(e, Event::KeyDown(c, _) if *c == k))
    }
}

pub enum Action {
    None,
    Quit,
}

fn clicked(inp: &Input, hovered: bool) -> bool {
    let c = hovered && inp.click == Some(0);
    if c {
        crate::audio::UI_CLICK.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    c
}

/// Darkened background used behind in-game menus.
fn menu_overlay(ui: &mut Ui, r: &Renderer) {
    ui.gradient(r, 0.0, 0.0, ui.w, ui.h, [16, 16, 16, 192], [16, 16, 16, 208]);
}

// ---------------- Title screen ----------------

pub fn title(g: &mut Game, r: &Renderer, ui: &mut Ui, inp: &Input, time: f32) -> Action {
    let (w, h) = (ui.w, ui.h);
    // vanilla overlays on top of the panorama
    ui.gradient(r, 0.0, 0.0, w, h, [255, 255, 255, 128], [255, 255, 255, 0]);
    ui.gradient(r, 0.0, 0.0, w, h, [0, 0, 0, 0], [0, 0, 0, 128]);
    let (lw, _lh) = ui.sprite_size(r, "logo");
    let lx = (w / 2.0 - lw / 2.0).floor();
    ui.sprite(r, "logo", lx, 30.0);
    // "Java Edition"-style subtitle replaced by Rust Edition
    let sub = "RUST EDITION";
    let sw = ui.text_width(r, sub);
    ui.text(r, sub, (w / 2.0 - sw / 2.0).floor(), 30.0 + 44.0 - 5.0, [255, 255, 255, 255]);

    // splash
    let splash = g.splash.clone();
    let tw = ui.text_width(r, &splash);
    let pulse = 1.8 - ((time * 1000.0 % 1000.0) / 1000.0 * std::f32::consts::TAU).sin().abs() * 0.1;
    let scale = pulse * 100.0 / (tw + 32.0);
    let mark = ui.mark();
    ui.text(r, &splash, -tw / 2.0, -8.0, [255, 255, 0, 255]);
    let (cx, cy) = (w / 2.0 + 90.0, 70.0);
    let ang = (-20.0f32).to_radians();
    ui.transform_since(mark, |x, y| {
        let (x, y) = (x * scale, y * scale);
        let (s, c) = ang.sin_cos();
        (cx + x * c - y * s, cy + x * s + y * c)
    });

    let y0 = h / 4.0 + 48.0;
    let bx = w / 2.0 - 100.0;
    let mut act = Action::None;
    if clicked(inp, ui.button(r, "Singleplayer", bx, y0, 200.0, 20.0, true)) {
        g.world_list = crate::save::list_worlds();
        g.selected_world = if g.world_list.is_empty() { None } else { Some(0) };
        g.confirm_delete = false;
        if g.world_list.is_empty() {
            open_create(g);
        } else {
            g.menu = Menu::SelectWorld;
        }
    }
    ui.button(r, "Multiplayer", bx, y0 + 24.0, 200.0, 20.0, false);
    ui.button(r, "Minecraft Realms", bx, y0 + 48.0, 200.0, 20.0, false);
    if clicked(inp, ui.button(r, "Options...", bx, y0 + 84.0, 98.0, 20.0, true)) {
        g.menu_return = Menu::Title;
        g.menu = Menu::Options;
    }
    if clicked(inp, ui.button(r, "Quit Game", bx + 102.0, y0 + 84.0, 98.0, 20.0, true)) {
        act = Action::Quit;
    }
    ui.text(r, "Minecraft Rust Edition 1.0", 2.0, h - 10.0, WHITE);
    let c = "Fan-made, not affiliated with Mojang";
    let cw = ui.text_width(r, c);
    ui.text(r, c, w - cw - 2.0, h - 10.0, WHITE);
    act
}

fn open_create(g: &mut Game) {
    g.name_text = "New World".into();
    g.seed_text.clear();
    g.focused_field = 0;
    g.menu = Menu::CreateWorld;
}

// ---------------- Select world ----------------

#[repr(C)]
struct Tm {
    sec: i32,
    min: i32,
    hour: i32,
    mday: i32,
    mon: i32,
    year: i32,
    wday: i32,
    yday: i32,
    isdst: i32,
    gmtoff: i64,
    zone: *const u8,
}
unsafe extern "C" {
    fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
}

fn format_date(secs: u64) -> String {
    // local time via libc (part of the OS, not a crate)
    let t = secs as i64;
    let mut tm = Tm { sec: 0, min: 0, hour: 0, mday: 0, mon: 0, year: 0, wday: 0, yday: 0, isdst: 0, gmtoff: 0, zone: std::ptr::null() };
    if !unsafe { localtime_r(&t, &mut tm) }.is_null() {
        return format!("{:02}/{:02}/{:02} {:02}:{:02}", tm.mon + 1, tm.mday, tm.year % 100, tm.hour, tm.min);
    }
    // civil-from-days (UTC) fallback
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:02}/{:02}/{:02} {:02}:{:02}", m, d, y % 100, rem / 3600, (rem / 60) % 60)
}

pub fn select_world(g: &mut Game, r: &Renderer, ui: &mut Ui, inp: &Input) {
    ui.dirt_background(r, 64);
    let (w, h) = (ui.w, ui.h);
    // list area darker, like vanilla
    ui.rect(r, 0.0, 32.0, w, h - 96.0, [0, 0, 0, 160]);
    ui.gradient(r, 0.0, 32.0, w, 4.0, [0, 0, 0, 255], [0, 0, 0, 0]);
    ui.gradient(r, 0.0, h - 68.0, w, 4.0, [0, 0, 0, 0], [0, 0, 0, 255]);
    ui.text_centered(r, "Select World", w / 2.0, 16.0, WHITE);
    let lx = w / 2.0 - 110.0;
    let mut y = 36.0;
    let list: Vec<(String, String, String)> = g
        .world_list
        .iter()
        .map(|i| (i.name.clone(), format!("{} ({})", i.folder, format_date(i.last_played)), format!("Survival Mode, Day {}", i.time / 24000)))
        .collect();
    for (k, (name, l2, l3)) in list.iter().enumerate() {
        if y > h - 104.0 {
            break;
        }
        let sel = g.selected_world == Some(k);
        if sel {
            ui.rect(r, lx - 2.0, y - 2.0, 220.0, 36.0, [128, 128, 128, 255]);
            ui.rect(r, lx - 1.0, y - 1.0, 218.0, 34.0, [0, 0, 0, 255]);
        }
        // world icon placeholder: grass block
        ui.block_icon(r, crate::block::GRASS, lx + 4.0, y + 8.0);
        ui.text(r, name, lx + 28.0, y + 1.0, WHITE);
        ui.text(r, l2, lx + 28.0, y + 12.0, [128, 128, 128, 255]);
        ui.text(r, l3, lx + 28.0, y + 22.0, [128, 128, 128, 255]);
        if inp.click == Some(0) && ui.hit(lx - 2.0, y - 2.0, 220.0, 36.0) {
            if g.selected_world == Some(k) {
                // double-click style: second click plays
                let folder = g.world_list[k].folder.clone();
                g.open_saved_world(&folder);
                return;
            }
            g.selected_world = Some(k);
            g.confirm_delete = false;
        }
        y += 36.0;
    }
    let has = g.selected_world.is_some();
    let by = h - 52.0;
    if clicked(inp, ui.button(r, "Play Selected World", w / 2.0 - 154.0, by, 150.0, 20.0, has)) && has {
        let folder = g.world_list[g.selected_world.unwrap()].folder.clone();
        g.open_saved_world(&folder);
        return;
    }
    if clicked(inp, ui.button(r, "Create New World", w / 2.0 + 4.0, by, 150.0, 20.0, true)) {
        open_create(g);
        return;
    }
    let del_label = if g.confirm_delete { "Click again to delete" } else { "Delete" };
    if clicked(inp, ui.button(r, del_label, w / 2.0 - 154.0, by + 24.0, 150.0, 20.0, has)) && has {
        if g.confirm_delete {
            let i = g.selected_world.unwrap();
            crate::save::delete_world(&g.world_list[i].folder);
            g.world_list = crate::save::list_worlds();
            g.selected_world = if g.world_list.is_empty() { None } else { Some(0) };
            g.confirm_delete = false;
        } else {
            g.confirm_delete = true;
        }
    }
    if clicked(inp, ui.button(r, "Cancel", w / 2.0 + 4.0, by + 24.0, 150.0, 20.0, true)) || inp.key_pressed(key::ESCAPE) {
        g.menu = Menu::Title;
    }
}

// ---------------- Create world ----------------

fn text_field(ui: &mut Ui, r: &Renderer, text: &str, x: f32, y: f32, w: f32, focused: bool, blink: bool) {
    ui.rect(r, x - 1.0, y - 1.0, w + 2.0, 22.0, if focused { [255, 255, 255, 255] } else { [160, 160, 160, 255] });
    ui.rect(r, x, y, w, 20.0, [0, 0, 0, 255]);
    let mut s = text.to_string();
    if focused && blink {
        s.push('_');
    }
    ui.text(r, &s, x + 4.0, y + 6.0, [224, 224, 224, 255]);
}

pub fn create_world(g: &mut Game, r: &Renderer, ui: &mut Ui, inp: &Input, time: f32) {
    ui.dirt_background(r, 64);
    let (w, h) = (ui.w, ui.h);
    ui.text_centered(r, "Create New World", w / 2.0, 20.0, WHITE);
    ui.text(r, "World Name", w / 2.0 - 100.0, 47.0, [160, 160, 160, 255]);
    let blink = (time * 3.0) as i32 % 2 == 0;
    text_field(ui, r, &g.name_text, w / 2.0 - 100.0, 60.0, 200.0, g.focused_field == 0, blink);
    ui.text(r, "Seed for the World Generator", w / 2.0 - 100.0, 92.0, [160, 160, 160, 255]);
    text_field(ui, r, &g.seed_text, w / 2.0 - 100.0, 105.0, 200.0, g.focused_field == 1, blink);
    ui.text(r, "Leave blank for a random seed", w / 2.0 - 100.0, 130.0, [160, 160, 160, 255]);
    if inp.click == Some(0) {
        if ui.hit(w / 2.0 - 100.0, 60.0, 200.0, 20.0) {
            g.focused_field = 0;
        } else if ui.hit(w / 2.0 - 100.0, 105.0, 200.0, 20.0) {
            g.focused_field = 1;
        }
    }
    let diff = format!("Difficulty: {}", g.options.difficulty.name());
    if clicked(inp, ui.button(r, &diff, w / 2.0 - 75.0, 150.0, 150.0, 20.0, true)) {
        g.options.difficulty = g.options.difficulty.next();
        g.options.save();
    }
    ui.text_centered(r, "Game Mode: Survival", w / 2.0, 178.0, [160, 160, 160, 255]);
    // keyboard
    for e in &inp.events {
        match e {
            Event::Char(c) => {
                let f = if g.focused_field == 0 { &mut g.name_text } else { &mut g.seed_text };
                if f.len() < 32 && (*c as u32) >= 32 && (*c as u32) < 127 {
                    f.push(*c);
                }
            }
            Event::KeyDown(key::BACKSPACE, _) => {
                let f = if g.focused_field == 0 { &mut g.name_text } else { &mut g.seed_text };
                f.pop();
            }
            Event::KeyDown(key::TAB, false) => g.focused_field = 1 - g.focused_field,
            _ => {}
        }
    }
    if clicked(inp, ui.button(r, "Create New World", w / 2.0 - 155.0, h - 28.0, 150.0, 20.0, true)) || inp.key_pressed(key::RETURN) {
        g.start_new_world();
    }
    if clicked(inp, ui.button(r, "Cancel", w / 2.0 + 5.0, h - 28.0, 150.0, 20.0, true)) || inp.key_pressed(key::ESCAPE) {
        g.world_list = crate::save::list_worlds();
        g.menu = if g.world_list.is_empty() { Menu::Title } else { Menu::SelectWorld };
    }
}

// ---------------- Options ----------------

fn slider(ui: &mut Ui, r: &Renderer, inp: &Input, label: &str, x: f32, y: f32, w: f32, value: f32, id: u8, drag: &mut Option<u8>) -> Option<f32> {
    // background: disabled-style button
    let hw = (w / 2.0).floor();
    ui.sprite_part(r, "widgets_button_disabled", x, y, 0.0, 0.0, hw, 20.0);
    ui.sprite_part(r, "widgets_button_disabled", x + hw, y, 200.0 - (w - hw), 0.0, w - hw, 20.0);
    let kx = x + value * (w - 8.0);
    let hovered = ui.hit(x, y, w, 20.0);
    let knob = if hovered || *drag == Some(id) { "widgets_button_hover" } else { "widgets_button" };
    ui.sprite_part(r, knob, kx, y, 0.0, 0.0, 4.0, 20.0);
    ui.sprite_part(r, knob, kx + 4.0, y, 196.0, 0.0, 4.0, 20.0);
    let col = if hovered { [255, 255, 160, 255] } else { [224, 224, 224, 255] };
    ui.text_centered(r, label, x + w / 2.0, y + 6.0, col);
    if inp.click == Some(0) && hovered {
        *drag = Some(id);
    }
    if *drag == Some(id) {
        if !inp.mouse_down && inp.click.is_none() {
            *drag = None;
            return None;
        }
        let v = ((ui.mouse_x - x - 4.0) / (w - 8.0)).clamp(0.0, 1.0);
        return Some(v);
    }
    None
}

pub fn options(g: &mut Game, r: &Renderer, ui: &mut Ui, inp: &Input, in_world: bool) {
    if in_world {
        menu_overlay(ui, r);
    } else {
        ui.dirt_background(r, 64);
    }
    let (w, h) = (ui.w, ui.h);
    ui.text_centered(r, "Options", w / 2.0, 15.0, WHITE);
    let pos = |i: i32| -> (f32, f32) { (w / 2.0 - 155.0 + (i % 2) as f32 * 160.0, h / 6.0 - 12.0 + 24.0 * (i / 2) as f32) };
    let mut drag = g.dragging_slider;
    // FOV
    let (x, y) = pos(0);
    let fov_label = if g.options.fov.round() as i32 == 70 { "FOV: Normal".to_string() } else if g.options.fov >= 110.0 { "FOV: Quake Pro".into() } else { format!("FOV: {}", g.options.fov.round()) };
    if let Some(v) = slider(ui, r, inp, &fov_label, x, y, 150.0, (g.options.fov - 30.0) / 80.0, 0, &mut drag) {
        g.options.fov = (30.0 + v * 80.0).round();
    }
    let (x, y) = pos(1);
    let diff = format!("Difficulty: {}", g.options.difficulty.name());
    if clicked(inp, ui.button(r, &diff, x, y, 150.0, 20.0, true)) {
        g.options.difficulty = g.options.difficulty.next();
    }
    let (x, y) = pos(2);
    let rd = format!("Render Distance: {} chunks", g.options.render_distance);
    if let Some(v) = slider(ui, r, inp, &rd, x, y, 150.0, (g.options.render_distance - 2) as f32 / 14.0, 1, &mut drag) {
        g.options.render_distance = 2 + (v * 14.0).round() as i32;
    }
    let (x, y) = pos(3);
    let sens = format!("Sensitivity: {}%", (g.options.sensitivity * 200.0).round());
    if let Some(v) = slider(ui, r, inp, &sens, x, y, 150.0, g.options.sensitivity, 2, &mut drag) {
        g.options.sensitivity = v;
    }
    let (x, y) = pos(4);
    let bob = format!("View Bobbing: {}", if g.options.view_bobbing { "ON" } else { "OFF" });
    if clicked(inp, ui.button(r, &bob, x, y, 150.0, 20.0, true)) {
        g.options.view_bobbing = !g.options.view_bobbing;
    }
    let (x, y) = pos(5);
    let gs = format!("GUI Scale: {}", match g.options.gui_scale { 0 => "Auto".to_string(), 1 => "Small".into(), 2 => "Normal".into(), 3 => "Large".into(), n => n.to_string() });
    if clicked(inp, ui.button(r, &gs, x, y, 150.0, 20.0, true)) {
        g.options.gui_scale = (g.options.gui_scale + 1) % 4;
    }
    let (x, y) = pos(6);
    let cl = format!("Clouds: {}", if g.options.clouds { "Fancy" } else { "OFF" });
    if clicked(inp, ui.button(r, &cl, x, y, 150.0, 20.0, true)) {
        g.options.clouds = !g.options.clouds;
    }
    let (x, y) = pos(7);
    if clicked(inp, ui.button(r, "Controls...", x, y, 150.0, 20.0, true)) {
        g.menu = Menu::Controls;
    }
    g.dragging_slider = drag;
    if clicked(inp, ui.button(r, "Done", w / 2.0 - 100.0, h / 6.0 + 168.0, 200.0, 20.0, true)) || inp.key_pressed(key::ESCAPE) {
        g.options.save();
        g.menu = g.menu_return;
    }
}

pub fn controls(g: &mut Game, r: &Renderer, ui: &mut Ui, inp: &Input, in_world: bool) {
    if in_world {
        menu_overlay(ui, r);
    } else {
        ui.dirt_background(r, 64);
    }
    let (w, h) = (ui.w, ui.h);
    ui.text_centered(r, "Controls", w / 2.0, 15.0, WHITE);
    let rows = [
        ("Walk", "W A S D"),
        ("Jump / Swim up", "Space"),
        ("Sneak", "Left Shift"),
        ("Sprint", "Control / Option / double-tap W"),
        ("Attack / Destroy", "Left Click (hold)"),
        ("Use Item / Place Block", "Right Click"),
        ("Inventory", "E"),
        ("Drop Item", "Q"),
        ("Hotbar Slots", "1 - 9 / Scroll"),
        ("Hide HUD / Debug / Perspective", "F1 / F3 / F5"),
        ("Pause", "Esc"),
    ];
    let mut y = 40.0;
    for (a, b) in rows {
        ui.text(r, a, w / 2.0 - 150.0, y, WHITE);
        let bw = ui.text_width(r, b);
        ui.text(r, b, w / 2.0 + 150.0 - bw, y, [255, 255, 160, 255]);
        y += 12.0;
    }
    if clicked(inp, ui.button(r, "Done", w / 2.0 - 100.0, h - 29.0, 200.0, 20.0, true)) || inp.key_pressed(key::ESCAPE) {
        g.menu = Menu::Options;
    }
}

// ---------------- Pause / death / loading ----------------

pub fn pause(g: &mut Game, r: &Renderer, ui: &mut Ui, inp: &Input) {
    menu_overlay(ui, r);
    let (w, h) = (ui.w, ui.h);
    ui.text_centered(r, "Game Menu", w / 2.0, 40.0, WHITE);
    let x = w / 2.0 - 100.0;
    let y = h / 4.0 + 8.0;
    if clicked(inp, ui.button(r, "Back to Game", x, y, 200.0, 20.0, true)) {
        g.menu = Menu::None;
    }
    ui.button(r, "Advancements", x, y + 24.0, 98.0, 20.0, false);
    ui.button(r, "Statistics", x + 102.0, y + 24.0, 98.0, 20.0, false);
    if clicked(inp, ui.button(r, "Options...", x, y + 72.0, 98.0, 20.0, true)) {
        g.menu_return = Menu::Pause;
        g.menu = Menu::Options;
    }
    ui.button(r, "Open to LAN", x + 102.0, y + 72.0, 98.0, 20.0, false);
    if clicked(inp, ui.button(r, "Save and Quit to Title", x, y + 96.0, 200.0, 20.0, true)) {
        g.quit_to_title();
    }
    if inp.key_pressed(key::ESCAPE) {
        g.menu = Menu::None;
    }
}

pub fn death(g: &mut Game, r: &Renderer, ui: &mut Ui, inp: &Input) {
    let (w, h) = (ui.w, ui.h);
    ui.gradient(r, 0.0, 0.0, w, h, [80, 0, 0, 96], [128, 48, 48, 160]);
    let t = "You died!";
    let tw = ui.text_width(r, t) * 2.0;
    ui.text_scaled(r, t, (w / 2.0 - tw / 2.0).floor(), 60.0, WHITE, 2.0);
    ui.text_centered(r, "Score: §e0", w / 2.0, 100.0, WHITE);
    let enabled = g.player.death_time > 20;
    if clicked(inp, ui.button(r, "Respawn", w / 2.0 - 100.0, h / 4.0 + 72.0, 200.0, 20.0, enabled)) && enabled {
        g.respawn();
    }
    if clicked(inp, ui.button(r, "Title screen", w / 2.0 - 100.0, h / 4.0 + 96.0, 200.0, 20.0, enabled)) && enabled {
        g.quit_to_title();
    }
}

pub fn loading(g: &Game, r: &Renderer, ui: &mut Ui, progress: f32) {
    ui.dirt_background(r, 64);
    let (w, h) = (ui.w, ui.h);
    ui.text_centered(r, "Loading world", w / 2.0, h / 2.0 - 30.0, WHITE);
    ui.text_centered(r, "Building terrain", w / 2.0, h / 2.0 - 10.0, WHITE);
    let bw = 100.0;
    let x = w / 2.0 - bw / 2.0;
    let y = h / 2.0 + 10.0;
    ui.rect(r, x, y, bw, 2.0, [128, 128, 128, 255]);
    ui.rect(r, x, y, bw * progress.clamp(0.0, 1.0), 2.0, [128, 255, 128, 255]);
    let _ = g;
}

// ---------------- Containers ----------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SlotRef {
    Inv(usize),
    Craft2(usize),
    Craft3(usize),
    CraftResult,
    FurnIn,
    FurnFuel,
    FurnOut,
    Chest(usize),
}

fn container_slots(menu: Menu) -> Vec<(f32, f32, SlotRef)> {
    let mut v = Vec::new();
    for r in 0..3 {
        for c in 0..9 {
            v.push((8.0 + c as f32 * 18.0, 84.0 + r as f32 * 18.0, SlotRef::Inv(9 + r * 9 + c)));
        }
    }
    for c in 0..9 {
        v.push((8.0 + c as f32 * 18.0, 142.0, SlotRef::Inv(c)));
    }
    match menu {
        Menu::Inventory => {
            for i in 0..4 {
                v.push((98.0 + (i % 2) as f32 * 18.0, 18.0 + (i / 2) as f32 * 18.0, SlotRef::Craft2(i)));
            }
            v.push((154.0, 28.0, SlotRef::CraftResult));
        }
        Menu::Crafting => {
            for i in 0..9 {
                v.push((30.0 + (i % 3) as f32 * 18.0, 17.0 + (i / 3) as f32 * 18.0, SlotRef::Craft3(i)));
            }
            v.push((124.0, 35.0, SlotRef::CraftResult));
        }
        Menu::Furnace => {
            v.push((56.0, 17.0, SlotRef::FurnIn));
            v.push((56.0, 53.0, SlotRef::FurnFuel));
            v.push((116.0, 35.0, SlotRef::FurnOut));
        }
        Menu::Chest => {
            for i in 0..27 {
                v.push((8.0 + (i % 9) as f32 * 18.0, 18.0 + (i / 9) as f32 * 18.0, SlotRef::Chest(i)));
            }
        }
        _ => {}
    }
    v
}

fn craft_result(g: &Game) -> Option<ItemStack> {
    match g.menu {
        Menu::Inventory => inventory::craft(&g.player.craft2, 2, 2),
        Menu::Crafting => inventory::craft(&g.craft3, 3, 3),
        _ => None,
    }
}

fn get_slot(g: &Game, s: SlotRef) -> ItemStack {
    match s {
        SlotRef::Inv(i) => g.player.inv.slots[i],
        SlotRef::Craft2(i) => g.player.craft2[i],
        SlotRef::Craft3(i) => g.craft3[i],
        SlotRef::CraftResult => craft_result(g).unwrap_or(ItemStack::EMPTY),
        SlotRef::FurnIn | SlotRef::FurnFuel | SlotRef::FurnOut => match g.world.block_entities.get(&g.open_pos) {
            Some(BlockEntity::Furnace(f)) => match s {
                SlotRef::FurnIn => f.input,
                SlotRef::FurnFuel => f.fuel,
                _ => f.output,
            },
            _ => ItemStack::EMPTY,
        },
        SlotRef::Chest(i) => match g.world.block_entities.get(&g.open_pos) {
            Some(BlockEntity::Chest(v)) => v[i],
            _ => ItemStack::EMPTY,
        },
    }
}

fn set_slot(g: &mut Game, s: SlotRef, st: ItemStack) {
    let st = if st.count == 0 { ItemStack::EMPTY } else { st };
    match s {
        SlotRef::Inv(i) => g.player.inv.slots[i] = st,
        SlotRef::Craft2(i) => g.player.craft2[i] = st,
        SlotRef::Craft3(i) => g.craft3[i] = st,
        SlotRef::CraftResult => {}
        SlotRef::FurnIn | SlotRef::FurnFuel | SlotRef::FurnOut => {
            if let Some(BlockEntity::Furnace(f)) = g.world.block_entities.get_mut(&g.open_pos) {
                match s {
                    SlotRef::FurnIn => f.input = st,
                    SlotRef::FurnFuel => f.fuel = st,
                    _ => f.output = st,
                }
            }
        }
        SlotRef::Chest(i) => {
            if let Some(BlockEntity::Chest(v)) = g.world.block_entities.get_mut(&g.open_pos) {
                v[i] = st;
            }
        }
    }
}

fn accepts(s: SlotRef, st: &ItemStack) -> bool {
    match s {
        SlotRef::CraftResult | SlotRef::FurnOut => false,
        SlotRef::FurnFuel => item::fuel_time(st.id) > 0,
        _ => true,
    }
}

fn consume_craft_grid(g: &mut Game) {
    let grid: &mut [ItemStack] = if g.menu == Menu::Inventory { &mut g.player.craft2 } else { &mut g.craft3 };
    for s in grid.iter_mut() {
        if !s.is_empty() {
            s.count -= 1;
            if s.count == 0 {
                *s = ItemStack::EMPTY;
            }
        }
    }
}

/// Move a stack into a range of slots (shift-click). Returns leftover.
fn move_into(g: &mut Game, mut st: ItemStack, targets: &[SlotRef]) -> ItemStack {
    let max = item::max_stack(st.id);
    for &t in targets {
        let cur = get_slot(g, t);
        if !cur.is_empty() && cur.can_stack_with(&st) && cur.count < max {
            let n = (max - cur.count).min(st.count);
            set_slot(g, t, ItemStack { count: cur.count + n, ..cur });
            st.count -= n;
            if st.count == 0 {
                return ItemStack::EMPTY;
            }
        }
    }
    for &t in targets {
        if get_slot(g, t).is_empty() && accepts(t, &st) {
            set_slot(g, t, st);
            return ItemStack::EMPTY;
        }
    }
    st
}

/// Would the whole stack fit into these slots?
fn can_fit(g: &Game, st: ItemStack, targets: &[SlotRef]) -> bool {
    let max = item::max_stack(st.id) as u32;
    let mut need = st.count as u32;
    for &t in targets {
        let cur = get_slot(g, t);
        if cur.is_empty() {
            if accepts(t, &st) {
                need = need.saturating_sub(max);
            }
        } else if cur.can_stack_with(&st) {
            need = need.saturating_sub(max - (cur.count as u32).min(max));
        }
        if need == 0 {
            return true;
        }
    }
    need == 0
}

fn shift_click(g: &mut Game, s: SlotRef) {
    let st = get_slot(g, s);
    if st.is_empty() {
        return;
    }
    let hotbar: Vec<SlotRef> = (0..9).map(SlotRef::Inv).collect();
    let main: Vec<SlotRef> = (9..36).map(SlotRef::Inv).collect();
    let all_inv: Vec<SlotRef> = main.iter().chain(hotbar.iter()).copied().collect();
    let all_inv_rev: Vec<SlotRef> = hotbar.iter().rev().chain(main.iter().rev()).copied().collect();
    match s {
        SlotRef::CraftResult => {
            // craft as many as possible
            for _ in 0..64 {
                let Some(res) = craft_result(g) else { break };
                if !can_fit(g, res, &all_inv_rev) {
                    break;
                }
                move_into(g, res, &all_inv_rev);
                consume_craft_grid(g);
            }
        }
        SlotRef::Inv(i) => {
            let targets: Vec<SlotRef> = match g.menu {
                Menu::Furnace => {
                    if item::smelt_result(st.id).is_some() {
                        vec![SlotRef::FurnIn]
                    } else if item::fuel_time(st.id) > 0 {
                        vec![SlotRef::FurnFuel]
                    } else if i < 9 {
                        main.clone()
                    } else {
                        hotbar.clone()
                    }
                }
                Menu::Chest => (0..27).map(SlotRef::Chest).collect(),
                _ => {
                    if i < 9 {
                        main.clone()
                    } else {
                        hotbar.clone()
                    }
                }
            };
            set_slot(g, s, ItemStack::EMPTY);
            let left = move_into(g, st, &targets);
            set_slot(g, s, left);
        }
        _ => {
            set_slot(g, s, ItemStack::EMPTY);
            let left = move_into(g, st, &all_inv);
            set_slot(g, s, left);
        }
    }
}

fn click_slot(g: &mut Game, s: SlotRef, button: u8, shift: bool) {
    if shift && button == 0 {
        shift_click(g, s);
        return;
    }
    let cur = get_slot(g, s);
    let cursor = g.player.cursor;
    if s == SlotRef::CraftResult {
        if cur.is_empty() {
            return;
        }
        if cursor.is_empty() {
            g.player.cursor = cur;
            consume_craft_grid(g);
        } else if cursor.can_stack_with(&cur) && cursor.count as u32 + cur.count as u32 <= item::max_stack(cur.id) as u32 {
            g.player.cursor.count += cur.count;
            consume_craft_grid(g);
        }
        return;
    }
    if s == SlotRef::FurnOut {
        if cur.is_empty() {
            return;
        }
        if cursor.is_empty() {
            g.player.cursor = cur;
            set_slot(g, s, ItemStack::EMPTY);
        } else if cursor.can_stack_with(&cur) {
            let max = item::max_stack(cur.id);
            let n = (max - cursor.count).min(cur.count);
            g.player.cursor.count += n;
            set_slot(g, s, ItemStack { count: cur.count - n, ..cur });
        }
        return;
    }
    if button == 0 {
        if cursor.is_empty() {
            g.player.cursor = cur;
            set_slot(g, s, ItemStack::EMPTY);
        } else if cur.is_empty() {
            if accepts(s, &cursor) {
                set_slot(g, s, cursor);
                g.player.cursor = ItemStack::EMPTY;
            }
        } else if cur.can_stack_with(&cursor) {
            let max = item::max_stack(cur.id);
            let n = (max - cur.count).min(cursor.count);
            set_slot(g, s, ItemStack { count: cur.count + n, ..cur });
            g.player.cursor.count -= n;
            if g.player.cursor.count == 0 {
                g.player.cursor = ItemStack::EMPTY;
            }
        } else if accepts(s, &cursor) {
            set_slot(g, s, cursor);
            g.player.cursor = cur;
        }
    } else if button == 1 {
        if cursor.is_empty() {
            if !cur.is_empty() {
                let take = cur.count.div_ceil(2);
                g.player.cursor = ItemStack { count: take, ..cur };
                set_slot(g, s, ItemStack { count: cur.count - take, ..cur });
            }
        } else if cur.is_empty() {
            if accepts(s, &cursor) {
                set_slot(g, s, ItemStack { count: 1, ..cursor });
                g.player.cursor.count -= 1;
            }
        } else if cur.can_stack_with(&cursor) && cur.count < item::max_stack(cur.id) {
            set_slot(g, s, ItemStack { count: cur.count + 1, ..cur });
            g.player.cursor.count -= 1;
        } else if accepts(s, &cursor) {
            set_slot(g, s, cursor);
            g.player.cursor = cur;
        }
        if g.player.cursor.count == 0 {
            g.player.cursor = ItemStack::EMPTY;
        }
    }
}

pub fn container(g: &mut Game, r: &mut Renderer, ui: &mut Ui, inp: &Input) {
    let (w, h) = (ui.w, ui.h);
    menu_overlay(ui, r);
    let left = ((w - 176.0) / 2.0).floor();
    let top = ((h - 166.0) / 2.0).floor();
    let bg = match g.menu {
        Menu::Inventory => "inventory",
        Menu::Crafting => "crafting_table",
        Menu::Furnace => "furnace",
        Menu::Chest => "chest",
        _ => "inventory",
    };
    ui.sprite(r, bg, left, top);
    match g.menu {
        Menu::Inventory => {
            ui.text_no_shadow(r, "Crafting", left + 97.0, top + 8.0, GRAY_TEXT);
        }
        Menu::Crafting => {
            ui.text_no_shadow(r, "Crafting", left + 28.0, top + 6.0, GRAY_TEXT);
            ui.text_no_shadow(r, "Inventory", left + 8.0, top + 72.0, GRAY_TEXT);
        }
        Menu::Furnace => {
            let t = "Furnace";
            let tw = ui.text_width(r, t);
            ui.text_no_shadow(r, t, left + 88.0 - tw / 2.0, top + 6.0, GRAY_TEXT);
            ui.text_no_shadow(r, "Inventory", left + 8.0, top + 72.0, GRAY_TEXT);
            if let Some(BlockEntity::Furnace(f)) = g.world.block_entities.get(&g.open_pos) {
                if f.burn_time > 0 && f.burn_total > 0 {
                    let fl = ((f.burn_time as f32 / f.burn_total as f32) * 13.0).ceil() + 1.0;
                    let fl = fl.min(14.0);
                    ui.sprite_part(r, "furnace_flame", left + 56.0, top + 36.0 + 14.0 - fl, 0.0, 14.0 - fl, 14.0, fl);
                }
                let ap = (f.cook_time as f32 / COOK_TOTAL as f32 * 24.0).floor();
                if ap > 0.0 {
                    ui.sprite_part(r, "furnace_arrow", left + 79.0, top + 34.0, 0.0, 0.0, ap + 1.0, 17.0);
                }
            }
        }
        Menu::Chest => {
            ui.text_no_shadow(r, "Chest", left + 8.0, top + 6.0, GRAY_TEXT);
            ui.text_no_shadow(r, "Inventory", left + 8.0, top + 72.0, GRAY_TEXT);
        }
        _ => {}
    }
    // player preview
    if g.menu == Menu::Inventory {
        ui.flush();
        draw_player_preview(g, r, ui, left + 51.0, top + 75.0, 30.0);
        ui.rebind(r);
    }
    // slots
    let slots = container_slots(g.menu);
    let mut hovered: Option<SlotRef> = None;
    for &(sx, sy, s) in &slots {
        let st = get_slot(g, s);
        let (x, y) = (left + sx, top + sy);
        ui.item(r, &st, x, y);
        if ui.hit(x - 1.0, y - 1.0, 18.0, 18.0) {
            hovered = Some(s);
            ui.flush();
            ui.rect(r, x, y, 16.0, 16.0, [255, 255, 255, 128]);
        }
    }
    // clicks
    if let Some(b) = inp.click {
        if let Some(s) = hovered {
            click_slot(g, s, b, inp.shift);
        } else if !ui.hit(left, top, 176.0, 166.0) && !g.player.cursor.is_empty() {
            // drop outside the window
            let mut st = g.player.cursor;
            if b == 1 {
                st.count = 1;
                g.player.cursor.count -= 1;
                if g.player.cursor.count == 0 {
                    g.player.cursor = ItemStack::EMPTY;
                }
            } else {
                g.player.cursor = ItemStack::EMPTY;
            }
            g.throw_item(st);
        }
    }
    // number keys swap hovered slot with hotbar
    if let Some(s) = hovered {
        let slots_keys = [key::N1, key::N2, key::N3, key::N4, key::N5, key::N6, key::N7, key::N8, key::N9];
        for (i, k) in slots_keys.iter().enumerate() {
            if inp.key_pressed(*k) && s != SlotRef::CraftResult && s != SlotRef::FurnOut {
                let a = get_slot(g, s);
                let b = g.player.inv.slots[i];
                if accepts(s, &b) || b.is_empty() {
                    set_slot(g, s, b);
                    g.player.inv.slots[i] = a;
                }
            }
        }
    }
    // cursor stack & tooltip
    if !g.player.cursor.is_empty() {
        let c = g.player.cursor;
        ui.item(r, &c, ui.mouse_x - 8.0, ui.mouse_y - 8.0);
    } else if let Some(s) = hovered {
        let st = get_slot(g, s);
        if !st.is_empty() {
            let name = item::name(st.id).to_string();
            ui.tooltip(r, &name, ui.mouse_x, ui.mouse_y);
        }
    }
    if inp.key_pressed(key::ESCAPE) || inp.key_pressed(key::E) {
        g.close_container();
    }
    let _ = block::AIR;
}

/// Render the player model into the inventory preview box, looking at the mouse.
fn draw_player_preview(g: &Game, r: &mut Renderer, ui: &Ui, x: f32, y: f32, scale: f32) {
    let dx = x - ui.mouse_x;
    let dy = (y - 50.0) - ui.mouse_y;
    let body_yaw = (dx / 40.0).atan() * 20.0;
    let head_yaw = (dx / 40.0).atan() * 40.0;
    let pitch = -(dy / 40.0).atan() * 20.0;
    let mut parts = models::build(ModelKind::Biped);
    let pose = Pose { head_yaw: -((head_yaw - body_yaw).to_radians()), head_pitch: -pitch.to_radians(), ..Default::default() };
    models::animate(ModelKind::Biped, &mut parts, &pose);
    // GUI ortho projection (y down) with depth
    let proj = Mat4::ortho(0.0, ui.w, ui.h, 0.0, -200.0, 200.0);
    let root = Mat4::translate(x, y, 50.0) * Mat4::scale(scale, -scale, scale) * Mat4::rot_y((180.0 + body_yaw).to_radians()) * Mat4::scale(-1.0, -1.0, 1.0) * Mat4::translate(0.0, -1.501, 0.0);
    let mut verts: Vec<Vertex> = Vec::new();
    models::emit(&mut verts, &parts, &root, (64.0, 64.0), Shade { sky: 255, block: 255, color: [255; 4] });
    let s = r.world_shader;
    s.bind();
    s.set_mat4("u_mvp", &proj.0);
    s.set_f("u_fixed_light", 1.0);
    s.set_v3("u_offset", [0.0; 3]);
    s.set_v4("u_flash", [0.0; 4]);
    s.set_v4("u_color_mul", [1.0; 4]);
    s.set_f("u_alpha_ref", 0.1);
    unsafe {
        gl::glUniform2f(s.loc("u_fog"), 1.0e5, 1.0e6);
        gl::glClear(gl::DEPTH_BUFFER_BIT);
        gl::glEnable(gl::DEPTH_TEST);
        gl::glDisable(gl::CULL_FACE);
    }
    if let Some(t) = r.skins.get("steve") {
        t.bind();
    }
    r.draw_stream(&verts);
    unsafe {
        gl::glDisable(gl::DEPTH_TEST);
    }
    s.set_f("u_fixed_light", -1.0);
    let _ = (g, v3(0.0, 0.0, 0.0), Difficulty::Normal);
}
