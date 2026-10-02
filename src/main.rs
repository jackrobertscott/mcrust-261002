mod assets;
mod gl;
mod image;
mod platform;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "--dump-assets" {
        let dir = args.get(2).map(|s| s.as_str()).unwrap_or("asset_dump");
        assets::dump_all(dir);
        println!("assets written to {dir}");
        return;
    }
    let mut win = platform::Window::new("Minecraft", 854.0, 480.0);
    let start = std::time::Instant::now();
    let mut frames = 0;
    while !win.should_close {
        win.poll();
        for e in &win.events { println!("{:?}", e); }
        unsafe {
            gl::glViewport(0, 0, win.width as i32, win.height as i32);
            gl::glClearColor(0.5, 0.7, 1.0, 1.0);
            gl::glClear(gl::COLOR_BUFFER_BIT);
        }
        win.swap();
        frames += 1;
        if start.elapsed().as_secs_f32() > 3.0 { break; }
    }
    println!("frames {frames} size {}x{} scale {}", win.width, win.height, win.scale);
}
