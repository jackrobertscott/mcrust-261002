mod assets;
mod image;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "--dump-assets" {
        let dir = args.get(2).map(|s| s.as_str()).unwrap_or("asset_dump");
        assets::dump_all(dir);
        println!("assets written to {dir}");
        return;
    }
    println!("game not implemented yet");
}
