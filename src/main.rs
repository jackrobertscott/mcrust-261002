//! Minecraft Rust Edition - a from-scratch, zero-dependency recreation of
//! classic Minecraft. Everything (windowing, OpenGL bindings, art, world
//! generation) is implemented in this crate using only the Rust standard
//! library and macOS system frameworks.

mod assets;
mod block;
mod draw;
mod entity;
mod game;
mod gl;
mod image;
mod inventory;
mod item;
mod math;
mod mesher;
mod models;
mod noise;
mod physics;
mod platform;
mod render;
mod save;
mod screens;
mod ui;
mod world;
mod worldgen;

use game::{Game, Menu};
use math::v3;
use platform::Window;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "--dump-assets" {
        let dir = args.get(2).map(|s| s.as_str()).unwrap_or("asset_dump");
        assets::dump_all(dir);
        assets::contact_sheet(&format!("{dir}/sheet.png"));
        println!("assets written to {dir}");
        return;
    }
    if args.len() >= 4 && args[1] == "--map" {
        worldgen::debug_map(args[2].parse().unwrap_or(1), &args[3], 512, 8);
        return;
    }
    // Debug/automation: --shot <file> saves a screenshot after N seconds; --autoplay starts a world
    let shot = args.iter().position(|a| a == "--shot").and_then(|i| args.get(i + 1).cloned());
    let shot_delay: f32 = args.iter().position(|a| a == "--shot-delay").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(8.0);
    let autoplay = args.iter().position(|a| a == "--autoplay").map(|i| args.get(i + 1).cloned().unwrap_or_default());
    let script = args.iter().position(|a| a == "--script").and_then(|i| args.get(i + 1).cloned());

    let mut win = Window::new("Minecraft", 854.0, 480.0);
    let mut r = render::Renderer::new();
    let mut scene = draw::Scene::new();
    let mut ui = ui::Ui::new();
    let mut game = Game::new();
    if let Some(seed) = &autoplay {
        game.seed_text = seed.clone();
        game.start_new_world();
    }
    let start = std::time::Instant::now();
    let mut last = start;
    let mut acc = 0.0f32;
    let mut fps = 60.0f32;
    let mut fps_frames = 0;
    let mut fps_time = 0.0f32;
    let mut was_menu = game.menu;

    while !win.should_close {
        win.poll();
        let now = std::time::Instant::now();
        let dt = (now - last).as_secs_f32().min(0.25);
        last = now;
        fps_frames += 1;
        fps_time += dt;
        if fps_time >= 0.5 {
            fps = fps_frames as f32 / fps_time;
            fps_frames = 0;
            fps_time = 0.0;
        }
        let elapsed = start.elapsed().as_secs_f32();
        if let Some(s) = &script {
            run_script(&mut game, &mut win, s, elapsed);
        }

        // GUI scale
        r.resize(win.width, win.height);
        if game.options.gui_scale > 0 {
            r.gui_scale = game.options.gui_scale.min(r.gui_scale.max(1));
        }

        // Mouse capture
        let playing = game.menu == Menu::None && game.in_world;
        if playing && !win.focused && shot.is_none() {
            game.menu = Menu::Pause;
        }
        let playing = game.menu == Menu::None && game.in_world;
        win.set_mouse_locked(playing && win.focused && shot.is_none());
        if game.menu != was_menu {
            // drain stale mouse button state between screens
            if was_menu == Menu::None || game.menu == Menu::None {
                win.buttons = [false; 3];
            }
            was_menu = game.menu;
        }

        if playing && (win.is_mouse_locked() || shot.is_some()) && !game.player.dead {
            let f = game.options.sensitivity * 0.6 + 0.2;
            let f3 = f * f * f * 8.0;
            game.player.yaw += win.mouse_dx * f3 * 0.15;
            game.player.pitch = (game.player.pitch + win.mouse_dy * f3 * 0.15).clamp(-90.0, 90.0);
            game.handle_game_input(&mut win);
        }

        // Fixed-rate simulation (20 TPS)
        let paused = matches!(game.menu, Menu::Pause | Menu::Options | Menu::Controls | Menu::Loading);
        acc += dt;
        let mut steps = 0;
        while acc >= 0.05 && steps < 10 {
            acc -= 0.05;
            steps += 1;
            if !paused {
                game.tick(&win);
            }
        }
        let alpha = if paused { 1.0 } else { acc / 0.05 };

        // World streaming & meshing
        let center = game.player.body.pos;
        let unloaded = game.stream_chunks(center);
        for (x, z) in unloaded {
            r.drop_chunk_meshes(x, z);
        }
        let cam_pos = center + v3(0.0, 1.6, 0.0);
        r.update_chunks(&mut game.world, cam_pos, game.options.render_distance);
        if !game.in_world {
            game.panorama_angle += dt * 2.0;
        }

        // Loading screen -> spawn when the area around the player is ready
        let mut load_progress = 0.0;
        if game.menu == Menu::Loading {
            let (px, _, pz) = game.player.body.pos.floor();
            let (pcx, pcz) = (px >> 4, pz >> 4);
            let mut ready = 0;
            let mut total = 0;
            for dz in -2..=2 {
                for dx in -2..=2 {
                    total += 1;
                    if let Some(c) = game.world.chunk(pcx + dx, pcz + dz) {
                        if !c.dirty.iter().any(|d| *d) || (dx.abs() == 2 || dz.abs() == 2) {
                            ready += 1;
                        }
                    }
                }
            }
            load_progress = ready as f32 / total as f32;
            game.loading_ticks += 1;
            if ready == total && game.loading_ticks > 10 {
                // settle the player on the surface (new worlds) or just out of any blocks (loaded)
                let (x, py, z) = game.player.body.pos.floor();
                let mut y = if game.settle_on_load { game.world.surface_height(x, z).max(1) } else { py.max(1) };
                while y < world::CHUNK_H as i32 - 2 && (block::is_solid(game.world.get(x, y, z)) || block::is_solid(game.world.get(x, y + 1, z))) {
                    y += 1;
                }
                game.player.body.pos.y = y as f32;
                game.player.prev_pos = game.player.body.pos;
                if game.settle_on_load {
                    game.player.spawn = game.player.body.pos;
                }
                game.menu = Menu::None;
                game.save();
            }
        }

        // ---------------- render ----------------
        let input = screens::Input::from_window(&win);
        let world_visible = !matches!(game.menu, Menu::Loading | Menu::CreateWorld | Menu::SelectWorld) && !(!game.in_world && matches!(game.menu, Menu::Options | Menu::Controls));
        if world_visible {
            scene.draw_world(&game, &mut r, alpha, !game.in_world);
        } else {
            unsafe {
                gl::glViewport(0, 0, win.width as i32, win.height as i32);
                gl::glClearColor(0.0, 0.0, 0.0, 1.0);
                gl::glClear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            }
        }
        ui.begin(&r, (win.mouse_x, win.mouse_y));
        if game.in_world && !matches!(game.menu, Menu::Loading) {
            scene.draw_hud(&game, &mut r, &mut ui, fps);
        }
        let mut quit = false;
        match game.menu {
            Menu::Title => {
                if let screens::Action::Quit = screens::title(&mut game, &r, &mut ui, &input, elapsed) {
                    quit = true;
                }
            }
            Menu::SelectWorld => screens::select_world(&mut game, &r, &mut ui, &input),
            Menu::CreateWorld => screens::create_world(&mut game, &r, &mut ui, &input, elapsed),
            Menu::Options => {
                let iw = game.in_world;
                screens::options(&mut game, &r, &mut ui, &input, iw)
            }
            Menu::Controls => {
                let iw = game.in_world;
                screens::controls(&mut game, &r, &mut ui, &input, iw)
            }
            Menu::Pause => screens::pause(&mut game, &r, &mut ui, &input),
            Menu::Death => screens::death(&mut game, &r, &mut ui, &input),
            Menu::Loading => screens::loading(&game, &r, &mut ui, load_progress),
            Menu::Inventory | Menu::Crafting | Menu::Furnace | Menu::Chest => screens::container(&mut game, &mut r, &mut ui, &input),
            Menu::None => {}
        }
        ui.end();
        if quit {
            break;
        }
        if let Some(path) = &shot {
            if elapsed > shot_delay {
                let _ = gl::screenshot(win.width, win.height).save_png(path);
                break;
            }
        }
        win.swap();
        game.sounds.clear();
    }
    game.options.save();
    game.save();
}

