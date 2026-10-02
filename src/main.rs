mod assets;
mod block;
mod gl;
mod image;
mod inventory;
mod item;
mod math;
mod mesher;
mod noise;
mod platform;
mod render;
mod ui;
mod world;
mod worldgen;

use math::{Mat4, Vec3};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "--dump-assets" {
        let dir = args.get(2).map(|s| s.as_str()).unwrap_or("asset_dump");
        assets::dump_all(dir);
        assets::contact_sheet(&format!("{dir}/sheet.png"));
        println!("assets written to {dir}");
        return;
    }
    let shot = args.iter().position(|a| a == "--shot").map(|i| args[i + 1].clone());
    let seed: u64 = args.iter().position(|a| a == "--seed").map(|i| args[i + 1].parse().unwrap()).unwrap_or(12345);
    let mut win = platform::Window::new("Minecraft", 854.0, 480.0);
    let mut r = render::Renderer::new();
    let mut world = world::World::new(seed);
    let (sx, sy, sz) = worldgen::find_spawn(&world.generator);
    let mut cam = Vec3 { x: sx as f32 + 0.5, y: sy as f32 + 12.0, z: sz as f32 + 0.5 };
    let mut yaw: f32 = 45.0;
    let mut pitch: f32 = 20.0;
    if let Some(i) = args.iter().position(|a| a == "--cam") {
        let v: Vec<f32> = args[i + 1].split(',').map(|s| s.parse().unwrap()).collect();
        cam = Vec3 { x: v[0], y: v[1], z: v[2] }; yaw = v[3]; pitch = v[4];
    }
    let rd = 8;
    let start = std::time::Instant::now();
    let mut last = start;
    let mut frames = 0;
    let mut ui = ui::Ui::new();
    while !win.should_close {
        win.poll();
        let now = std::time::Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;
        for e in win.events.clone() {
            if let platform::Event::MouseDown(0) = e { win.set_mouse_locked(true); }
            if let platform::Event::KeyDown(platform::key::ESCAPE, _) = e { win.set_mouse_locked(false); }
        }
        if win.is_mouse_locked() {
            yaw += win.mouse_dx * 0.15;
            pitch = (pitch + win.mouse_dy * 0.15).clamp(-90.0, 90.0);
        }
        let (sy_, cy_) = yaw.to_radians().sin_cos();
        let fwd = Vec3 { x: -sy_, y: 0.0, z: -cy_ };
        let right = Vec3 { x: cy_, y: 0.0, z: -sy_ };
        let sp = 20.0 * dt;
        use platform::key::*;
        if win.key(W) { cam += fwd * sp; }
        if win.key(S) { cam -= fwd * sp; }
        if win.key(D) { cam += right * sp; }
        if win.key(A) { cam -= right * sp; }
        if win.key(SPACE) { cam.y += sp; }
        if win.key(SHIFT) { cam.y -= sp; }
        let pcx = (cam.x.floor() as i32) >> 4;
        let pcz = (cam.z.floor() as i32) >> 4;
        for dz in -rd - 1..=rd + 1 { for dx in -rd - 1..=rd + 1 { world.request(pcx + dx, pcz + dz); } }
        world.receive_chunks(16);
        r.resize(win.width, win.height);
        r.update_chunks(&mut world, cam, rd);

        let aspect = win.width as f32 / win.height as f32;
        let proj = Mat4::perspective(70f32.to_radians(), aspect, 0.05, 512.0);
        let view = Mat4::rot_x(pitch.to_radians()) * Mat4::rot_y(yaw.to_radians());
        let mvp = proj * view;
        let fog = [0.75, 0.85, 1.0];
        unsafe {
            gl::glViewport(0, 0, win.width as i32, win.height as i32);
            gl::glClearColor(fog[0], fog[1], fog[2], 1.0);
            gl::glClear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::glEnable(gl::DEPTH_TEST);
            gl::glDepthFunc(gl::LEQUAL);
            gl::glEnable(gl::CULL_FACE);
            gl::glCullFace(gl::BACK);
            gl::glFrontFace(gl::CCW);
        }
        r.draw_sky(&mvp, [0.47, 0.65, 1.0], fog, -0.3, 0.0, None);
        let fd = (rd * 16) as f32;
        r.setup_world_shader(&mvp, fog, (fd * 0.6, fd), 0.0);
        r.draw_chunks(cam, &mvp, false);
        r.draw_clouds(cam, &mvp, start.elapsed().as_secs_f64() * 20.0, [1.0, 1.0, 1.0], fd);
        r.setup_world_shader(&mvp, fog, (fd * 0.6, fd), 0.0);
        unsafe { gl::glEnable(gl::BLEND); gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA); }
        r.draw_chunks(cam, &mvp, true);
        unsafe { gl::glDisable(gl::BLEND); }
        ui.begin(&r, (win.mouse_x, win.mouse_y));
        ui.text(&r, &format!("Minecraft (fps {:.0}) chunks {} pending {}", 1.0 / dt.max(1e-4), r.chunks_rendered, world.pending_count()), 2.0, 2.0, ui::WHITE);
        ui.sprite(&r, "hotbar", ui.w / 2.0 - 91.0, ui.h - 22.0);
        ui.block_icon(&r, block::GRASS, ui.w / 2.0 - 88.0, ui.h - 19.0);
        ui.end();
        if let Some(path) = &shot {
            if start.elapsed().as_secs_f32() > 6.0 {
                gl::screenshot(win.width, win.height).save_png(path).unwrap();
                break;
            }
        }
        win.swap();
        frames += 1;
    }
    let _ = frames;
}
