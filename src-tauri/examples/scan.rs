//! Test manuel des scanners de plateformes :
//! `cargo run --example scan` (depuis src-tauri).

fn main() {
    let games = ludo_lib::platforms::scan_all(None);
    println!("=== {} jeux détectés ===\n", games.len());
    for g in &games {
        let cover = if g.cover_url.is_some() { "🖼" } else { "—" };
        println!(
            "[{:<6}] {:<45} {:>7.1} Go  {}  cible={}",
            g.platform, g.title, g.size_gb, cover, g.launch_target
        );
    }
}