/// Tiny scripted-input driver for automated testing: "t:action;t:action".
fn run_script(g: &mut Game, win: &mut Window, script: &str, t: f32) {
    for cmd in script.split(';') {
        let mut it = cmd.splitn(2, ':');
        let (Some(ts), Some(a)) = (it.next(), it.next()) else { continue };
        let Ok(at) = ts.parse::<f32>() else { continue };
        let key = format!("{cmd}");
        if t < at || g.splash.contains(&key) {
            continue;
        }
        // mark as done by appending to an internal string (debug only)
        g.splash.push_str(&key);
        let mut parts = a.split(',');
        match parts.next() {
            Some("menu") => {
                g.menu = match parts.next() {
                    Some("inventory") => Menu::Inventory,
                    Some("crafting") => Menu::Crafting,
                    Some("furnace") => Menu::Furnace,
                    Some("pause") => Menu::Pause,
                    Some("options") => Menu::Options,
                    Some("create") => Menu::CreateWorld,
                    Some("death") => Menu::Death,
                    Some("chest") => Menu::Chest,
                    _ => Menu::None,
                }
            }
            Some("give") => {
                let id: u16 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                let n: u8 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                g.player.inv.add(item::ItemStack::new(id, n));
            }
            Some("look") => {
                g.player.yaw = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                g.player.pitch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            }
            Some("time") => g.world.time = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1000),
            Some("f5") => g.third_person = (g.third_person + 1) % 3,
            Some("f3") => g.show_debug = !g.show_debug,
            Some("mob") => {
                let k = match parts.next() {
                    Some("pig") => entity::MobKind::Pig,
                    Some("cow") => entity::MobKind::Cow,
                    Some("sheep") => entity::MobKind::Sheep,
                    Some("chicken") => entity::MobKind::Chicken,
                    Some("zombie") => entity::MobKind::Zombie,
                    Some("skeleton") => entity::MobKind::Skeleton,
                    Some("creeper") => entity::MobKind::Creeper,
                    _ => entity::MobKind::Spider,
                };
                let d = g.player.look_dir();
                let dist: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(4.0);
                let p = g.player.body.pos + v3(d.x * dist, 0.5, d.z * dist);
                let mut m = entity::Mob::new(k, p, &mut g.rng);
                m.yaw = g.player.yaw + 180.0;
                m.persistent = true;
                g.mobs.push(m);
            }
            Some("lmb") => win.buttons[0] = parts.next() == Some("1"),
            Some("rmb") => win.buttons[1] = parts.next() == Some("1"),
            Some("key") => {
                let k: u16 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                win.keys[k as usize] = parts.next() == Some("1");
            }
            Some("tp") => {
                let v: Vec<f32> = parts.filter_map(|s| s.parse().ok()).collect();
                if v.len() == 3 {
                    g.player.body.pos = v3(v[0], v[1], v[2]);
                    g.player.prev_pos = g.player.body.pos;
                }
            }
            Some("craft3") => {
                let i: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let id: u16 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                g.craft3[i % 9] = item::ItemStack::new(id, 1);
            }
            Some("furnace") => {
                let (x, y, z) = g.player.body.pos.floor();
                g.open_pos = (x, y - 1, z);
                let mut f = inventory::FurnaceState::new();
                f.input = item::ItemStack::new(block::IRON_ORE as u16, 5);
                f.fuel = item::ItemStack::new(item::COAL, 3);
                g.world.block_entities.insert(g.open_pos, world::BlockEntity::Furnace(f));
            }
            Some("chest") => {
                let (x, y, z) = g.player.body.pos.floor();
                g.open_pos = (x, y - 1, z);
                let mut v = vec![item::ItemStack::EMPTY; 27];
                v[0] = item::ItemStack::new(item::DIAMOND, 5);
                v[13] = item::ItemStack::new(item::APPLE, 2);
                g.world.block_entities.insert(g.open_pos, world::BlockEntity::Chest(v));
            }
            Some("load") => {
                let f = parts.next().unwrap_or("").to_string();
                g.open_saved_world(&f);
            }
            Some("select") => {
                g.world_list = save::list_worlds();
                g.selected_world = Some(0);
                g.menu = Menu::SelectWorld;
            }
            Some("slot") => g.player.inv.selected = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            Some("hurt") => g.damage_player(parts.next().and_then(|s| s.parse().ok()).unwrap_or(1.0), None),
            _ => {}
        }
    }
}
